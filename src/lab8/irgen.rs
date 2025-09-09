use koopa::ir::{BasicBlock, Function, FunctionData, Program, Type, TypeKind, Value};
use koopa::ir::builder::{BasicBlockBuilder, LocalInstBuilder, ValueBuilder};
use crate::ast::{CompUnit, CompUnitItem, FuncDef, FuncType};
use crate::lab8::irgen::symbol::{ScopeStack, SymbolInfo};
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
}

/// 函数级IR生成器，负责单个函数的IR生成
pub struct FunctionIRGen<'a> {
    irgen: &'a mut IRGen,   // 引用到 IRGen
    function: Function,     // 当前函数句柄
    pub scope_stack: ScopeStack,
    pub current_bb: Option<BasicBlock>,
    pub bb_counter: u32,        // 函数内基本块计数器
    pub control_flow_stack: Vec<ControlFlowContext>,
    pub pending_jumps: HashMap<BasicBlock, BasicBlock>,
    pub loop_stack: Vec<LoopContext>,
}

impl IRGen {
    pub fn new() -> Self {
        Self {
            program: Program::new(),
            functions: HashMap::new(),
        }
    }
    
    /// 提供对 Program 的可变访问
    pub fn program_mut(&mut self) -> &mut Program {
        &mut self.program
    }
    
    /// 提供对 Program 的不可变访问
    pub fn program(&self) -> &Program {
        &self.program
    }
    
    pub fn generate_koopa_ir(mut self, ast: CompUnit) -> Result<Program, String> {
        // 首先收集所有函数定义
        let mut func_defs = Vec::new();
        for item in ast.items {
            match item {
                CompUnitItem::FuncDef(func_def) => {
                    func_defs.push(func_def);
                }
            }
        }
        
        // 检查是否存在main函数
        let has_main = func_defs.iter().any(|f| f.id == "main");
        if !has_main {
            return Err("Program must have a main function".to_string());
        }
        
        // 为每个函数创建函数声明
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
        for func_def in func_defs {
            self.generate_function_ir(&func_def)?;
        }
        
        Ok(self.program)
    }
    
    /// 生成单个函数的IR
    fn generate_function_ir(&mut self, func_def: &FuncDef) -> Result<(), String> {
        let function = *self.functions.get(&func_def.id).unwrap();
        
        // 创建函数级IR生成器
        let mut func_gen = FunctionIRGen::new(self, function);
        func_gen.generate_function_body(func_def)?;
        
        Ok(())
    }
}

impl<'a> FunctionIRGen<'a> {
    pub fn new(irgen: &'a mut IRGen, function: Function) -> Self {
        Self {
            irgen,
            function,
            scope_stack: ScopeStack::new(),
            current_bb: None,
            bb_counter: 0,
            control_flow_stack: Vec::new(),
            pending_jumps: HashMap::new(),
            loop_stack: Vec::new(),
        }
    }
    
    /// 获取程序的可变引用（安全方式）
    fn program_mut(&mut self) -> &mut Program {
        self.irgen.program_mut()
    }
    
    /// 获取当前函数的可变引用
    fn function_data_mut(&mut self) -> &mut FunctionData {
        let function_handler = self.function;
        let func_data = self.program_mut().func_mut(function_handler);
        func_data
    }
    
    fn current_bb(&self) -> BasicBlock {
        self.current_bb.unwrap()
    }
    
    /// 生成函数体
    pub fn generate_function_body(&mut self, func_def: &FuncDef) -> Result<(), String> {
        // 创建entry基本块
        let entry = {
            let func_data = self.function_data_mut();
            let bb = func_data.dfg_mut().new_bb().basic_block(Some("%entry".to_string()));
            func_data.layout_mut().bbs_mut().extend([bb]);
            bb
        };
        self.current_bb = Some(entry);
        
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
                    let unique_name = self.scope_stack.generate_unique_name(&param.ident);
                    
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
                    if let Err(err) = self.scope_stack.define(param.ident.clone(), SymbolInfo::Var(alloc_inst)) {
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
        if let Some(bb) = self.current_bb {
            self.ensure_terminator(bb);
        }
        
        Ok(())
    }
    
    /// 推入控制流上下文到栈中
    pub fn push_control_flow(&mut self, end_bb: BasicBlock, context_type: ControlFlowType) {
        self.control_flow_stack.push(ControlFlowContext {
            end_bb,
            context_type,
        });
    }
    
    /// 从栈中弹出控制流上下文
    pub fn pop_control_flow(&mut self) -> Option<ControlFlowContext> {
        self.control_flow_stack.pop()
    }
    
    /// 记录延迟跳转
    pub fn record_pending_jump(&mut self, current_end_bb: BasicBlock) {
        if self.control_flow_stack.len() > 1 {
            if let Some(outer_context) = self.control_flow_stack.get(self.control_flow_stack.len() - 2) {
                self.pending_jumps.insert(current_end_bb, outer_context.end_bb);
            }
        }
    }
    
    /// 处理所有延迟跳转
    pub fn process_pending_jumps(&mut self) {
        let pending_jumps = std::mem::take(&mut self.pending_jumps);
        
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