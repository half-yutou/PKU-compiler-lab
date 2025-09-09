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
use crate::lab6::ast::{AddExp, EqExp, EqOp, Exp, LAndExp, LOrExp, MulDivOp, MulExp, PlusSubOp, PrimaryExp, RelExp, RelOp, UnaryExp, UnaryOp};
use crate::lab6::irgen::symbol::SymbolInfo;
use crate::lab6::irgen::IRGen;

impl IRGen {
    pub fn evaluate_exp(&self, exp: &Exp) -> i32 {
        match exp {
            Exp::LOr(lor_exp) => self.evaluate_lor_exp(lor_exp),
        }
    }

    pub fn evaluate_lor_exp(&self, lor_exp: &LOrExp) -> i32 {
        match lor_exp {
            LOrExp::LAnd(land_exp) => self.evaluate_land_exp(land_exp),
            LOrExp::LOr(left, right) => {
                let left_val = self.evaluate_lor_exp(left);
                if left_val != 0 { // 短路返回
                    1
                } else {
                    let right_val = self.evaluate_land_exp(right);
                    if right_val != 0 { 1 } else { 0 }
                }
            }
        }
    }

    pub fn evaluate_land_exp(&self, land_exp: &LAndExp) -> i32 {
        match land_exp {
            LAndExp::Eq(eq_exp) => self.evaluate_eq_exp(eq_exp),
            LAndExp::LAnd(left, right) => {
                let left_val = self.evaluate_land_exp(left);
                if left_val == 0 { // 短路返回
                    0
                } else {
                    let right_val = self.evaluate_eq_exp(right);
                    if right_val != 0 { 1 } else { 0 }
                }
            }
        }
    }

    pub fn evaluate_eq_exp(&self, eq_exp: &EqExp) -> i32 {
        match eq_exp {
            EqExp::Rel(rel_exp) => self.evaluate_rel_exp(rel_exp),
            EqExp::Eq(left, op, right) => {
                let left_val = self.evaluate_eq_exp(left);
                let right_val = self.evaluate_rel_exp(right);
                match op {
                    EqOp::Eq => (left_val == right_val) as i32,
                    EqOp::Ne => (left_val != right_val) as i32,
                }
            }
        }
    }

    pub fn evaluate_rel_exp(&self, rel_exp: &RelExp) -> i32 {
        match rel_exp {
            RelExp::Add(add_exp) => self.evaluate_add_exp(add_exp),
            RelExp::Rel(left, op, right) => {
                let left_val = self.evaluate_rel_exp(left);
                let right_val = self.evaluate_add_exp(right);
                match op {
                    RelOp::Lt => (left_val < right_val) as i32,
                    RelOp::Gt => (left_val > right_val) as i32,
                    RelOp::Le => (left_val <= right_val) as i32,
                    RelOp::Ge => (left_val >= right_val) as i32,
                }
            }
        }
    }

    pub fn evaluate_add_exp(&self, add_exp: &AddExp) -> i32 {
        match add_exp {
            AddExp::Mul(mul_exp) => self.evaluate_mul_exp(mul_exp), 
            AddExp::AddMul(left, op, right) => {
                let left_val = self.evaluate_add_exp(left);
                let right_val = self.evaluate_mul_exp(right);
                match op {
                    PlusSubOp::Plus => left_val + right_val, 
                    PlusSubOp::Minus => left_val - right_val, 
                }
            }
        }
    }

    pub fn evaluate_mul_exp(&self, mul_exp: &MulExp) -> i32 {
        match mul_exp {
            MulExp::Unary(unary_exp) => self.evaluate_unary_exp(unary_exp),
            MulExp::MulDiv(left, op, right) => {
                let left_val = self.evaluate_mul_exp(left);
                let right_val = self.evaluate_unary_exp(right);
                match op {
                    MulDivOp::Mul => left_val * right_val,
                    MulDivOp::Div => left_val / right_val,
                    MulDivOp::Mod => left_val % right_val,
                }
            }
        }
    }

    pub fn evaluate_unary_exp(&self, unary_exp: &UnaryExp) -> i32 {
        match unary_exp {
            UnaryExp::Primary(primary) => self.evaluate_primary_exp(primary),
            UnaryExp::Unary(op, exp) => {
                let val = self.evaluate_unary_exp(exp);
                match op {
                    UnaryOp::Plus => val,
                    UnaryOp::Minus => -val,
                    UnaryOp::Not => if val == 0 { 1 } else { 0 },
                }
            }
        }
    }

    pub fn evaluate_primary_exp(&self, primary: &PrimaryExp) -> i32 {
        match primary {
            PrimaryExp::Number(num) => *num,
            PrimaryExp::Paren(exp) => self.evaluate_exp(exp),
            PrimaryExp::LVal(lval) => {
                match self.scope_stack.lookup(&lval.ident) {
                    Some(SymbolInfo::Const(value)) => *value,
                    Some(SymbolInfo::Var(_)) => panic!("Cannot use variable '{}' in constant expression", lval.ident),
                    None => panic!("Identifier '{}' not found", lval.ident),
                }
            }
        }
    }
}