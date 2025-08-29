use crate::ast::{AddExp, Block, BlockItem, CompUnit, ConstExp, Decl, EqExp, EqOp, Exp, LAndExp, LOrExp, MulDivOp, MulExp, PlusSubOp, PrimaryExp, RelExp, RelOp, Stmt, UnaryExp, UnaryOp};
use koopa::ir::builder::{BasicBlockBuilder, LocalInstBuilder, ValueBuilder};
use koopa::ir::{BinaryOp, FunctionData, Program, Type, Value};
use std::collections::HashMap;

// 符号信息：区分常量和变量
#[derive(Debug, Clone)]
enum SymbolInfo {
    Const(i32),           // 常量：直接存储值
    Var(Value),           // 变量：存储 alloc 返回的指针
}

// 符号表：存储标识符到符号信息的映射
type SymbolTable = HashMap<String, SymbolInfo>;

pub fn generate_koopa_ir(ast: CompUnit) -> Program {
    let mut program = Program::new();
    let main_func = program.new_func(FunctionData::with_param_names(
        "@main".into(),
        vec![],
        Type::get_i32(),
    ));
    let main_data = program.func_mut(main_func);

    let entry = main_data.dfg_mut().new_bb().basic_block(Some("%entry".into()));
    main_data.layout_mut().bbs_mut().extend([entry]);

    // 创建符号表
    let mut symbol_table = SymbolTable::new();

    // 处理函数体
    generate_block(&ast.func_def.block, main_data, &mut symbol_table);

    program
}

// 处理代码块
fn generate_block(block: &Block, func_data: &mut FunctionData, symbol_table: &mut SymbolTable) {
    for item in &block.block_item_list {
        match item {
            BlockItem::Decl(decl) => generate_decl(decl, func_data, symbol_table),
            BlockItem::Stmt(stmt) => generate_stmt(stmt, func_data, symbol_table),
        }
    }
}

// 处理声明（常量定义和变量定义）
fn generate_decl(decl: &Decl, func_data: &mut FunctionData, symbol_table: &mut SymbolTable) {
    match decl {
        Decl::Const(const_decl) => {
            for def in &const_decl.const_def_list {
                // 编译时计算常量值
                let value = evaluate_const_exp(&def.const_init_val.const_exp, symbol_table);
                // 存入符号表
                symbol_table.insert(def.ident.clone(), SymbolInfo::Const(value));
            }
        }
        Decl::Var(var_decl) => {
            for def in &var_decl.var_def_list {
                // 为变量分配内存
                let alloc_inst = func_data.dfg_mut().new_value().alloc(Type::get_i32());
                let entry = func_data.layout().entry_bb().unwrap();
                func_data.layout_mut().bb_mut(entry).insts_mut().push_key_back(alloc_inst).unwrap();
                
                // 如果有初始化值，生成 store 指令
                if let Some(init_val) = &def.init_val {
                    let init_value = generate_exp(&init_val.exp, func_data, symbol_table);
                    let store_inst = func_data.dfg_mut().new_value().store(init_value, alloc_inst);
                    func_data.layout_mut().bb_mut(entry).insts_mut().push_key_back(store_inst).unwrap();
                }
                
                // 存入符号表
                symbol_table.insert(def.ident.clone(), SymbolInfo::Var(alloc_inst));
            }
        }
    }
}

// 处理语句
fn generate_stmt(stmt: &Stmt, func_data: &mut FunctionData, symbol_table: &mut SymbolTable) {
    match stmt {
        Stmt::Return(exp) => {
            let value = generate_exp(exp, func_data, symbol_table);
            let ret = func_data.dfg_mut().new_value().ret(Some(value));
            let entry = func_data.layout().entry_bb().unwrap();
            func_data.layout_mut().bb_mut(entry).insts_mut().push_key_back(ret).unwrap();
        }
        Stmt::Assign(lval, exp) => {
            // 生成右侧表达式的值
            let value = generate_exp(exp, func_data, symbol_table);
            
            // 获取左值对应的指针
            match symbol_table.get(&lval.ident) {
                Some(SymbolInfo::Var(ptr)) => {
                    // 生成 store 指令
                    let store_inst = func_data.dfg_mut().new_value().store(value, *ptr);
                    let entry = func_data.layout().entry_bb().unwrap();
                    func_data.layout_mut().bb_mut(entry).insts_mut().push_key_back(store_inst).unwrap();
                }
                Some(SymbolInfo::Const(_)) => {
                    panic!("Cannot assign to constant '{}'!", lval.ident);
                }
                None => {
                    panic!("Variable '{}' not found!", lval.ident);
                }
            }
        }
        Stmt::Exp(Some(exp)) => {
            // 表达式语句，生成IR但丢弃结果
            generate_exp(exp, func_data, symbol_table);
        }
        Stmt::Exp(None) => {
            // 空语句，什么也不做
        }
        Stmt::Block(block) => {
            // 嵌套代码块
            generate_block(block, func_data, symbol_table);
        }
    }
}

// region 编译时常量求值

fn evaluate_const_exp(const_exp: &ConstExp, symbol_table: &SymbolTable) -> i32 {
    evaluate_lor_exp_for_const(&const_exp.lor_exp, symbol_table)
}

fn evaluate_add_exp(add_exp: &AddExp, symbol_table: &SymbolTable) -> i32 {
    match add_exp {
        AddExp::Mul(mul_exp) => evaluate_mul_exp(mul_exp, symbol_table),
        AddExp::AddMul(left, op, right) => {
            let left_val = evaluate_add_exp(left, symbol_table);
            let right_val = evaluate_mul_exp(right, symbol_table);
            match op {
                PlusSubOp::Plus => left_val + right_val,
                PlusSubOp::Minus => left_val - right_val,
            }
        }
    }
}

fn evaluate_mul_exp(mul_exp: &MulExp, symbol_table: &SymbolTable) -> i32 {
    match mul_exp {
        MulExp::Unary(unary_exp) => evaluate_unary_exp(unary_exp, symbol_table),
        MulExp::MulDiv(left, op, right) => {
            let left_val = evaluate_mul_exp(left, symbol_table);
            let right_val = evaluate_unary_exp(right, symbol_table);
            match op {
                MulDivOp::Mul => left_val * right_val,
                MulDivOp::Div => left_val / right_val,
                MulDivOp::Mod => left_val % right_val,
            }
        }
    }
}

fn evaluate_unary_exp(unary_exp: &UnaryExp, symbol_table: &SymbolTable) -> i32 {
    match unary_exp {
        UnaryExp::Primary(primary) => evaluate_primary_exp(primary, symbol_table),
        UnaryExp::Unary(op, exp) => {
            let val = evaluate_unary_exp(exp, symbol_table);
            match op {
                UnaryOp::Plus => val,
                UnaryOp::Minus => -val,
                UnaryOp::Not => if val == 0 { 1 } else { 0 },
            }
        }
    }
}

fn evaluate_primary_exp(primary: &PrimaryExp, symbol_table: &SymbolTable) -> i32 {
    match primary {
        PrimaryExp::Number(num) => *num,
        PrimaryExp::Paren(exp) => evaluate_exp_for_const(exp, symbol_table),
        PrimaryExp::LVal(lval) => {
            match symbol_table.get(&lval.ident) {
                Some(SymbolInfo::Const(value)) => *value,
                Some(SymbolInfo::Var(_)) => panic!("Cannot use variable '{}' in constant expression", lval.ident),
                None => panic!("Identifier '{}' not found", lval.ident),
            }
        }
    }
}

fn evaluate_exp_for_const(exp: &Exp, symbol_table: &SymbolTable) -> i32 {
    match exp {
        Exp::LOr(lor_exp) => evaluate_lor_exp_for_const(lor_exp, symbol_table),
    }
}

fn evaluate_lor_exp_for_const(lor_exp: &LOrExp, symbol_table: &SymbolTable) -> i32 {
    match lor_exp {
        LOrExp::LAnd(land_exp) => evaluate_land_exp_for_const(land_exp, symbol_table),
        LOrExp::LOr(left, right) => {
            let left_val = evaluate_lor_exp_for_const(left, symbol_table);
            if left_val != 0 {
                1
            } else {
                if evaluate_land_exp_for_const(right, symbol_table) != 0 { 1 } else { 0 }
            }
        }
    }
}

fn evaluate_land_exp_for_const(land_exp: &LAndExp, symbol_table: &SymbolTable) -> i32 {
    match land_exp {
        LAndExp::Eq(eq_exp) => evaluate_eq_exp_for_const(eq_exp, symbol_table),
        LAndExp::LAnd(left, right) => {
            let left_val = evaluate_land_exp_for_const(left, symbol_table);
            if left_val == 0 {
                0
            } else {
                if evaluate_eq_exp_for_const(right, symbol_table) != 0 { 1 } else { 0 }
            }
        }
    }
}

fn evaluate_eq_exp_for_const(eq_exp: &EqExp, symbol_table: &SymbolTable) -> i32 {
    match eq_exp {
        EqExp::Rel(rel_exp) => evaluate_rel_exp_for_const(rel_exp, symbol_table),
        EqExp::Eq(left, op, right) => {
            let left_val = evaluate_eq_exp_for_const(left, symbol_table);
            let right_val = evaluate_rel_exp_for_const(right, symbol_table);
            match op {
                EqOp::Eq => (left_val == right_val) as i32,
                EqOp::Ne => (left_val != right_val) as i32,
            }
        }
    }
}

fn evaluate_rel_exp_for_const(rel_exp: &RelExp, symbol_table: &SymbolTable) -> i32 {
    match rel_exp {
        RelExp::Add(add_exp) => evaluate_add_exp(add_exp, symbol_table),
        RelExp::Rel(left, op, right) => {
            let left_val = evaluate_rel_exp_for_const(left, symbol_table);
            let right_val = evaluate_add_exp(right, symbol_table);
            match op {
                RelOp::Lt => (left_val < right_val) as i32,
                RelOp::Gt => (left_val > right_val) as i32,
                RelOp::Le => (left_val <= right_val) as i32,
                RelOp::Ge => (left_val >= right_val) as i32,
            }
        }
    }
}

// endregion 编译时常量求值

// region 运行时IR生成（需要修改 generate_primary_exp）

fn generate_exp(exp: &Exp, func_data: &mut FunctionData, symbol_table: &mut SymbolTable) -> Value {
    match exp {
        Exp::LOr(lor_exp) => generate_lor_exp(lor_exp, func_data, symbol_table),
    }
}

fn generate_lor_exp(lor_exp: &LOrExp, func_data: &mut FunctionData, symbol_table: &mut SymbolTable) -> Value {
    match lor_exp {
        LOrExp::LAnd(land_exp) => generate_land_exp(land_exp, func_data, symbol_table),
        LOrExp::LOr(left, right) => {
            let left_value = generate_lor_exp(left, func_data, symbol_table);
            let right_value = generate_land_exp(right, func_data, symbol_table);
            generate_lor_binary_op(left_value, right_value, func_data)
        }
    }
}

fn generate_lor_binary_op(left: Value, right: Value, func_data: &mut FunctionData) -> Value {
    let zero = func_data.dfg_mut().new_value().integer(0);
    let entry = func_data.layout().entry_bb().unwrap();
    
    let left_ne_zero = func_data.dfg_mut().new_value().binary(BinaryOp::NotEq, left, zero);
    func_data.layout_mut().bb_mut(entry).insts_mut().push_key_back(left_ne_zero).unwrap();
    
    let zero2 = func_data.dfg_mut().new_value().integer(0);
    let right_ne_zero = func_data.dfg_mut().new_value().binary(BinaryOp::NotEq, right, zero2);
    func_data.layout_mut().bb_mut(entry).insts_mut().push_key_back(right_ne_zero).unwrap();
    
    let result = func_data.dfg_mut().new_value().binary(BinaryOp::Or, left_ne_zero, right_ne_zero);
    func_data.layout_mut().bb_mut(entry).insts_mut().push_key_back(result).unwrap();
    result
}

fn generate_land_exp(land_exp: &LAndExp, func_data: &mut FunctionData, symbol_table: &mut SymbolTable) -> Value {
    match land_exp {
        LAndExp::Eq(eq_exp) => generate_eq_exp(eq_exp, func_data, symbol_table),
        LAndExp::LAnd(left, right) => {
            let left_value = generate_land_exp(left, func_data, symbol_table);
            let right_value = generate_eq_exp(right, func_data, symbol_table);
            generate_land_binary_op(left_value, right_value, func_data)
        }
    }
}

fn generate_land_binary_op(left: Value, right: Value, func_data: &mut FunctionData) -> Value {
    let zero = func_data.dfg_mut().new_value().integer(0);
    let entry = func_data.layout().entry_bb().unwrap();

    let left_ne_zero = func_data.dfg_mut().new_value().binary(BinaryOp::NotEq, left, zero);
    func_data.layout_mut().bb_mut(entry).insts_mut().push_key_back(left_ne_zero).unwrap();

    let zero2 = func_data.dfg_mut().new_value().integer(0);
    let right_ne_zero = func_data.dfg_mut().new_value().binary(BinaryOp::NotEq, right, zero2);
    func_data.layout_mut().bb_mut(entry).insts_mut().push_key_back(right_ne_zero).unwrap();

    let result = func_data.dfg_mut().new_value().binary(BinaryOp::And, left_ne_zero, right_ne_zero);
    func_data.layout_mut().bb_mut(entry).insts_mut().push_key_back(result).unwrap();
    result
}

fn generate_eq_exp(eq_exp: &EqExp, func_data: &mut FunctionData, symbol_table: &mut SymbolTable) -> Value {
    match eq_exp {
        EqExp::Rel(rel_exp) => generate_rel_exp(rel_exp, func_data, symbol_table),
        EqExp::Eq(left, op, right) => {
            let left_value = generate_eq_exp(left, func_data, symbol_table);
            let right_value = generate_rel_exp(right, func_data, symbol_table);
            generate_eq_binary_op(op, left_value, right_value, func_data)
        }
    }
}

fn generate_eq_binary_op(op: &EqOp, left: Value, right: Value, func_data: &mut FunctionData) -> Value {
    let binary_op = match op {
        EqOp::Eq => BinaryOp::Eq,
        EqOp::Ne => BinaryOp::NotEq,
    };

    let inst = func_data.dfg_mut().new_value().binary(binary_op, left, right);
    let entry = func_data.layout().entry_bb().unwrap();
    func_data.layout_mut().bb_mut(entry).insts_mut().push_key_back(inst).unwrap();
    inst
}

fn generate_rel_exp(rel_exp: &RelExp, func_data: &mut FunctionData, symbol_table: &mut SymbolTable) -> Value {
    match rel_exp {
        RelExp::Add(add_exp) => generate_add_exp(add_exp, func_data, symbol_table),
        RelExp::Rel(left, op, right) => {
            let left_value = generate_rel_exp(left, func_data, symbol_table);
            let right_value = generate_add_exp(right, func_data, symbol_table);
            generate_rel_binary_op(op, left_value, right_value, func_data)
        }
    }
}

fn generate_rel_binary_op(op: &RelOp, left: Value, right: Value, func_data: &mut FunctionData) -> Value {
    let binary_op = match op {
        RelOp::Lt => BinaryOp::Lt,
        RelOp::Gt => BinaryOp::Gt,
        RelOp::Le => BinaryOp::Le,
        RelOp::Ge => BinaryOp::Ge,
    };

    let inst = func_data.dfg_mut().new_value().binary(binary_op, left, right);
    let entry = func_data.layout().entry_bb().unwrap();
    func_data.layout_mut().bb_mut(entry).insts_mut().push_key_back(inst).unwrap();
    inst
}

fn generate_add_exp(add_exp: &AddExp, func_data: &mut FunctionData, symbol_table: &mut SymbolTable) -> Value {
    match add_exp {
        AddExp::Mul(mul_exp) => generate_mul_exp(mul_exp, func_data, symbol_table),
        AddExp::AddMul(left, op, right) => {
            let left_value = generate_add_exp(left, func_data, symbol_table);
            let right_value = generate_mul_exp(right, func_data, symbol_table);
            generate_add_binary_op(op, left_value, right_value, func_data)
        }
    }
}

fn generate_mul_exp(mul_exp: &MulExp, func_data: &mut FunctionData, symbol_table: &mut SymbolTable) -> Value {
    match mul_exp {
        MulExp::Unary(unary_exp) => generate_unary_exp(unary_exp, func_data, symbol_table),
        MulExp::MulDiv(left, op, right) => {
            let left_value = generate_mul_exp(left, func_data, symbol_table);
            let right_value = generate_unary_exp(right, func_data, symbol_table);
            generate_mul_binary_op(op, left_value, right_value, func_data)
        }
    }
}

fn generate_add_binary_op(op: &PlusSubOp, left: Value, right: Value, func_data: &mut FunctionData) -> Value {
    let binary_op = match op {
        PlusSubOp::Plus => BinaryOp::Add,
        PlusSubOp::Minus => BinaryOp::Sub,
    };

    let inst = func_data.dfg_mut().new_value().binary(binary_op, left, right);
    let entry = func_data.layout().entry_bb().unwrap();
    func_data.layout_mut().bb_mut(entry).insts_mut().push_key_back(inst).unwrap();
    inst
}

fn generate_mul_binary_op(op: &MulDivOp, left: Value, right: Value, func_data: &mut FunctionData) -> Value {
    let binary_op = match op {
        MulDivOp::Mul => BinaryOp::Mul,
        MulDivOp::Div => BinaryOp::Div,
        MulDivOp::Mod => BinaryOp::Mod,
    };

    let inst = func_data.dfg_mut().new_value().binary(binary_op, left, right);
    let entry = func_data.layout().entry_bb().unwrap();
    func_data.layout_mut().bb_mut(entry).insts_mut().push_key_back(inst).unwrap();
    inst
}

fn generate_unary_exp(unary_exp: &UnaryExp, func_data: &mut FunctionData, symbol_table: &mut SymbolTable) -> Value {
    match unary_exp {
        UnaryExp::Primary(primary) => generate_primary_exp(primary, func_data, symbol_table),
        UnaryExp::Unary(op, exp) => {
            let operand = generate_unary_exp(exp, func_data, symbol_table);
            generate_unary_op(op, operand, func_data)
        }
    }
}

fn generate_primary_exp(primary: &PrimaryExp, func_data: &mut FunctionData, symbol_table: &mut SymbolTable) -> Value {
    match primary {
        PrimaryExp::Number(num) => {
            func_data.dfg_mut().new_value().integer(*num)
        },
        PrimaryExp::Paren(exp) => generate_exp(exp, func_data, symbol_table),
        PrimaryExp::LVal(lval) => {
            match symbol_table.get(&lval.ident) {
                Some(SymbolInfo::Const(value)) => {
                    // 常量：直接生成整数IR
                    func_data.dfg_mut().new_value().integer(*value)
                }
                Some(SymbolInfo::Var(ptr)) => {
                    // 变量：生成 load 指令
                    let load_inst = func_data.dfg_mut().new_value().load(*ptr);
                    let entry = func_data.layout().entry_bb().unwrap();
                    func_data.layout_mut().bb_mut(entry).insts_mut().push_key_back(load_inst).unwrap();
                    load_inst
                }
                None => {
                    panic!("Identifier '{}' not found", lval.ident);
                }
            }
        }
    }
}

fn generate_unary_op(op: &UnaryOp, operand: Value, func_data: &mut FunctionData) -> Value {
    match op {
        UnaryOp::Plus => operand,
        UnaryOp::Minus => {
            let zero = func_data.dfg_mut().new_value().integer(0);
            let sub_inst = func_data.dfg_mut().new_value().binary(BinaryOp::Sub, zero, operand);
            let entry = func_data.layout().entry_bb().unwrap();
            func_data.layout_mut().bb_mut(entry).insts_mut().push_key_back(sub_inst).unwrap();
            sub_inst
        },
        UnaryOp::Not => {
            let zero = func_data.dfg_mut().new_value().integer(0);
            let eq_inst = func_data.dfg_mut().new_value().binary(BinaryOp::Eq, operand, zero);
            let entry = func_data.layout().entry_bb().unwrap();
            func_data.layout_mut().bb_mut(entry).insts_mut().push_key_back(eq_inst).unwrap();
            eq_inst
        }
    }
}

// endregion 运行时IR生成
