#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AstProgram {
    pub items: Vec<AstItem>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AstItem {
    ExternDecl(ExternDecl),
    FnDecl(FnDecl),
    LetDecl(LetDecl),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternDecl {
    pub name: String,
    pub params: Vec<Param>,
    pub ret: TypeRef,
    pub symbol: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FnDecl {
    pub name: String,
    pub params: Vec<Param>,
    pub ret: TypeRef,
    pub body: Block,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LetDecl {
    pub name: String,
    pub ty: Option<TypeRef>,
    pub value: Expr,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Param {
    pub name: String,
    pub ty: TypeRef,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    pub tail: Option<Expr>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Stmt {
    Let(LetDecl),
    Assign { name: String, value: Expr },
    Expr(Expr),
    Return(Option<Expr>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expr {
    Ident(String),
    Int(i64),
    Float(String),
    Bool(bool),
    Char(String),
    String(String),
    Call { callee: String, args: Vec<Expr> },
    Pipe { value: Box<Expr>, target: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeRef {
    I32,
    F32,
    Bool,
    Char,
    AChar,
    CStr,
    ACStr,
    S32,
    U8,
    Bytes,
    Unit,
    Named(String),
}
