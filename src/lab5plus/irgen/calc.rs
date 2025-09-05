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
use crate::ast::{AddExp, EqExp, EqOp, Exp, LAndExp, LOrExp, MulDivOp, MulExp, PlusSubOp, PrimaryExp, RelExp, RelOp, UnaryExp, UnaryOp};
use crate::lab5plus::irgen::symbol::{ScopeStack, SymbolInfo};
use crate::lab5plus::irgen::calc;

pub fn evaluate_exp(exp: &Exp, scope_stack: &ScopeStack) -> i32 {
    match exp {
        Exp::LOr(lor_exp) => evaluate_lor_exp(lor_exp, scope_stack),
    }
}

pub fn evaluate_lor_exp(lor_exp: &LOrExp, scope_stack: &ScopeStack) -> i32 {
    match lor_exp {
        LOrExp::LAnd(land_exp) => evaluate_land_exp(land_exp, scope_stack),
        LOrExp::LOr(left, right) => {
            let left_val = evaluate_lor_exp(left, scope_stack);
            if left_val != 0 { // 短路返回
                1
            } else {
                let right_val = evaluate_land_exp(right, scope_stack);
                if right_val != 0 { 1 } else { 0 }
            }
        }
    }
}

pub fn evaluate_land_exp(land_exp: &LAndExp, scope_stack: &ScopeStack) -> i32 {
    match land_exp {
        LAndExp::Eq(eq_exp) => evaluate_eq_exp(eq_exp, scope_stack),
        LAndExp::LAnd(left, right) => {
            let left_val = evaluate_land_exp(left, scope_stack);
            if left_val == 0 { // 短路返回
                0
            } else {
                let right_val = evaluate_eq_exp(right, scope_stack);
                if right_val != 0 { 1 } else { 0 }
            }
        }
    }
}

pub fn evaluate_eq_exp(eq_exp: &EqExp, scope_stack: &ScopeStack) -> i32 {
    match eq_exp {
        EqExp::Rel(rel_exp) => evaluate_rel_exp(rel_exp, scope_stack),
        EqExp::Eq(left, op, right) => {
            let left_val = evaluate_eq_exp(left, scope_stack);
            let right_val = evaluate_rel_exp(right, scope_stack);
            match op {
                EqOp::Eq => (left_val == right_val) as i32,
                EqOp::Ne => (left_val != right_val) as i32,
            }
        }
    }
}

pub fn evaluate_rel_exp(rel_exp: &RelExp, scope_stack: &ScopeStack) -> i32 {
    match rel_exp {
        RelExp::Add(add_exp) => calc::evaluate_add_exp(add_exp, scope_stack),
        RelExp::Rel(left, op, right) => {
            let left_val = evaluate_rel_exp(left, scope_stack);
            let right_val = calc::evaluate_add_exp(right, scope_stack);
            match op {
                RelOp::Lt => (left_val < right_val) as i32,
                RelOp::Gt => (left_val > right_val) as i32,
                RelOp::Le => (left_val <= right_val) as i32,
                RelOp::Ge => (left_val >= right_val) as i32,
            }
        }
    }
}

pub fn evaluate_add_exp(add_exp: &AddExp, scope_stack: &ScopeStack) -> i32 {
    match add_exp {
        AddExp::Mul(mul_exp) => evaluate_mul_exp(mul_exp, scope_stack), 
        AddExp::AddMul(left, op, right) => {
            let left_val = evaluate_add_exp(left, scope_stack);
            let right_val = evaluate_mul_exp(right, scope_stack);
            match op {
                PlusSubOp::Plus => left_val + right_val, 
                PlusSubOp::Minus => left_val - right_val, 
            }
        }
    }
}

pub fn evaluate_mul_exp(mul_exp: &MulExp, scope_stack: &ScopeStack) -> i32 {
    match mul_exp {
        MulExp::Unary(unary_exp) => evaluate_unary_exp(unary_exp, scope_stack),
        MulExp::MulDiv(left, op, right) => {
            let left_val = evaluate_mul_exp(left, scope_stack);
            let right_val = evaluate_unary_exp(right, scope_stack);
            match op {
                MulDivOp::Mul => left_val * right_val,
                MulDivOp::Div => left_val / right_val,
                MulDivOp::Mod => left_val % right_val,
            }
        }
    }
}

pub fn evaluate_unary_exp(unary_exp: &UnaryExp, scope_stack: &ScopeStack) -> i32 {
    match unary_exp {
        UnaryExp::Primary(primary) => evaluate_primary_exp(primary, scope_stack),
        UnaryExp::Unary(op, exp) => {
            let val = evaluate_unary_exp(exp, scope_stack);
            match op {
                UnaryOp::Plus => val,
                UnaryOp::Minus => -val,
                UnaryOp::Not => if val == 0 { 1 } else { 0 },
            }
        }
    }
}

pub fn evaluate_primary_exp(primary: &PrimaryExp, scope_stack: &ScopeStack) -> i32 {
    match primary {
        PrimaryExp::Number(num) => *num,
        PrimaryExp::Paren(exp) => evaluate_exp(exp, scope_stack),
        PrimaryExp::LVal(lval) => {
            match scope_stack.lookup(&lval.ident) {
                Some(SymbolInfo::Const(value)) => *value,
                Some(SymbolInfo::Var(_)) => panic!("Cannot use variable '{}' in constant expression", lval.ident),
                None => panic!("Identifier '{}' not found", lval.ident),
            }
        }
    }
}