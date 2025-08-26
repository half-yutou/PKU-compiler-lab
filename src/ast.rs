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
/// └── AddExp
///     └── MulExp
///         └── UnaryExp
///             └── PrimaryExp (最高优先级)
///                 ├── Number
///                 └── Paren(Exp) ← 括号在这里重新开始
/// ```
#[derive(Debug)]
pub enum Exp {
    AddExp(AddExp),
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
}

// endregion 表达式
