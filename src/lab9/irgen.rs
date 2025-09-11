use koopa::ir::{BasicBlock, Function, FunctionData, Program, Type, TypeKind, Value};
use koopa::ir::builder::{BasicBlockBuilder, GlobalInstBuilder, LocalInstBuilder, ValueBuilder};
use crate::ast::{CompUnit, CompUnitItem, FuncDef, FuncType, GlobalDecl, Exp, ConstInitVal, InitVal};
use crate::lab9::irgen::symbol::{ScopeStack, SymbolInfo};
use std::collections::HashMap;

pub mod symbol;
pub mod declare;
pub mod block;
pub mod calc;
pub mod statement;
pub mod vars;
pub mod array;
mod args;

/// 初始化器枚举，用于处理数组初始化
#[derive(Debug, Clone)]
pub enum Initializer {
    Const(i32),
    Value(Value),
    List(Vec<Initializer>),
}

impl Initializer {
    /// 从AST的ConstInitVal创建Initializer（用于全局/局部 常量）
    pub fn from_const_init_val(init_val: &ConstInitVal, irgen: &IRGen) -> Result<Self, String> {
        match init_val {
            // 单个常量表达式,直接计算并返回Self::Const(i32)
            ConstInitVal::Exp(const_exp) => {
                let value = irgen.evaluate_lor_exp(&const_exp.lor_exp);
                Ok(Self::Const(value))
            }
            // 数组初始化列表,递归处理每个元素,最后返回一个Self::Const列表
            // 例如const int arr[2][3][4] = {1, 2, 3, 4, {5}, {6}, {7, 8}};
            // 解析得到以下初始化器
            // Self::List([
            //     Self::Const(1),
            //     Self::Const(2), 
            //     Self::Const(3), 
            //     Self::Const(4),
            //     Self::List([Self::Const(5)]),
            //     Self::List([Self::Const(6)]),
            //     Self::List([Self::Const(7), Self::Const(8)])
            // ])
            ConstInitVal::List(list) => {
                let inits: Result<Vec<_>, _> = list
                    .iter()
                    .map(|v| Self::from_const_init_val(v, irgen))
                    .collect();
                Ok(Self::List(inits?))
            }
        }
    }

    /// 从AST的InitVal创建Initializer（用于全局变量）
    pub fn from_global_var_init_val(init_val: &InitVal, irgen: &IRGen) -> Result<Self, String> {
        match init_val {
            InitVal::Exp(exp) => {
                let value = match exp {
                    Exp::LOr(lor_exp) => irgen.evaluate_lor_exp(lor_exp)
                };
                Ok(Self::Const(value))
            }
            InitVal::List(list) => {
                let inits: Result<Vec<_>, _> = list
                    .iter()
                    .map(|v| Self::from_global_var_init_val(v, irgen))
                    .collect();
                Ok(Self::List(inits?))
            }
        }
    }
    
    /// 根据给定类型重塑初始化器
    pub fn reshape(self, ty: &Type) -> Result<Self, String> {
        // 获取维度列表
        let mut lens = Vec::new();
        
        // ty是当前变量的类型,若是数组类型,则其递归记录着每一层的数组长度
        // 例如int a[2][3][4]的类型为Array(Array(Array(Int32, 4), 3), 2)
        // 则lens = [2, 3, 4]
        let mut current_ty = ty;
        loop {
            match current_ty.kind() {
                TypeKind::Int32 => break,
                TypeKind::Array(base, len) => {
                    lens.push(*len);
                    current_ty = base;
                }
                _ => return Err("Unsupported type for array initialization".to_string()),
            }
        }
        
        // 计算累积长度 -> 什么是累积长度:每个累积长度表示 从当前维度到最内层维度的总元素数量
        // 输入的 lens = [2, 3, 4] （表示数组维度）
        // 1.
        //    反转后 : [4, 3, 2]
        // 2.
        //    逐步计算累积长度
        //    - 处理 4 : last_len = 1  * 4 = 4  → 结果 (4, 4)
        //    - 处理 3 : last_len = 4  * 3 = 12 → 结果 (3, 12)
        //    - 处理 2 : last_len = 12 * 2 = 24 → 结果 (2, 24)
        // 3.
        //    最终结果 : lens = [(4, 4), (3, 12), (2, 24)]
        let mut last_len = 1;
        let lens: Vec<_> = lens
            .into_iter()
            .rev()
            .map(|l| {
                last_len *= l;
                (l, last_len)
            })
            .collect();
        
        // 为什么要计算累计长度?
        // 这些累积长度后续用于执行重塑时：
        // 1. 确定分组大小 : 知道每一层应该包含多少个元素
        // 2. 进位计算 : 当某一层填满时，需要进位到上一层
        // 3. 零填充 : 计算还需要填充多少个零元素
        // 例如，对于初始化列表 {1, 2, 3, 4, {5}, {6}, {7, 8}} ：
        // 
        // - 最内层每组 4 个元素： {1,2,3,4} , {5,0,0,0} , {6,0,0,0} , {7,8,0,0} ...
        // - 中间层每组 12 个元素（3×4）：第一组{{1,2,3,4}, {5,0,0,0}, {6,0,0,0}}, 第二组{{7,8,0,0}, {0,0,0,0}, {0,0,0,0}}
        // - 最外层总共 24 个元素（2×3×4）：...
        
        // 执行重塑
        match self {
            // 标量不需要重塑
            Self::Const(val) if lens.is_empty() => Ok(Self::Const(val)),
            Self::Value(val) if lens.is_empty() => Ok(Self::Value(val)),
            // 数组需要重塑
            Self::List(l) if !lens.is_empty() => Self::reshape_impl(l, &lens),
            _ => Err("Invalid initialization".to_string()),
        }
    }
    
    fn reshape_impl(inits: Vec<Self>, lens: &[(usize, usize)]) -> Result<Self, String> {
        let mut reshaped: Vec<Vec<Self>> = (0..=lens.len()).map(|_| Vec::new()).collect();
        let mut len = 0;
        
        // 处理初始化器元素
        for init in inits {
            // 元素过多
            if len >= lens.last().unwrap().1 {
                return Err("Too many initializer elements".to_string());
            }
            match init {
                Self::List(list) => {
                    // 获取下一级长度列表
                    let next_lens = match reshaped.iter().position(|v| !v.is_empty()) {
                        Some(0) => return Err("Misaligned initialization".to_string()),
                        Some(i) => &lens[..i],
                        None => &lens[..lens.len() - 1],
                    };
                    // 重塑并添加到重塑初始化器列表
                    reshaped[next_lens.len()].push(Self::reshape_impl(list, next_lens)?);
                    Self::carry(&mut reshaped, lens);
                    len += next_lens.last().unwrap().1;
                }
                _ => {
                    // 直接推入
                    reshaped[0].push(init);
                    Self::carry(&mut reshaped, lens);
                    len += 1;
                }
            }
        }
        
        // 填充零
        while len < lens.last().unwrap().1 {
            reshaped[0].push(Self::Const(0));
            Self::carry(&mut reshaped, lens);
            len += 1;
        }
        
        Ok(reshaped.pop().unwrap().pop().unwrap())
    }
    
    fn carry(reshaped: &mut [Vec<Self>], lens: &[(usize, usize)]) {
        // 执行进位
        for (i, &(len, _)) in lens.iter().enumerate() {
            if reshaped[i].len() == len {
                let init = Self::List(reshaped[i].drain(..).collect());
                reshaped[i + 1].push(init);
            }
        }
    }
    
    /// 将初始化器转换为常量值（必须先重塑）
    pub fn into_const(self, program: &mut Program) -> Result<Value, String> {
        match self {
            Self::Const(num) => Ok(program.new_value().integer(num)),
            Self::Value(_) => Err("Cannot evaluate non-constant value".to_string()),
            Self::List(list) => {
                let values: Result<Vec<_>, _> = list
                    .into_iter()
                    .map(|i| i.into_const(program))
                    .collect();
                let values = values?;
                Ok(program.new_value().aggregate(values))
            }
        }
    }
}

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
        
        // 为每个函数创建函数声明(也就是函数头 -> void func_name(int a, int b[]) )
        for func_def in &func_defs {
            let func_name = format!("@{}", func_def.id);
            let return_type = match func_def.func_type {
                FuncType::Int => Type::get_i32(),
                FuncType::Void => Type::get_unit(),
            };
            
            // 处理参数类型
            let param_types = match &func_def.params {
                Some(params) => {
                    let mut types = Vec::new();
                    for param in &params.params {
                        let param_type = match param.dimensions.is_empty() {
                            // 标量参数
                            true => Type::get_i32(),
                            
                            // 数组参数：根据维度创建指针类型
                            false =>  {
                                // 对于 int arr[] -> *i32
                                // 对于 int arr[][10] -> *[i32, 10]
                                // 对于 int arr[][10][20] -> *[[i32, 20], 10]
                                let mut base_type = Type::get_i32();
                                
                                // 从第二维开始（第一维在函数参数中被忽略）
                                for dim_opt in param.dimensions.iter().skip(1).rev() {
                                    if let Some(dim_exp) = dim_opt {
                                        let dim_value = self.evaluate_lor_exp(&dim_exp.lor_exp);
                                        base_type = Type::get_array(base_type, dim_value as usize);
                                    } else {
                                        panic!("Array parameter dimension cannot be empty except for the first dimension");
                                    }
                                }
                                
                                // 创建指针类型
                                Type::get_pointer(base_type)
                            }
                        };
                        types.push((Some(format!("@{}", param.ident)), param_type));
                    }
                    types
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
        
        // 生成每个函数体的IR
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
                    match def.dimensions.is_empty() {
                        // 标量常量
                        true => {
                            let initializer = Initializer::from_const_init_val(&def.const_init_val, self)?;
                            let value = match initializer {
                                Initializer::Const(value) => value,
                                _ => return Err("Invalid constant initialization".to_string()),
                            };
                            
                            if let Err(err) = self.function_irgen.scope_stack.define(def.ident.clone(), SymbolInfo::Const(value)) {
                                return Err(err);
                            }
                        }
                        
                        // 数组常量
                        false => {
                            let global_name = format!("@{}", def.ident);
                            
                            // 构建数组类型
                            let mut ty = Type::get_i32();
                            for dim_exp in def.dimensions.iter().rev() {
                                let dim_size = self.evaluate_lor_exp(&dim_exp.lor_exp);
                                ty = Type::get_array(ty, dim_size as usize);
                            }
                            
                            // 创建初始化器并重塑
                            let initializer = Initializer::from_const_init_val(&def.const_init_val, self)?;
                            let reshaped = initializer.reshape(&ty)?;
                            let init_value = reshaped.into_const(&mut self.program)?;
                            
                            // 创建全局常量数组（和变量数组一样分配内存）
                            let global_var_ptr = self.program.new_value().global_alloc(init_value);
                            self.program.set_value_name(global_var_ptr, Some(global_name));
                            
                            // 计算数组维度
                            let mut dimensions = Vec::new();
                            for dim_exp in &def.dimensions {
                                let dim_size = self.evaluate_lor_exp(&dim_exp.lor_exp);
                                dimensions.push(dim_size as usize);
                            }
                            
                            // 存入符号表
                            if let Err(err) = self.function_irgen.scope_stack.define(
                                def.ident.clone(), 
                                SymbolInfo::GlobalConstArray(global_var_ptr, dimensions)
                            ) {
                                return Err(err);
                            }
                        }
                    }
                }
            }
            GlobalDecl::Var(var_decl) => {
                for def in &var_decl.var_def_list {
                    let global_name = format!("@{}", def.ident);
                    
                    // 构建数组类型(注释同上)
                    let ty = match def.dimensions.is_empty() {
                        // 标量
                        true => Type::get_i32(),
                        
                        // 数组
                        false => {
                            let mut ty = Type::get_i32();
                            for dim_exp in def.dimensions.iter().rev() {
                                let dim_size = self.evaluate_lor_exp(&dim_exp.lor_exp);
                                ty = Type::get_array(ty, dim_size as usize);
                            }
                            ty
                        }
                    };
                    
                    // 创建初始化值
                    let init_value = match &def.init_val {
                        Some(init_val) => {
                            let initializer = Initializer::from_global_var_init_val(init_val, self)?;
                            let reshaped = initializer.reshape(&ty)?;
                            reshaped.into_const(&mut self.program)?
                        }
                        None => {
                            // 没有初始化值，使用零初始化
                            self.program.new_value().zero_init(ty)
                        }
                    };
                    
                    // 创建全局变量
                    let global_var_ptr = self.program.new_value().global_alloc(init_value);
                    self.program.set_value_name(global_var_ptr, Some(global_name));
                    
                    // 将全局变量添加到符号表
                    // 全局变量不需要考虑和局部变量重名,被作用域shadow了
                    let symbol_info = match def.dimensions.is_empty() {
                        // 全局变量
                        true => SymbolInfo::GlobalVar(global_var_ptr),
                        
                        // 数组
                        false =>  {
                            // 计算数组维度
                            let mut dimensions = Vec::new();
                            for dim_exp in &def.dimensions {
                                let dim_size = self.evaluate_lor_exp(&dim_exp.lor_exp);
                                dimensions.push(dim_size as usize);
                            }
                            SymbolInfo::GlobalArray(global_var_ptr, dimensions)
                        }
                    };
                    
                    if let Err(err) = self.function_irgen.scope_stack.define(def.ident.clone(), symbol_info) {
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
            // 先获取内存IR中的参数值列表(函数头，即dfg存储着的各个参数的类型)
            let param_values: Vec<Value> = {
                let func_data = self.function_data_mut();
                func_data.params().iter().cloned().collect()
            };
            
            // 为每个参数处理(用ast向dfg中的参数填充参数值) => 把参数赋值给函数内部的一个局部变量处理，会使得后续处理更方便
            for (i, param) in params.params.iter().enumerate() {
                if let Some(param_value) = param_values.get(i) {
                    // 生成唯一名称
                    let unique_name = self.function_irgen.scope_stack.generate_unique_name(&param.ident);
                    
                    // 检查是否为数组参数
                    match param.dimensions.is_empty() {
                        // 标量参数
                        true => {                        
                            let param_ptr = {
                                let func_data = self.function_data_mut();

                                // 为参数分配栈空间
                                let param_ptr = func_data.dfg_mut().new_value().alloc(Type::get_i32());
                                func_data.dfg_mut().set_value_name(param_ptr, Some(unique_name));
                                func_data.layout_mut().bb_mut(entry).insts_mut().push_key_back(param_ptr).unwrap();

                                // 将参数值存储到分配的空间
                                let store_inst = func_data.dfg_mut().new_value().store(*param_value, param_ptr);
                                func_data.layout_mut().bb_mut(entry).insts_mut().push_key_back(store_inst).unwrap();

                                param_ptr
                            };

                            // 存储为标量变量
                            if let Err(err) = self.function_irgen.scope_stack.define(param.ident.clone(), SymbolInfo::Var(param_ptr)) {
                                panic!("{}", err);
                            }
                        }
                        
                        // 数组参数 - 数组参数在函数中实际上是指针
                        false => {
                            // 为数组参数(指针)分配栈空间存储指针 *i32 -> **i32
                            let param_ptr = {
                                let func_data = self.function_data_mut();
                                
                                // 使用参数句柄获取参数，再获取参数的类型
                                let param_type = func_data.dfg().value(*param_value).ty().clone();
                                println!("Debug: construct param_type: {:}", param_type);
                                
                                // 为局部变量分配栈空间
                                let param_ptr = func_data.dfg_mut().new_value().alloc(param_type);
                                func_data.dfg_mut().set_value_name(param_ptr, Some(unique_name));
                                func_data.layout_mut().bb_mut(entry).insts_mut().push_key_back(param_ptr).unwrap();
                                
                                // 将传递进来的参数值存储到分配的空间，完成实参到局部变量的赋值
                                let store_inst = func_data.dfg_mut().new_value().store(*param_value, param_ptr);
                                func_data.layout_mut().bb_mut(entry).insts_mut().push_key_back(store_inst).unwrap();
                                
                                param_ptr
                            };
                            
                            // 计算形参数组的维度信息
                            let mut dimensions = Vec::new();
                            for dim_opt in &param.dimensions {
                                match dim_opt {
                                    Some(dim_exp) => {
                                        let dim_value = self.evaluate_lor_exp(&dim_exp.lor_exp) as usize;
                                        dimensions.push(dim_value);
                                    }
                                    None => {
                                        // 第一维为None表示不定长，这里用0表示
                                        dimensions.push(0);
                                    }
                                }
                            }
                            
                            // 存储为函数数组参数类型
                            if let Err(err) = self.function_irgen.scope_stack.define(
                                param.ident.clone(), 
                                SymbolInfo::ParamArray(param_ptr, dimensions)// 注意这里的param_ptr是*param_value类型，相当于在上述处理中多了一层指针
                            ) {
                                panic!("{}", err);
                            }
                        }
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