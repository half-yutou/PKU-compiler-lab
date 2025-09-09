use koopa::ir::{BasicBlock, Function, FunctionData, Program, Type, TypeKind, Value};
use koopa::ir::builder::{BasicBlockBuilder, GlobalInstBuilder, LocalInstBuilder, ValueBuilder};
use crate::ast::{CompUnit, CompUnitItem, FuncDef, FuncType, GlobalDecl};
use crate::lab9::irgen::symbol::{ScopeStack, SymbolInfo};
use std::collections::HashMap;

/// 控制流上下文，用于跟踪嵌套的if-else结构
#[derive(Debug, Clone)]
pub struct ControlFlowContext {
    pub end_bb: BasicBlock,  // 当前控制流的结束基本块
    pub context_type: ControlFlowType,
}

/// 循环上下文，用于break/continue跳转
#[derive(Debug, Clone)]
pub struct LoopContext {
    pub loop_header: BasicBlock,  // continue跳转目标
    pub loop_end: BasicBlock,     // break跳转目标
}

#[derive(Debug, Clone)]
pub enum ControlFlowType {
    IfElse,  // if-else语句
    While {  // while循环
        loop_header: BasicBlock,  // 循环头（条件检查）
        loop_end: BasicBlock,     // 循环结束
    },
}

pub mod symbol;
pub mod declare;
pub mod block;
pub mod calc;
pub mod statement;
pub mod vars;

/// 程序级IR生成器，负责整个程序的IR生成
pub struct IRGen {
    program: Program,
    functions: HashMap<String, Function>, // 函数名到函数句柄的映射
    function_irgen: FunctionIRGen,        // 复用的函数IR生成器
}

/// 函数级IR生成器，负责单个函数的IR生成
pub struct FunctionIRGen {
    current_function: Option<Function>,   // 当前函数句柄
    pub scope_stack: ScopeStack,
    pub current_bb: Option<BasicBlock>,
    pub bb_counter: u32,        // 函数内基本块计数器
    pub control_flow_stack: Vec<ControlFlowContext>,// if 控制流
    pub pending_jumps: HashMap<BasicBlock, BasicBlock>,// 延迟跳转记录
    pub loop_stack: Vec<LoopContext>,// 循环控制流
}

impl IRGen {
    pub fn new() -> Self {
        Self {
            program: Program::new(),
            functions: HashMap::new(),
            function_irgen: FunctionIRGen::new(),
        }
    }
    
    
    pub fn generate_koopa_ir(mut self, ast: CompUnit) -> Result<Program, String> {
        // 首先添加 SysY 库函数声明
        self.declare_sysy_library_functions();
        
        // 然后处理全局声明和收集函数定义
        let mut func_defs = Vec::new();
        for item in ast.items {
            match item {
                CompUnitItem::FuncDef(func_def) => {
                    func_defs.push(func_def);
                }
                CompUnitItem::GlobalDecl(global_decl) => {
                    self.generate_global_decl(&global_decl)?;
                }
            }
        }
        
        // 检查是否存在main函数
        let has_main = func_defs.iter().any(|f| f.id == "main");
        if !has_main {
            return Err("Program must have a main function".to_string());
        }
        
        // 为每个函数创建函数声明(dfg层次的声明而不是layout层次的声明)
        for func_def in &func_defs {
            let func_name = format!("@{}", func_def.id);
            let return_type = match func_def.func_type {
                FuncType::Int => Type::get_i32(),
                FuncType::Void => Type::get_unit(),
            };
            
            // 处理参数类型
            let param_types = match &func_def.params {
                Some(params) => {
                    params.params.iter().map(|param| {
                        (Some(format!("@{}", param.ident)), Type::get_i32())
                    }).collect()
                }
                None => vec![],
            };
            
            let function = self.program.new_func(FunctionData::with_param_names(
                func_name.clone(),
                param_types,
                return_type,
            ));
            
            self.functions.insert(func_def.id.clone(), function);
        }
        
        // 生成每个函数的IR
        for func_def in &func_defs {
            self.generate_function_ir(func_def)?;
        }
        
        Ok(self.program)
    }
    
    /// 声明 SysY 库函数
    fn declare_sysy_library_functions(&mut self) {
        let library_functions = [
            ("getint", vec![], Type::get_i32()),
            ("getch", vec![], Type::get_i32()),
            ("getarray", vec![(None, Type::get_pointer(Type::get_i32()))], Type::get_i32()),
            ("putint", vec![(None, Type::get_i32())], Type::get_unit()),
            ("putch", vec![(None, Type::get_i32())], Type::get_unit()),
            ("putarray", vec![(None, Type::get_i32()), (None, Type::get_pointer(Type::get_i32()))], Type::get_unit()),
            ("starttime", vec![], Type::get_unit()),
            ("stoptime", vec![], Type::get_unit()),
        ];

        for (name, params, return_type) in library_functions {
            let func_name = format!("@{}", name);
            let function = self.program.new_func(FunctionData::with_param_names(
                func_name,
                params,
                return_type,
            ));

            // 将库函数也添加到函数映射中，以便在函数调用时能找到
            self.functions.insert(name.to_string(), function);
        }
    }
    
    /// 处理全局声明
    fn generate_global_decl(&mut self, global_decl: &GlobalDecl) -> Result<(), String> {
        match global_decl {
            GlobalDecl::Const(const_decl) => {
                for def in &const_decl.const_def_list {
                    let value = self.evaluate_lor_exp(&def.const_init_val.const_exp.lor_exp);
                    // 全局常量不需要考虑和局部常量重名,被作用域shadow了
                    if let Err(err) = self.function_irgen.scope_stack.define(def.ident.clone(), SymbolInfo::Const(value)) {
                        return Err(err);
                    }
                }
            }
            GlobalDecl::Var(var_decl) => {
                for def in &var_decl.var_def_list {
                    let global_name = format!("@{}", def.ident);
                    
                    // 创建初始化值
                    let init_ir = match &def.init_val { 
                        Some(init_val) => {
                            // 如果有初始化值，计算表达式的值
                            let init_val = self.evaluate_exp(&init_val.exp);
                            self.program.new_value().integer(init_val)
                        }
                        None => {
                            // 如果没有初始化值，使用零初始化(注意zero_init 不完全等价 0)
                            self.program.new_value().zero_init(Type::get_i32())
                        }
                    };
                    
                    // 创建全局变量
                    let global_alloc = self.program.new_value().global_alloc(init_ir);
                    self.program.set_value_name(global_alloc, Some(global_name));
                    
                    // 将全局变量添加到符号表
                    // 全局变量不需要考虑和局部变量重名,被作用域shadow了
                    if let Err(err) = self.function_irgen.scope_stack.define(def.ident.clone(), SymbolInfo::GlobalVar(global_alloc)) {
                        return Err(err);
                    }
                }
            }
        }
        Ok(())
    }
    
    /// 生成单个函数的IR
    fn generate_function_ir(&mut self, func_def: &FuncDef) -> Result<(), String> {
        let function = *self.functions.get(&func_def.id).unwrap();
        
        // 切换到新函数
        self.function_irgen.switch_to_function(function);
        
        // 生成函数体
        self.generate_function_body(func_def)?;
        
        // 完成函数处理
        self.function_irgen.finish_function();
        
        Ok(())
    }
    
    /// 生成函数体
    fn generate_function_body(&mut self, func_def: &FuncDef) -> Result<(), String> {
        // 进入函数作用域
        self.function_irgen.scope_stack.enter_scope();
        
        // 创建entry基本块(函数入口)
        let entry = {
            let func_data = self.function_data_mut();
            let bb = func_data.dfg_mut().new_bb().basic_block(Some("%entry".to_string()));
            func_data.layout_mut().bbs_mut().extend([bb]);
            bb
        };
        self.function_irgen.current_bb = Some(entry);
        
        // 处理函数参数，将其加入符号表
        if let Some(params) = &func_def.params {
            // 先获取参数值列表
            let param_values: Vec<Value> = {
                let func_data = self.function_data_mut();
                func_data.params().iter().cloned().collect()
            };
            
            // 为每个参数处理
            for (i, param) in params.params.iter().enumerate() {
                if let Some(param_value) = param_values.get(i) {
                    // 生成唯一名称
                    let unique_name = self.function_irgen.scope_stack.generate_unique_name(&param.ident);
                    
                    // 然后在单独的作用域中处理IR生成
                    let alloc_inst = {
                        let func_data = self.function_data_mut();
                        
                        // 为参数分配栈空间
                        let alloc_inst = func_data.dfg_mut().new_value().alloc(Type::get_i32());
                        func_data.dfg_mut().set_value_name(alloc_inst, Some(unique_name));
                        func_data.layout_mut().bb_mut(entry).insts_mut().push_key_back(alloc_inst).unwrap();
                        
                        // 将参数值存储到分配的空间
                        let store_inst = func_data.dfg_mut().new_value().store(*param_value, alloc_inst);
                        func_data.layout_mut().bb_mut(entry).insts_mut().push_key_back(store_inst).unwrap();
                        
                        alloc_inst
                    };
                    
                    // 现在可以安全地操作scope_stack
                    if let Err(err) = self.function_irgen.scope_stack.define(param.ident.clone(), SymbolInfo::Var(alloc_inst)) {
                        panic!("{}", err);
                    }
                }
            }
        }
        
        // 生成函数体
        self.generate_block(&func_def.block);
        
        // 处理所有延迟跳转
        self.process_pending_jumps();
        
        // 确保函数有终结指令
        if let Some(bb) = self.function_irgen.current_bb {
            self.ensure_terminator(bb);
        }
        
        // 退出函数作用域
        self.function_irgen.scope_stack.exit_scope();
        
        Ok(())
    }


    /// 获取当前函数的可变引用
    fn function_data_mut(&mut self) -> &mut FunctionData {
        let function_handler = self.function_irgen.current_function();
        let func_data = self.program.func_mut(function_handler);
        func_data
    }

    fn current_bb(&self) -> BasicBlock {
        self.function_irgen.current_bb.unwrap()
    }
    
    /// 推入控制流上下文到栈中
    pub fn push_control_flow(&mut self, end_bb: BasicBlock, context_type: ControlFlowType) {
        self.function_irgen.control_flow_stack.push(ControlFlowContext {
            end_bb,
            context_type,
        });
    }
    
    /// 从栈中弹出控制流上下文
    pub fn pop_control_flow(&mut self) -> Option<ControlFlowContext> {
        self.function_irgen.control_flow_stack.pop()
    }
    
    /// 记录延迟跳转
    pub fn record_pending_jump(&mut self, current_end_bb: BasicBlock) {
        if self.function_irgen.control_flow_stack.len() > 1 {
            if let Some(outer_context) = self.function_irgen.control_flow_stack.get(self.function_irgen.control_flow_stack.len() - 2) {
                self.function_irgen.pending_jumps.insert(current_end_bb, outer_context.end_bb);
            }
        }
    }
    
    /// 处理所有延迟跳转
    pub fn process_pending_jumps(&mut self) {
        let pending_jumps = std::mem::take(&mut self.function_irgen.pending_jumps);
        
        for (from_bb, to_bb) in pending_jumps {
            let func_data = self.function_data_mut();
            let bb_data = func_data.layout().bbs().node(&from_bb).unwrap();
            let has_terminator = bb_data.insts().back_key()
                .map(|inst| {
                    let value_data = func_data.dfg().value(*inst);
                    matches!(value_data.kind(), 
                        koopa::ir::ValueKind::Return(_) | 
                        koopa::ir::ValueKind::Jump(_) | 
                        koopa::ir::ValueKind::Branch(_))
                })
                .unwrap_or(false);
            
            if !has_terminator {
                let jump_inst = func_data.dfg_mut().new_value().jump(to_bb);
                func_data.layout_mut().bb_mut(from_bb).insts_mut().push_key_back(jump_inst).unwrap();
            }
        }
    }
    
    /// 确保指定基本块有终结指令
    pub fn ensure_terminator(&mut self, bb: BasicBlock) {
        let func_data = self.function_data_mut();
        let bb_data = func_data.layout().bbs().node(&bb).unwrap();
        let has_terminator = bb_data.insts().back_key()
            .map(|inst| {
                let value_data = func_data.dfg().value(*inst);
                matches!(value_data.kind(), 
                    koopa::ir::ValueKind::Return(_) | 
                    koopa::ir::ValueKind::Jump(_) | 
                    koopa::ir::ValueKind::Branch(_))
            })
            .unwrap_or(false);

        if !has_terminator {
            // 根据函数返回类型添加默认返回值
            let func_type = func_data.ty();
            let return_type = match func_type.kind() {
                TypeKind::Function(_, ret) => ret,
                _ => panic!("Function should have function type"),
            };
            
            let ret_inst = if return_type.is_unit() {
                // void函数返回空
                func_data.dfg_mut().new_value().ret(None)
            } else {
                // int函数返回0
                let zero = func_data.dfg_mut().new_value().integer(0);
                func_data.dfg_mut().new_value().ret(Some(zero))
            };
            
            func_data.layout_mut().bb_mut(bb).insts_mut().push_key_back(ret_inst).unwrap();
        }
    }
    
}

impl FunctionIRGen {
    pub fn new() -> Self {
        Self {
            current_function: None,
            scope_stack: ScopeStack::new(), // 包含全局符号表
            current_bb: None,
            bb_counter: 0,
            control_flow_stack: Vec::new(),
            pending_jumps: HashMap::new(),
            loop_stack: Vec::new(),
        }
    }
    
    /// 切换到新函数，重置函数级状态但保持全局符号表
    pub fn switch_to_function(&mut self, function: Function) {
        self.current_function = Some(function);
        self.current_bb = None;
        self.bb_counter = 0;
        self.control_flow_stack.clear();
        self.pending_jumps.clear();
        self.loop_stack.clear();
        // 注意：scope_stack 不重置，保持全局符号表
    }
    
    /// 完成函数处理，清理函数级状态
    pub fn finish_function(&mut self) {
        self.current_function = None;
        self.current_bb = None;
        self.bb_counter = 0;
        self.control_flow_stack.clear();
        self.pending_jumps.clear();
        self.loop_stack.clear();
        // scope_stack 保持不变，全局符号表继续存在
    }
    
    /// 获取当前函数句柄
    fn current_function(&self) -> Function {
        self.current_function.expect("No current function set")
    }
}