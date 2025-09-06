// use crate::ast::{AddExp, Block, BlockItem, CompUnit, ConstExp, Decl, EqExp, EqOp, Exp, LAndExp, LOrExp, MulDivOp, MulExp, PlusSubOp, PrimaryExp, RelExp, RelOp, Stmt, UnaryExp, UnaryOp};
// use koopa::ir::builder::{BasicBlockBuilder, LocalInstBuilder, ValueBuilder};
// use koopa::ir::{BinaryOp, FunctionData, Program, Type, Value};
// use std::collections::HashMap;
// 
// // 符号信息：区分常量和变量
// #[derive(Debug, Clone)]
// enum SymbolInfo {
//     Const(i32),           // 常量：直接存储值
//     Var(Value),           // 变量：存储 alloc 返回的指针
// }
// 
// // 作用域栈：支持嵌套作用域的符号表
// #[derive(Debug)]
// struct ScopeStack {
//     scopes: Vec<HashMap<String, SymbolInfo>>,  // 作用域栈，每层是一个符号表
//     var_counter: HashMap<String, usize>,       // 变量重命名计数器
// }
// 
// impl ScopeStack {
//     fn new() -> Self {
//         Self {
//             scopes: vec![HashMap::new()], // 初始化全局作用域
//             var_counter: HashMap::new(),
//         }
//     }
//     
//     // 进入新作用域
//     fn enter_scope(&mut self) {
//         self.scopes.push(HashMap::new());
//     }
//     
//     // 退出当前作用域
//     fn exit_scope(&mut self) {
//         if self.scopes.len() > 1 {
//             self.scopes.pop();
//         }
//     }
//     
//     // 在当前作用域定义符号
//     fn define(&mut self, name: String, info: SymbolInfo) -> Result<(), String> {
//         if let Some(current_scope) = self.scopes.last_mut() {
//             if current_scope.contains_key(&name) {
//                 return Err(format!("Symbol '{}' already defined in current scope", name));
//             }
//             current_scope.insert(name, info);
//             Ok(())
//         } else {
//             Err("No active scope".to_string())
//         }
//     }
//     
//     // 跨作用域查找符号（从内层到外层）
//     fn lookup(&self, name: &str) -> Option<&SymbolInfo> {
//         for scope in self.scopes.iter().rev() {
//             if let Some(info) = scope.get(name) {
//                 return Some(info);
//             }
//         }
//         None
//     }
//     
//     // 生成唯一的变量名（用于KoopaIR）
//     fn generate_unique_name(&mut self, base_name: &str) -> String {
//         let counter = self.var_counter.entry(base_name.to_string()).or_insert(0);
//         *counter += 1;
//         format!("@{}_{}", base_name, counter)
//     }
// }
// 
// pub fn generate_koopa_ir(ast: CompUnit) -> Program {
//     let mut program = Program::new();
//     let main_func = program.new_func(FunctionData::with_param_names(
//         "@main".into(),
//         vec![],
//         Type::get_i32(),
//     ));
//     let main_data = program.func_mut(main_func);
// 
//     let entry = main_data.dfg_mut().new_bb().basic_block(Some("%entry".into()));
//     main_data.layout_mut().bbs_mut().extend([entry]);
// 
//     // 创建作用域栈
//     let mut scope_stack = ScopeStack::new();
// 
//     // 处理函数体
//     let has_return = generate_block(&ast.func_def.block, main_data, &mut scope_stack);
//     
//     // 如果函数没有显式的return语句，添加默认的return 0
//     if !has_return {
//         let zero = main_data.dfg_mut().new_value().integer(0);
//         let ret = main_data.dfg_mut().new_value().ret(Some(zero));
//         let entry = main_data.layout().entry_bb().unwrap();
//         main_data.layout_mut().bb_mut(entry).insts_mut().push_key_back(ret).unwrap();
//     }
// 
//     program
// }
// 
// // 处理代码块（支持作用域管理）
// fn generate_block(block: &Block, func_data: &mut FunctionData, scope_stack: &mut ScopeStack) -> bool {
//     // 进入新作用域
//     scope_stack.enter_scope();
//     
//     let mut has_return = false;
//     for item in &block.block_item_list {
//         if has_return {
//             // 如果已经有return语句，跳过后续语句
//             break;
//         }
//         
//         match item {
//             BlockItem::Decl(decl) => generate_decl(decl, func_data, scope_stack),
//             BlockItem::Stmt(stmt) => {
//                 has_return = generate_stmt(stmt, func_data, scope_stack);
//             }
//         }
//     }
//     
//     // 退出当前作用域
//     scope_stack.exit_scope();
//     has_return
// }
// 
// // 处理声明（常量定义和变量定义）
// fn generate_decl(decl: &Decl, func_data: &mut FunctionData, scope_stack: &mut ScopeStack) {
//     match decl {
//         Decl::Const(const_decl) => {
//             for def in &const_decl.const_def_list {
//                 // 编译时计算常量值
//                 let value = evaluate_const_exp(&def.const_init_val.const_exp, scope_stack);
//                 
//                 // 检查重定义并存入符号表
//                 if let Err(err) = scope_stack.define(def.ident.clone(), SymbolInfo::Const(value)) {
//                     panic!("{}", err);
//                 }
//             }
//         }
//         Decl::Var(var_decl) => {
//             for def in &var_decl.var_def_list {
//                 // 生成唯一的变量名
//                 let unique_name = scope_stack.generate_unique_name(&def.ident);
//                 
//                 // 为变量分配内存
//                 let alloc_inst = func_data.dfg_mut().new_value().alloc(Type::get_i32());
//                 func_data.dfg_mut().set_value_name(alloc_inst, Some(unique_name));
//                 
//                 let entry = func_data.layout().entry_bb().unwrap();
//                 func_data.layout_mut().bb_mut(entry).insts_mut().push_key_back(alloc_inst).unwrap();
//                 
//                 // 如果有初始化值，生成 store 指令
//                 if let Some(init_val) = &def.init_val {
//                     let init_value = generate_exp(&init_val.exp, func_data, scope_stack);
//                     let store_inst = func_data.dfg_mut().new_value().store(init_value, alloc_inst);
//                     func_data.layout_mut().bb_mut(entry).insts_mut().push_key_back(store_inst).unwrap();
//                 }
//                 
//                 // 检查重定义并存入符号表
//                 if let Err(err) = scope_stack.define(def.ident.clone(), SymbolInfo::Var(alloc_inst)) {
//                     panic!("{}", err);
//                 }
//             }
//         }
//     }
// }
// 
// // 处理语句
// fn generate_stmt(stmt: &Stmt, func_data: &mut FunctionData, scope_stack: &mut ScopeStack) -> bool {
//     match stmt {
//         Stmt::Return(exp_opt) => {
//             match exp_opt {
//                 Some(exp) => {
//                     // 有返回值的return语句
//                     let value = generate_exp(exp, func_data, scope_stack);
//                     let ret = func_data.dfg_mut().new_value().ret(Some(value));
//                     let entry = func_data.layout().entry_bb().unwrap();
//                     func_data.layout_mut().bb_mut(entry).insts_mut().push_key_back(ret).unwrap();
//                 }
//                 None => {
//                     // 无返回值的return语句
//                     let ret = func_data.dfg_mut().new_value().ret(None);
//                     let entry = func_data.layout().entry_bb().unwrap();
//                     func_data.layout_mut().bb_mut(entry).insts_mut().push_key_back(ret).unwrap();
//                 }
//             }
//             true // 返回true表示遇到了return语句
//         }
//         Stmt::Assign(lval, exp) => {
//             // 生成右侧表达式的值
//             let value = generate_exp(exp, func_data, scope_stack);
//             
//             // 获取左值对应的指针
//             match scope_stack.lookup(&lval.ident) {
//                 Some(SymbolInfo::Var(ptr)) => {
//                     // 生成 store 指令
//                     let store_inst = func_data.dfg_mut().new_value().store(value, *ptr);
//                     let entry = func_data.layout().entry_bb().unwrap();
//                     func_data.layout_mut().bb_mut(entry).insts_mut().push_key_back(store_inst).unwrap();
//                 }
//                 Some(SymbolInfo::Const(_)) => {
//                     panic!("Cannot assign to constant '{}'!", lval.ident);
//                 }
//                 None => {
//                     panic!("Variable '{}' not found!", lval.ident);
//                 }
//             }
//             false
//         }
//         Stmt::Exp(exp_opt) => {
//             match exp_opt {
//                 Some(exp) => {
//                     // 表达式语句，生成IR但丢弃结果
//                     generate_exp(exp, func_data, scope_stack);
//                 }
//                 None => {
//                     // 空语句，什么也不做
//                 }
//             }
//             false
//         }
//         Stmt::Block(block) => {
//             // 嵌套代码块，递归处理（会自动管理作用域）
//             generate_block(block, func_data, scope_stack)
//         }
//     }
// }
// 
// // region 编译时常量求值
// 
// fn evaluate_const_exp(const_exp: &ConstExp, scope_stack: &ScopeStack) -> i32 {
//     evaluate_lor_exp_for_const(&const_exp.lor_exp, scope_stack)
// }
// 
// fn evaluate_add_exp(add_exp: &AddExp, scope_stack: &ScopeStack) -> i32 {
//     match add_exp {
//         AddExp::Mul(mul_exp) => evaluate_mul_exp(mul_exp, scope_stack),
//         AddExp::AddMul(left, op, right) => {
//             let left_val = evaluate_add_exp(left, scope_stack);
//             let right_val = evaluate_mul_exp(right, scope_stack);
//             match op {
//                 PlusSubOp::Plus => left_val + right_val,
//                 PlusSubOp::Minus => left_val - right_val,
//             }
//         }
//     }
// }
// 
// fn evaluate_mul_exp(mul_exp: &MulExp, scope_stack: &ScopeStack) -> i32 {
//     match mul_exp {
//         MulExp::Unary(unary_exp) => evaluate_unary_exp(unary_exp, scope_stack),
//         MulExp::MulDiv(left, op, right) => {
//             let left_val = evaluate_mul_exp(left, scope_stack);
//             let right_val = evaluate_unary_exp(right, scope_stack);
//             match op {
//                 MulDivOp::Mul => left_val * right_val,
//                 MulDivOp::Div => left_val / right_val,
//                 MulDivOp::Mod => left_val % right_val,
//             }
//         }
//     }
// }
// 
// fn evaluate_unary_exp(unary_exp: &UnaryExp, scope_stack: &ScopeStack) -> i32 {
//     match unary_exp {
//         UnaryExp::Primary(primary) => evaluate_primary_exp(primary, scope_stack),
//         UnaryExp::Unary(op, exp) => {
//             let val = evaluate_unary_exp(exp, scope_stack);
//             match op {
//                 UnaryOp::Plus => val,
//                 UnaryOp::Minus => -val,
//                 UnaryOp::Not => if val == 0 { 1 } else { 0 },
//             }
//         }
//     }
// }
// 
// fn evaluate_primary_exp(primary: &PrimaryExp, scope_stack: &ScopeStack) -> i32 {
//     match primary {
//         PrimaryExp::Number(num) => *num,
//         PrimaryExp::Paren(exp) => evaluate_exp_for_const(exp, scope_stack),
//         PrimaryExp::LVal(lval) => {
//             match scope_stack.lookup(&lval.ident) {
//                 Some(SymbolInfo::Const(value)) => *value,
//                 Some(SymbolInfo::Var(_)) => panic!("Cannot use variable '{}' in constant expression", lval.ident),
//                 None => panic!("Identifier '{}' not found", lval.ident),
//             }
//         }
//     }
// }
// 
// fn evaluate_exp_for_const(exp: &Exp, scope_stack: &ScopeStack) -> i32 {
//     match exp {
//         Exp::LOr(lor_exp) => evaluate_lor_exp_for_const(lor_exp, scope_stack),
//     }
// }
// 
// fn evaluate_lor_exp_for_const(lor_exp: &LOrExp, scope_stack: &ScopeStack) -> i32 {
//     match lor_exp {
//         LOrExp::LAnd(land_exp) => evaluate_land_exp_for_const(land_exp, scope_stack),
//         LOrExp::LOr(left, right) => {
//             let left_val = evaluate_lor_exp_for_const(left, scope_stack);
//             if left_val != 0 {
//                 1
//             } else {
//                 if evaluate_land_exp_for_const(right, scope_stack) != 0 { 1 } else { 0 }
//             }
//         }
//     }
// }
// 
// fn evaluate_land_exp_for_const(land_exp: &LAndExp, scope_stack: &ScopeStack) -> i32 {
//     match land_exp {
//         LAndExp::Eq(eq_exp) => evaluate_eq_exp_for_const(eq_exp, scope_stack),
//         LAndExp::LAnd(left, right) => {
//             let left_val = evaluate_land_exp_for_const(left, scope_stack);
//             if left_val == 0 {
//                 0
//             } else {
//                 if evaluate_eq_exp_for_const(right, scope_stack) != 0 { 1 } else { 0 }
//             }
//         }
//     }
// }
// 
// fn evaluate_eq_exp_for_const(eq_exp: &EqExp, scope_stack: &ScopeStack) -> i32 {
//     match eq_exp {
//         EqExp::Rel(rel_exp) => evaluate_rel_exp_for_const(rel_exp, scope_stack),
//         EqExp::Eq(left, op, right) => {
//             let left_val = evaluate_eq_exp_for_const(left, scope_stack);
//             let right_val = evaluate_rel_exp_for_const(right, scope_stack);
//             match op {
//                 EqOp::Eq => (left_val == right_val) as i32,
//                 EqOp::Ne => (left_val != right_val) as i32,
//             }
//         }
//     }
// }
// 
// fn evaluate_rel_exp_for_const(rel_exp: &RelExp, scope_stack: &ScopeStack) -> i32 {
//     match rel_exp {
//         RelExp::Add(add_exp) => evaluate_add_exp(add_exp, scope_stack),
//         RelExp::Rel(left, op, right) => {
//             let left_val = evaluate_rel_exp_for_const(left, scope_stack);
//             let right_val = evaluate_add_exp(right, scope_stack);
//             match op {
//                 RelOp::Lt => (left_val < right_val) as i32,
//                 RelOp::Gt => (left_val > right_val) as i32,
//                 RelOp::Le => (left_val <= right_val) as i32,
//                 RelOp::Ge => (left_val >= right_val) as i32,
//             }
//         }
//     }
// }
// 
// // endregion 编译时常量求值
// 
// // region 运行时IR生成
// 
// fn generate_exp(exp: &Exp, func_data: &mut FunctionData, scope_stack: &mut ScopeStack) -> Value {
//     match exp {
//         Exp::LOr(lor_exp) => generate_lor_exp(lor_exp, func_data, scope_stack),
//     }
// }
// 
// fn generate_lor_exp(lor_exp: &LOrExp, func_data: &mut FunctionData, scope_stack: &mut ScopeStack) -> Value {
//     match lor_exp {
//         LOrExp::LAnd(land_exp) => generate_land_exp(land_exp, func_data, scope_stack),
//         LOrExp::LOr(left, right) => {
//             let left_value = generate_lor_exp(left, func_data, scope_stack);
//             let right_value = generate_land_exp(right, func_data, scope_stack);
//             generate_lor_binary_op(left_value, right_value, func_data)
//         }
//     }
// }
// 
// fn generate_lor_binary_op(left: Value, right: Value, func_data: &mut FunctionData) -> Value {
//     let zero = func_data.dfg_mut().new_value().integer(0);
//     let entry = func_data.layout().entry_bb().unwrap();
//     
//     let left_ne_zero = func_data.dfg_mut().new_value().binary(BinaryOp::NotEq, left, zero);
//     func_data.layout_mut().bb_mut(entry).insts_mut().push_key_back(left_ne_zero).unwrap();
//     
//     let zero2 = func_data.dfg_mut().new_value().integer(0);
//     let right_ne_zero = func_data.dfg_mut().new_value().binary(BinaryOp::NotEq, right, zero2);
//     func_data.layout_mut().bb_mut(entry).insts_mut().push_key_back(right_ne_zero).unwrap();
//     
//     let result = func_data.dfg_mut().new_value().binary(BinaryOp::Or, left_ne_zero, right_ne_zero);
//     func_data.layout_mut().bb_mut(entry).insts_mut().push_key_back(result).unwrap();
//     result
// }
// 
// fn generate_land_exp(land_exp: &LAndExp, func_data: &mut FunctionData, scope_stack: &mut ScopeStack) -> Value {
//     match land_exp {
//         LAndExp::Eq(eq_exp) => generate_eq_exp(eq_exp, func_data, scope_stack),
//         LAndExp::LAnd(left, right) => {
//             let left_value = generate_land_exp(left, func_data, scope_stack);
//             let right_value = generate_eq_exp(right, func_data, scope_stack);
//             generate_land_binary_op(left_value, right_value, func_data)
//         }
//     }
// }
// 
// fn generate_land_binary_op(left: Value, right: Value, func_data: &mut FunctionData) -> Value {
//     let zero = func_data.dfg_mut().new_value().integer(0);
//     let entry = func_data.layout().entry_bb().unwrap();
// 
//     let left_ne_zero = func_data.dfg_mut().new_value().binary(BinaryOp::NotEq, left, zero);
//     func_data.layout_mut().bb_mut(entry).insts_mut().push_key_back(left_ne_zero).unwrap();
// 
//     let zero2 = func_data.dfg_mut().new_value().integer(0);
//     let right_ne_zero = func_data.dfg_mut().new_value().binary(BinaryOp::NotEq, right, zero2);
//     func_data.layout_mut().bb_mut(entry).insts_mut().push_key_back(right_ne_zero).unwrap();
// 
//     let result = func_data.dfg_mut().new_value().binary(BinaryOp::And, left_ne_zero, right_ne_zero);
//     func_data.layout_mut().bb_mut(entry).insts_mut().push_key_back(result).unwrap();
//     result
// }
// 
// fn generate_eq_exp(eq_exp: &EqExp, func_data: &mut FunctionData, scope_stack: &mut ScopeStack) -> Value {
//     match eq_exp {
//         EqExp::Rel(rel_exp) => generate_rel_exp(rel_exp, func_data, scope_stack),
//         EqExp::Eq(left, op, right) => {
//             let left_value = generate_eq_exp(left, func_data, scope_stack);
//             let right_value = generate_rel_exp(right, func_data, scope_stack);
//             generate_eq_binary_op(op, left_value, right_value, func_data)
//         }
//     }
// }
// 
// fn generate_eq_binary_op(op: &EqOp, left: Value, right: Value, func_data: &mut FunctionData) -> Value {
//     let binary_op = match op {
//         EqOp::Eq => BinaryOp::Eq,
//         EqOp::Ne => BinaryOp::NotEq,
//     };
// 
//     let inst = func_data.dfg_mut().new_value().binary(binary_op, left, right);
//     let entry = func_data.layout().entry_bb().unwrap();
//     func_data.layout_mut().bb_mut(entry).insts_mut().push_key_back(inst).unwrap();
//     inst
// }
// 
// fn generate_rel_exp(rel_exp: &RelExp, func_data: &mut FunctionData, scope_stack: &mut ScopeStack) -> Value {
//     match rel_exp {
//         RelExp::Add(add_exp) => generate_add_exp(add_exp, func_data, scope_stack),
//         RelExp::Rel(left, op, right) => {
//             let left_value = generate_rel_exp(left, func_data, scope_stack);
//             let right_value = generate_add_exp(right, func_data, scope_stack);
//             generate_rel_binary_op(op, left_value, right_value, func_data)
//         }
//     }
// }
// 
// fn generate_rel_binary_op(op: &RelOp, left: Value, right: Value, func_data: &mut FunctionData) -> Value {
//     let binary_op = match op {
//         RelOp::Lt => BinaryOp::Lt,
//         RelOp::Gt => BinaryOp::Gt,
//         RelOp::Le => BinaryOp::Le,
//         RelOp::Ge => BinaryOp::Ge,
//     };
// 
//     let inst = func_data.dfg_mut().new_value().binary(binary_op, left, right);
//     let entry = func_data.layout().entry_bb().unwrap();
//     func_data.layout_mut().bb_mut(entry).insts_mut().push_key_back(inst).unwrap();
//     inst
// }
// 
// fn generate_add_exp(add_exp: &AddExp, func_data: &mut FunctionData, scope_stack: &mut ScopeStack) -> Value {
//     match add_exp {
//         AddExp::Mul(mul_exp) => generate_mul_exp(mul_exp, func_data, scope_stack),
//         AddExp::AddMul(left, op, right) => {
//             let left_value = generate_add_exp(left, func_data, scope_stack);
//             let right_value = generate_mul_exp(right, func_data, scope_stack);
//             generate_add_binary_op(op, left_value, right_value, func_data)
//         }
//     }
// }
// 
// fn generate_mul_exp(mul_exp: &MulExp, func_data: &mut FunctionData, scope_stack: &mut ScopeStack) -> Value {
//     match mul_exp {
//         MulExp::Unary(unary_exp) => generate_unary_exp(unary_exp, func_data, scope_stack),
//         MulExp::MulDiv(left, op, right) => {
//             let left_value = generate_mul_exp(left, func_data, scope_stack);
//             let right_value = generate_unary_exp(right, func_data, scope_stack);
//             generate_mul_binary_op(op, left_value, right_value, func_data)
//         }
//     }
// }
// 
// fn generate_add_binary_op(op: &PlusSubOp, left: Value, right: Value, func_data: &mut FunctionData) -> Value {
//     let binary_op = match op {
//         PlusSubOp::Plus => BinaryOp::Add,
//         PlusSubOp::Minus => BinaryOp::Sub,
//     };
// 
//     let inst = func_data.dfg_mut().new_value().binary(binary_op, left, right);
//     let entry = func_data.layout().entry_bb().unwrap();
//     func_data.layout_mut().bb_mut(entry).insts_mut().push_key_back(inst).unwrap();
//     inst
// }
// 
// fn generate_mul_binary_op(op: &MulDivOp, left: Value, right: Value, func_data: &mut FunctionData) -> Value {
//     let binary_op = match op {
//         MulDivOp::Mul => BinaryOp::Mul,
//         MulDivOp::Div => BinaryOp::Div,
//         MulDivOp::Mod => BinaryOp::Mod,
//     };
// 
//     let inst = func_data.dfg_mut().new_value().binary(binary_op, left, right);
//     let entry = func_data.layout().entry_bb().unwrap();
//     func_data.layout_mut().bb_mut(entry).insts_mut().push_key_back(inst).unwrap();
//     inst
// }
// 
// fn generate_unary_exp(unary_exp: &UnaryExp, func_data: &mut FunctionData, scope_stack: &mut ScopeStack) -> Value {
//     match unary_exp {
//         UnaryExp::Primary(primary) => generate_primary_exp(primary, func_data, scope_stack),
//         UnaryExp::Unary(op, exp) => {
//             let operand = generate_unary_exp(exp, func_data, scope_stack);
//             generate_unary_op(op, operand, func_data)
//         }
//     }
// }
// 
// fn generate_primary_exp(primary: &PrimaryExp, func_data: &mut FunctionData, scope_stack: &mut ScopeStack) -> Value {
//     match primary {
//         PrimaryExp::Number(num) => {
//             func_data.dfg_mut().new_value().integer(*num)
//         },
//         PrimaryExp::Paren(exp) => generate_exp(exp, func_data, scope_stack),
//         PrimaryExp::LVal(lval) => {
//             match scope_stack.lookup(&lval.ident) {
//                 Some(SymbolInfo::Const(value)) => {
//                     // 常量：直接生成整数IR
//                     func_data.dfg_mut().new_value().integer(*value)
//                 }
//                 Some(SymbolInfo::Var(ptr)) => {
//                     // 变量：生成 load 指令
//                     let load_inst = func_data.dfg_mut().new_value().load(*ptr);
//                     let entry = func_data.layout().entry_bb().unwrap();
//                     func_data.layout_mut().bb_mut(entry).insts_mut().push_key_back(load_inst).unwrap();
//                     load_inst
//                 }
//                 None => {
//                     panic!("Identifier '{}' not found", lval.ident);
//                 }
//             }
//         }
//     }
// }
// 
// fn generate_unary_op(op: &UnaryOp, operand: Value, func_data: &mut FunctionData) -> Value {
//     match op {
//         UnaryOp::Plus => operand,
//         UnaryOp::Minus => {
//             let zero = func_data.dfg_mut().new_value().integer(0);
//             let sub_inst = func_data.dfg_mut().new_value().binary(BinaryOp::Sub, zero, operand);
//             let entry = func_data.layout().entry_bb().unwrap();
//             func_data.layout_mut().bb_mut(entry).insts_mut().push_key_back(sub_inst).unwrap();
//             sub_inst
//         },
//         UnaryOp::Not => {
//             let zero = func_data.dfg_mut().new_value().integer(0);
//             let eq_inst = func_data.dfg_mut().new_value().binary(BinaryOp::Eq, operand, zero);
//             let entry = func_data.layout().entry_bb().unwrap();
//             func_data.layout_mut().bb_mut(entry).insts_mut().push_key_back(eq_inst).unwrap();
//             eq_inst
//         }
//     }
// }
// 
// // endregion 运行时IR生成
