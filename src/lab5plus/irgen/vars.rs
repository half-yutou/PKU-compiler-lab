//! ```text
//! Exp (最低优先级)
//! └── LOrExp      (逻辑或 ||)
//!     └── LAndExp  (逻辑与 &&)
//!         └── EqExp    (相等比较 == !=)
//!             └── RelExp   (关系比较 < > <= >=)
//!                 └── AddExp   (加减 + -)
//!                     └── MulExp   (乘除模 * / %)
//!                         └── UnaryExp (一元运算 + - !)
//!                             └── PrimaryExp (最高优先级)
//! ```
use koopa::ir::{BinaryOp, FunctionData, Value};
use koopa::ir::builder::{LocalInstBuilder, ValueBuilder};
use crate::lab5plus::ast::{AddExp, EqExp, EqOp, Exp, LAndExp, LOrExp, MulDivOp, MulExp, PlusSubOp, PrimaryExp, RelExp, RelOp, UnaryExp, UnaryOp};
use crate::lab5plus::irgen::symbol::{ScopeStack, SymbolInfo};

pub fn generate_exp(exp: &Exp, func_data: &mut FunctionData, scope_stack: &mut ScopeStack) -> Value {
    match exp { 
        Exp::LOr(lor_exp) => generate_lor_exp(lor_exp, func_data, scope_stack)
    }
}

/// 生成lor表达式的ir
fn generate_lor_exp(lor_exp: &LOrExp, func_data: &mut FunctionData, scope_stack: &mut ScopeStack) -> Value {
    match lor_exp {
        LOrExp::LAnd(land_exp) => generate_land_exp(land_exp, func_data, scope_stack),
        LOrExp::LOr(left, right) => {
            let left_value = generate_lor_exp(left, func_data, scope_stack);
            let right_value = generate_land_exp(right, func_data, scope_stack);
            generate_lor_binary_op(left_value, right_value, func_data)
        }
    }
}

/// koopaIR 只支持按位或，如何实现逻辑或？
/// ```txt
/// (left || right) <=> (left != 0) | (right != 0)
/// ```
/// 但是没有实现短路求值，后续版本会引入新方法
fn generate_lor_binary_op(left: Value, right: Value, func_data: &mut FunctionData) -> Value {
    let entry = func_data.layout().entry_bb().unwrap();

    let left_zero = func_data.dfg_mut().new_value().integer(0);
    let left_ne_zero = func_data.dfg_mut().new_value().binary(BinaryOp::NotEq, left, left_zero);
    func_data.layout_mut().bb_mut(entry).insts_mut().push_key_back(left_ne_zero).unwrap();

    let right_zero = func_data.dfg_mut().new_value().integer(0);
    let right_ne_zero = func_data.dfg_mut().new_value().binary(BinaryOp::NotEq, right, right_zero);
    func_data.layout_mut().bb_mut(entry).insts_mut().push_key_back(right_ne_zero).unwrap();

    let result = func_data.dfg_mut().new_value().binary(BinaryOp::Or, left_ne_zero, right_ne_zero);
    func_data.layout_mut().bb_mut(entry).insts_mut().push_key_back(result).unwrap();
    result
}

/// 生成land表达式的ir
fn generate_land_exp(land_exp: &LAndExp, func_data: &mut FunctionData, scope_stack: &mut ScopeStack) -> Value {
    match land_exp {
        LAndExp::Eq(eq_exp) => generate_eq_exp(eq_exp, func_data, scope_stack),
        LAndExp::LAnd(left, right) => {
            let left_value = generate_land_exp(left, func_data, scope_stack);
            let right_value = generate_eq_exp(right, func_data, scope_stack);
            generate_land_binary_op(left_value, right_value, func_data)
        }
    }
}

/// koopaIR只支持按位与，如何实现逻辑与
/// ```txt
/// (left && right) <=> (left != 0) & (right != 0)
/// ```
/// 但是没有实现短路求值，后续版本会引入新方法
fn generate_land_binary_op(left: Value, right: Value, func_data: &mut FunctionData) -> Value {
    let entry = func_data.layout().entry_bb().unwrap();

    let left_zero = func_data.dfg_mut().new_value().integer(0);
    let left_ne_zero = func_data.dfg_mut().new_value().binary(BinaryOp::NotEq, left, left_zero);
    func_data.layout_mut().bb_mut(entry).insts_mut().push_key_back(left_ne_zero).unwrap();

    let right_zero = func_data.dfg_mut().new_value().integer(0);
    let right_ne_zero = func_data.dfg_mut().new_value().binary(BinaryOp::NotEq, right, right_zero);
    func_data.layout_mut().bb_mut(entry).insts_mut().push_key_back(right_ne_zero).unwrap();

    let result = func_data.dfg_mut().new_value().binary(BinaryOp::And, left_ne_zero, right_ne_zero);
    func_data.layout_mut().bb_mut(entry).insts_mut().push_key_back(result).unwrap();
    result
}

/// 生成eq表达式的ir
fn generate_eq_exp(eq_exp: &EqExp, func_data: &mut FunctionData, scope_stack: &mut ScopeStack) -> Value {
    match eq_exp {
        EqExp::Rel(rel_exp) => generate_rel_exp(rel_exp, func_data, scope_stack),
        EqExp::Eq(left, op, right) => {
            let left_value = generate_eq_exp(left, func_data, scope_stack);
            let right_value = generate_rel_exp(right, func_data, scope_stack);
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

/// 生成rel表达式的ir
fn generate_rel_exp(rel_exp: &RelExp, func_data: &mut FunctionData, scope_stack: &mut ScopeStack) -> Value {
    match rel_exp {
        RelExp::Add(add_exp) => generate_add_exp(add_exp, func_data, scope_stack),
        RelExp::Rel(left, op, right) => {
            let left_value = generate_rel_exp(left, func_data, scope_stack);
            let right_value = generate_add_exp(right, func_data, scope_stack);
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

/// 生成add表达式的ir
fn generate_add_exp(add_exp: &AddExp, func_data: &mut FunctionData, scope_stack: &mut ScopeStack) -> Value {
    match add_exp {
        AddExp::Mul(mul_exp) => generate_mul_exp(mul_exp, func_data, scope_stack),
        AddExp::AddMul(left, op, right) => {
            let left_value = generate_add_exp(left, func_data, scope_stack);
            let right_value = generate_mul_exp(right, func_data, scope_stack);
            generate_add_binary_op(op, left_value, right_value, func_data)
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

/// 生成mul表达式的ir
fn generate_mul_exp(mul_exp: &MulExp, func_data: &mut FunctionData, scope_stack: &mut ScopeStack) -> Value {
    match mul_exp {
        MulExp::Unary(unary_exp) => generate_unary_exp(unary_exp, func_data, scope_stack),
        MulExp::MulDiv(left, op, right) => {
            let left_value = generate_mul_exp(left, func_data, scope_stack);
            let right_value = generate_unary_exp(right, func_data, scope_stack);
            generate_mul_binary_op(op, left_value, right_value, func_data)
        }
    }
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

/// 生成unary表达式(一元表达式)的ir
fn generate_unary_exp(unary_exp: &UnaryExp, func_data: &mut FunctionData, scope_stack: &mut ScopeStack) -> Value {
    match unary_exp {
        UnaryExp::Primary(primary) => generate_primary_exp(primary, func_data, scope_stack),
        UnaryExp::Unary(op, exp) => {
            let operand = generate_unary_exp(exp, func_data, scope_stack);
            generate_unary_op(op, operand, func_data)
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

fn generate_primary_exp(primary: &PrimaryExp, func_data: &mut FunctionData, scope_stack: &mut ScopeStack) -> Value {
    match primary {
        PrimaryExp::Number(num) => {
            func_data.dfg_mut().new_value().integer(*num)
        },
        PrimaryExp::Paren(exp) => generate_exp(exp, func_data, scope_stack),
        PrimaryExp::LVal(lval) => {
            match scope_stack.lookup(&lval.ident) {
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
