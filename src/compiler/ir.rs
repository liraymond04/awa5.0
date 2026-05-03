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
    List(Box<Type>),
    Option(Box<Type>),
    Result(Box<Type>, Box<Type>),
    Tuple(Vec<Type>),
    Arrow(Vec<Type>),
    PolyVariant {
        rows: Vec<PolyVariantTypeRow>,
        open: bool,
    },
    Named(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolyVariantTypeRow {
    pub tag: String,
    pub payload: Option<Type>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypedProgram {
    pub items: Vec<TypedItem>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypedItem {
    TypeDecl(TypedTypeDecl),
    ExternDecl(ExternSig),
    ModuleDecl(TypedModuleDecl),
    ModuleTypeDecl(TypedModuleTypeDecl),
    Include(TypedIncludeStmt),
    FnDecl(TypedFn),
    LetDecl(TypedLet),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypedTypeDecl {
    pub name: String,
    pub type_params: Vec<String>,
    pub def: TypedTypeDef,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypedTypeDef {
    Alias(Type),
    Variant(Vec<TypedConstructorDecl>),
    Record(Vec<TypedRecordFieldDecl>),
    PolyVariant {
        rows: Vec<PolyVariantTypeRow>,
        open: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypedConstructorDecl {
    pub name: String,
    pub payload: Option<Type>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypedRecordFieldDecl {
    pub name: String,
    pub ty: Type,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypedModuleDecl {
    pub name: String,
    pub expr: TypedModuleExpr,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypedModuleTypeDecl {
    pub name: String,
    pub sig: TypedModuleSig,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypedIncludeStmt {
    pub module_expr: TypedModuleExpr,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypedModuleExpr {
    Ident(String),
    Struct(Vec<TypedItem>),
    Functor {
        param_name: String,
        param_sig: TypedModuleSig,
        body: Box<TypedModuleExpr>,
    },
    Apply {
        functor: Box<TypedModuleExpr>,
        arg: Box<TypedModuleExpr>,
    },
    FirstClass(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypedModuleSig {
    Ident(String),
    Sig(Vec<TypedSigItem>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypedSigItem {
    Val {
        name: String,
        ty: Type,
    },
    Type {
        name: String,
        type_params: Vec<String>,
        def: Option<TypedTypeDef>,
    },
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
    pub params: Vec<TypedPattern>,
    pub ret: Type,
    pub body: TypedBlock,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypedParam {
    pub label: ParamLabel,
    pub name: String,
    pub ty: Type,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParamLabel {
    Plain,
    Labeled(String),
    Optional(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypedLet {
    pub name: String,
    pub is_rec: bool,
    pub params: Vec<TypedPattern>,
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
    ModulePath(Vec<String>),
    Int(i64),
    Float(String),
    Bool(bool),
    Char(String),
    String(String),
    Unit,
    Tuple(Vec<TypedExpr>),
    List(Vec<TypedExpr>),
    LetIn {
        is_rec: bool,
        bindings: Vec<TypedLetBinding>,
        body: Box<TypedExpr>,
    },
    If {
        cond: Box<TypedExpr>,
        then_branch: Box<TypedExpr>,
        else_branch: Box<TypedExpr>,
    },
    Match {
        value: Box<TypedExpr>,
        cases: Vec<TypedMatchCase>,
    },
    Fun {
        params: Vec<TypedPattern>,
        body: Box<TypedExpr>,
    },
    Function {
        cases: Vec<TypedMatchCase>,
    },
    LocalOpen {
        module_name: String,
        body: Box<TypedExpr>,
    },
    Cons {
        head: Box<TypedExpr>,
        tail: Box<TypedExpr>,
    },
    RecordLiteral(Vec<TypedRecordFieldValue>),
    RecordUpdate {
        base: Box<TypedExpr>,
        fields: Vec<TypedRecordFieldValue>,
    },
    PolyVariant {
        tag: String,
        payload: Option<Box<TypedExpr>>,
    },
    Call { callee: String, args: Vec<TypedExpr> },
    Apply {
        callee: Box<TypedExpr>,
        args: Vec<TypedArg>,
    },
    Pipe { value: Box<TypedExpr>, target: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypedArg {
    pub label: ParamLabel,
    pub value: TypedExpr,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypedLetBinding {
    pub pattern: TypedPattern,
    pub value: TypedExpr,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypedMatchCase {
    pub pattern: TypedPattern,
    pub guard: Option<TypedExpr>,
    pub body: TypedExpr,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypedRecordFieldValue {
    pub name: String,
    pub value: TypedExpr,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypedPattern {
    Wildcard,
    Ident(String),
    Int(i64),
    Char(String),
    String(String),
    Constructor {
        name: String,
        payload: Option<Box<TypedPattern>>,
    },
    PolyVariant {
        tag: String,
        payload: Option<Box<TypedPattern>>,
    },
    List(Vec<TypedPattern>),
    Tuple(Vec<TypedPattern>),
    As {
        pattern: Box<TypedPattern>,
        alias: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypedStmt {
    Let(TypedLet),
    Include(TypedIncludeStmt),
    Assign { name: String, value: TypedExpr },
    Expr(TypedExpr),
    Return(Option<TypedExpr>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoreProgram {
    pub extern_sigs: Vec<ExternSig>,
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
    CallUser {
        dst: Option<LocalId>,
        callee: String,
        args: Vec<LocalId>,
    },
    Print { src: LocalId, newline: bool },
    ExternCall { dst: Option<LocalId>, call: CoreExternCall },
    OpBinary {
        dst: LocalId,
        op: CoreBinOp,
        lhs: LocalId,
        rhs: LocalId,
    },
    ModuleDecl { name: String },
    IncludeModule { name: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoreBinOp {
    Add,
    Sub,
    Mul,
    Div,
    Eql,
    Lss,
    Gr8,
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
    ModulePath(Vec<String>),
    ConstInt(i64),
    ConstFloat(String),
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
    DecodeU8,
    DecodeI32Le,
    DecodeF32Le,
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
