#[derive(Debug)]
pub struct CompUnit {
    pub items: Vec<CompUnitItem>,
}

#[derive(Debug)]
pub enum CompUnitItem {
    FuncDef(FuncDef),
    GlobalDecl(GlobalDecl), // 全局声明
}

#[derive(Debug)]
pub struct FuncDef {
    pub func_type: FuncType,
    pub id: String,
    pub params: Option<FuncFParams>,
    pub block: Block,
}

#[derive(Debug)]
pub enum FuncType {
    Int,
    Void,
}

#[derive(Debug)]
pub struct FuncFParams {
    pub params: Vec<FuncFParam>, // 形参列表
}

#[derive(Debug)]
pub struct FuncFParam {
    pub b_type: String,
    pub ident: String,
    pub dimensions: Vec<Option<ConstExp>>, // 数组参数的维度信息，第一维为None表示不定长
}

#[derive(Debug, Clone)]
pub struct FuncRParams {
    pub params: Vec<Exp>,   // 实参列表
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
    Return(Option<Exp>),
    Exp(Option<Exp>),
    Block(Block),
    Assign(LVal, Exp), // 赋值语句: LVal = Exp
    If(Exp, Box<Stmt>, Option<Box<Stmt>>), // if语句：条件，then分支，可选else分支
    While(Exp, Box<Stmt>), 
    Break,
    Continue,
}

// 全局声明（只能在编译单元级别出现）
#[derive(Debug)]
pub enum GlobalDecl {
    Const(ConstDecl),     // 全局常量
    Var(GlobalVarDecl),   // 全局变量
}

// 局部声明（只能在块内出现）
#[derive(Debug)]
pub enum Decl {
    Const(ConstDecl),
    Var(VarDecl),
}

#[derive(Debug, Clone)]
pub struct LVal {
    pub ident: String,
    pub indices: Vec<Exp>, // 数组索引表达式列表，空表示普通变量
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
    pub dimensions: Vec<ConstExp>, // 数组维度，空表示普通常量
    pub const_init_val: ConstInitVal,
}

#[derive(Debug)]
pub enum ConstInitVal {
    Exp(ConstExp),           // 单个常量表达式
    List(Vec<ConstInitVal>), // 数组初始化列表
}

#[derive(Debug)]
pub struct ConstExp {
    pub lor_exp: LOrExp,
}

// endregion 常量声明

// region 变量声明

// 局部变量声明
#[derive(Debug)]
pub struct VarDecl {
    pub b_type: String,
    pub var_def_list: Vec<VarDef>,
}

#[derive(Debug)]
pub struct VarDef {
    pub ident: String,
    pub dimensions: Vec<ConstExp>, // 数组维度，空表示普通变量
    pub init_val: Option<InitVal>, // 局部变量可以没有初始化值
}

// 全局变量声明
#[derive(Debug)]
pub struct GlobalVarDecl {
    pub b_type: String,
    pub var_def_list: Vec<GlobalVarDef>,
}

#[derive(Debug)]
pub struct GlobalVarDef {
    pub ident: String,
    pub dimensions: Vec<ConstExp>, // 数组维度，空表示普通变量
    pub init_val: Option<InitVal>, // 全局变量如果没有显式初始值，IR生成时会使用zeroinit
}

#[derive(Debug)]
pub enum InitVal {
    Exp(Exp),           // 单个表达式
    List(Vec<InitVal>), // 数组初始化列表
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
#[derive(Debug, Clone)]
pub enum Exp {
    LOr(Box<LOrExp>),
}

#[derive(Debug, Clone)]
pub enum RelExp {
    Add(Box<AddExp>),
    Rel(Box<RelExp>, RelOp, Box<AddExp>),
}

#[derive(Debug, Clone)]
pub enum EqExp {
    Rel(Box<RelExp>), 
    Eq(Box<EqExp>, EqOp, Box<RelExp>),
}

#[derive(Debug, Clone)]
pub enum LAndExp {
    Eq(Box<EqExp>),
    LAnd(Box<LAndExp>, Box<EqExp>), // 不需要op，因为只有&&
}

#[derive(Debug, Clone)]
pub enum LOrExp {
    LAnd(Box<LAndExp>),
    LOr(Box<LOrExp>, Box<LAndExp>), // 不需要op，因为||
}

#[derive(Debug, Clone)]
pub enum AddExp {
    Mul(Box<MulExp>),
    AddMul(Box<AddExp>, PlusSubOp, Box<MulExp>),
}

#[derive(Debug, Clone)]
pub enum MulExp {
    Unary(Box<UnaryExp>),
    MulDiv(Box<MulExp>, MulDivOp, Box<UnaryExp>),
}

#[derive(Debug, Clone)]
pub enum UnaryExp {
    Primary(PrimaryExp),
    Unary(UnaryOp, Box<UnaryExp>),
    FuncCall(String, Option<FuncRParams>), // 函数调用(函数名, 可选参数列表)
}

#[derive(Debug, Clone)]
pub enum PrimaryExp {
    Number(i32),
    Paren(Box<Exp>),
    LVal(LVal), // LVal ::= IDENT,表示对一个标识符(常量名)的引用
}

#[derive(Debug, Clone)]
pub enum UnaryOp {
    Plus,   // +
    Minus,  // -
    Not,    // !
}

#[derive(Debug, Clone)]
pub enum PlusSubOp {
    Plus,  // +
    Minus, // -
}

#[derive(Debug, Clone)]
pub enum MulDivOp {
    Mul, // *
    Div, // /
    Mod, // %
}

#[derive(Debug, Clone)]
pub enum RelOp {
    Lt,  // <
    Gt,  // >
    Le,  // <=
    Ge,  // >=
}

#[derive(Debug, Clone)]
pub enum EqOp {
    Eq,  // ==
    Ne,  // !=
}

// endregion 表达式
