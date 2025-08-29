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
    pub block_item_list: Vec<BlockItem>,
}

#[derive(Debug)]
pub enum BlockItem {
    Decl(Decl),
    Stmt(Stmt),
}

#[derive(Debug)]
pub enum Stmt {
    Return(Exp),
    Exp(Option<Exp>),
    Block(Block),
    Assign(LVal, Exp) // 赋值语句: LVal = Exp
}
#[derive(Debug)]
pub enum Decl {
    Const(ConstDecl),
    Var(VarDecl),
}

#[derive(Debug)]
pub struct LVal {
    pub ident: String,
}

// region 常量声明

#[derive(Debug)]
pub struct ConstDecl {
    pub b_type: String,
    pub const_def_list: Vec<ConstDef>,
}

#[derive(Debug)]
pub struct ConstDef {
    pub ident: String,
    pub const_init_val: ConstInitVal,
}

#[derive(Debug)]
pub struct ConstInitVal {
    pub const_exp: ConstExp,
}

#[derive(Debug)]
pub struct ConstExp {
    pub lor_exp: LOrExp,
}

// endregion 常量声明

// region 变量声明

#[derive(Debug)]
pub struct VarDecl {
    pub b_type: String,
    pub var_def_list: Vec<VarDef>,
}

#[derive(Debug)]
pub struct VarDef {
    pub ident: String,
    pub init_val: Option<InitVal>, // 可以没有初始化值
}

#[derive(Debug)]
pub struct InitVal {
    pub exp: Exp,
}

// endregion 变量声明

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
    LVal(LVal), // LVal ::= IDENT,表示对一个标识符(常量名)的引用
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
