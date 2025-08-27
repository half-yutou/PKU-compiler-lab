#[derive(Debug)]
pub struct CompUnit {
    pub func_def: FuncDef,
}

#[derive(Debug)]
pub struct FuncDef {
    pub func_type: FuncType,
    pub id: String,
    pub block: Block,
}

#[derive(Debug)]
pub enum FuncType {
    Int,
}

#[derive(Debug)]
pub struct Block {
    pub stmt: Stmt,
}

#[derive(Debug)]
pub struct Stmt {
    // 暂时只有一句 return exp ;
    pub exp: Exp,
}

// region 表达式
/// ```text
/// Exp (最低优先级)
/// └── LOrExp      (逻辑或 ||)
///     └── LAndExp  (逻辑与 &&)
///         └── EqExp    (相等比较 == !=)
///             └── RelExp   (关系比较 < > <= >=)
///                 └── AddExp   (加减 + -)
///                     └── MulExp   (乘除模 * / %)
///                         └── UnaryExp (一元运算 + - !)
///                             └── PrimaryExp (最高优先级)
/// ```
#[derive(Debug)]
pub enum Exp {
    LOr(Box<LOrExp>),
}

#[derive(Debug)]
pub enum RelExp {
    Add(Box<AddExp>),
    Rel(Box<RelExp>, RelOp, Box<AddExp>),
}

#[derive(Debug)]
pub enum EqExp {
    Rel(Box<RelExp>), 
    Eq(Box<EqExp>, EqOp, Box<RelExp>),
}

#[derive(Debug)]
pub enum LAndExp {
    Eq(Box<EqExp>),
    LAnd(Box<LAndExp>, Box<EqExp>), // 不需要op，因为只有&&
}

#[derive(Debug)]
pub enum LOrExp {
    LAnd(Box<LAndExp>),
    LOr(Box<LOrExp>, Box<LAndExp>), // 不需要op，因为只有||
}

#[derive(Debug)]
pub enum AddExp {
    Mul(Box<MulExp>),
    AddMul(Box<AddExp>, PlusSubOp, Box<MulExp>),
}

#[derive(Debug)]
pub enum MulExp {
    Unary(Box<UnaryExp>),
    MulDiv(Box<MulExp>, MulDivOp, Box<UnaryExp>),
}

#[derive(Debug)]
pub enum UnaryExp {
    Primary(PrimaryExp),
    Unary(UnaryOp, Box<UnaryExp>),
}

#[derive(Debug)]
pub enum PrimaryExp {
    Number(i32),
    Paren(Box<Exp>),
}

#[derive(Debug)]
pub enum UnaryOp {
    Plus,   // +
    Minus,  // -
    Not,    // !
}

#[derive(Debug)]
pub enum PlusSubOp {
    Plus,  // +
    Minus, // -
}

#[derive(Debug)]
pub enum MulDivOp {
    Mul, // *
    Div, // /
    Mod, // %
}

#[derive(Debug)]
pub enum RelOp {
    Lt,  // <
    Gt,  // >
    Le,  // <=
    Ge,  // >=
}

#[derive(Debug)]
pub enum EqOp {
    Eq,  // ==
    Ne,  // !=
}

#[derive(Debug)]
pub enum LogicOp {
    Or,  // ||
    And, // &&
}

// endregion 表达式
