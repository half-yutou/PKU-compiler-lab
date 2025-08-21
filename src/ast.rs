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
    Int
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

#[derive(Debug)]
pub enum Exp {
    Unary(UnaryExp), 
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
