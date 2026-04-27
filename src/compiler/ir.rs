use crate::compiler::ast::Span;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    I32,
    F32,
    Bool,
    CharAscii,
    CharAwa,
    StrAscii,
    StrAwa,
    S32,
    U8,
    Bytes,
    Unit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypedProgram {
    pub items: Vec<TypedItem>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypedItem {
    ExternDecl(ExternSig),
    FnDecl(TypedFn),
    LetDecl(TypedLet),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternSig {
    pub local_name: String,
    pub symbol_name: String,
    pub params: Vec<ExternParam>,
    pub ret: Type,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternParam {
    pub name: String,
    pub ty: Type,
    pub explicit_tag: Option<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypedFn {
    pub name: String,
    pub params: Vec<TypedParam>,
    pub ret: Type,
    pub body: TypedBlock,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypedParam {
    pub name: String,
    pub ty: Type,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypedLet {
    pub name: String,
    pub ty: Option<Type>,
    pub value: TypedExpr,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypedBlock {
    pub stmts: Vec<TypedStmt>,
    pub tail_expr: Option<TypedExpr>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypedExpr {
    pub kind: TypedExprKind,
    pub ty: Type,
    pub span: Option<Span>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypedExprKind {
    Ident(String),
    Int(i64),
    Float(String),
    Bool(bool),
    Char(String),
    String(String),
    Call { callee: String, args: Vec<TypedExpr> },
    Pipe { value: Box<TypedExpr>, target: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypedStmt {
    Let(TypedLet),
    Assign { name: String, value: TypedExpr },
    Expr(TypedExpr),
    Return(Option<TypedExpr>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoreProgram {
    pub functions: Vec<CoreFunc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoreFunc {
    pub name: String,
    pub params: Vec<LocalId>,
    pub ret: Type,
    pub blocks: Vec<CoreBlock>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoreBlock {
    pub id: BlockId,
    pub stmts: Vec<CoreStmt>,
    pub term: CoreTerminator,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoreStmt {
    Let { dst: LocalId, value: CoreValue },
    Assign { dst: LocalId, value: CoreValue },
    ExternCall { dst: Option<LocalId>, call: CoreExternCall },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoreTerminator {
    Goto(BlockId),
    If { cond: LocalId, then_bb: BlockId, else_bb: BlockId },
    Return(Option<LocalId>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoreValue {
    pub kind: CoreValueKind,
    pub ty: Type,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoreValueKind {
    Local(LocalId),
    ConstInt(i64),
    ConstBool(bool),
    ConstString(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoreExternCall {
    pub sig: ExternSigId,
    pub args: Vec<LocalId>,
    pub decode: ReturnDecode,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReturnDecode {
    None,
    U8,
    I32,
    F32,
    Bytes,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LocalId(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlockId(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ExternSigId(pub usize);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AbiCall {
    pub symbol_name: String,
    pub args: Vec<AbiArg>,
    pub expected_return: AbiReturn,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AbiArg {
    pub tag: u8,
    pub payload: AbiPayload,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AbiPayload {
    Le4([u8; 4]),
    OneByte(u8),
    CString(Vec<u8>),
    SimpleI32Le4([u8; 4]),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AbiReturn {
    Unit,
    RawBytes,
    DecodeU8,
    DecodeI32Le,
    DecodeF32Le,
}
