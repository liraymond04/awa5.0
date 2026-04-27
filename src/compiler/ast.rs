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
    TypeDecl(TypeDecl),
    ExternDecl(ExternDecl),
    ModuleDecl(ModuleDecl),
    ModuleTypeDecl(ModuleTypeDecl),
    IncludeStmt(IncludeStmt),
    FnDecl(FnDecl),
    LetDecl(LetDecl),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeDecl {
    pub name: String,
    pub type_params: Vec<String>,
    pub def: TypeDef,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeDef {
    Alias(TypeRef),
    Variant(Vec<ConstructorDecl>),
    Record(Vec<RecordFieldDecl>),
    PolyVariant {
        rows: Vec<PolyVariantRowDecl>,
        open: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConstructorDecl {
    pub name: String,
    pub payload: Option<TypeRef>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordFieldDecl {
    pub name: String,
    pub ty: TypeRef,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolyVariantRowDecl {
    pub tag: String,
    pub payload: Option<TypeRef>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleDecl {
    pub name: String,
    pub expr: ModuleExpr,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleTypeDecl {
    pub name: String,
    pub sig: ModuleSig,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IncludeStmt {
    pub module_expr: ModuleExpr,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModuleExpr {
    Ident(String),
    Struct(Vec<AstItem>),
    Functor {
        param_name: String,
        param_sig: ModuleSig,
        body: Box<ModuleExpr>,
    },
    Apply {
        functor: Box<ModuleExpr>,
        arg: Box<ModuleExpr>,
    },
    FirstClass(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModuleSig {
    Ident(String),
    Sig(Vec<SigItem>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SigItem {
    Val {
        name: String,
        ty: TypeRef,
    },
    Type {
        name: String,
        type_params: Vec<String>,
        def: Option<TypeDef>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternDecl {
    pub name: String,
    pub ty: TypeRef,
    pub symbol: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FnDecl {
    pub name: String,
    pub params: Vec<Pattern>,
    pub ret: TypeRef,
    pub body: Block,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LetDecl {
    pub name: String,
    pub is_rec: bool,
    pub params: Vec<Pattern>,
    pub ty: Option<TypeRef>,
    pub value: Expr,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Param {
    pub label: ParamLabel,
    pub name: String,
    pub ty: TypeRef,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParamLabel {
    Plain,
    Labeled(String),
    Optional(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    pub tail: Option<Expr>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Stmt {
    Let(LetDecl),
    Include(IncludeStmt),
    Assign { name: String, value: Expr },
    Expr(Expr),
    Return(Option<Expr>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expr {
    Ident(String),
    ModulePath(Vec<String>),
    Int(i64),
    Float(String),
    Bool(bool),
    Char(String),
    String(String),
    Unit,
    Tuple(Vec<Expr>),
    List(Vec<Expr>),
    Call { callee: String, args: Vec<Expr> },
    Apply { callee: Box<Expr>, args: Vec<Arg> },
    LetIn {
        is_rec: bool,
        bindings: Vec<LetBinding>,
        body: Box<Expr>,
    },
    If {
        cond: Box<Expr>,
        then_branch: Box<Expr>,
        else_branch: Box<Expr>,
    },
    Match {
        value: Box<Expr>,
        cases: Vec<MatchCase>,
    },
    Fun {
        params: Vec<Pattern>,
        body: Box<Expr>,
    },
    Function {
        cases: Vec<MatchCase>,
    },
    LocalOpen {
        module_name: String,
        body: Box<Expr>,
    },
    Cons {
        head: Box<Expr>,
        tail: Box<Expr>,
    },
    RecordLiteral(Vec<RecordFieldValue>),
    RecordUpdate {
        base: Box<Expr>,
        fields: Vec<RecordFieldValue>,
    },
    PolyVariant {
        tag: String,
        payload: Option<Box<Expr>>,
    },
    Pipe { value: Box<Expr>, target: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Arg {
    pub label: ParamLabel,
    pub value: Expr,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LetBinding {
    pub pattern: Pattern,
    pub value: Expr,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchCase {
    pub pattern: Pattern,
    pub guard: Option<Expr>,
    pub body: Expr,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordFieldValue {
    pub name: String,
    pub value: Expr,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Pattern {
    Wildcard,
    Ident(String),
    Int(i64),
    Char(String),
    String(String),
    Constructor {
        name: String,
        payload: Option<Box<Pattern>>,
    },
    PolyVariant {
        tag: String,
        payload: Option<Box<Pattern>>,
    },
    List(Vec<Pattern>),
    Tuple(Vec<Pattern>),
    As {
        pattern: Box<Pattern>,
        alias: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeRef {
    Int,
    Float,
    Bool,
    Char,
    String,
    AwaChar,
    AwaString,
    Bytes,
    Unit,
    List(Box<TypeRef>),
    Option(Box<TypeRef>),
    Result(Box<TypeRef>, Box<TypeRef>),
    Tuple(Vec<TypeRef>),
    Arrow(Vec<TypeRef>),
    PolyVariant {
        rows: Vec<PolyVariantTypeRow>,
        open: bool,
    },
    Named(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolyVariantTypeRow {
    pub tag: String,
    pub payload: Option<TypeRef>,
}
