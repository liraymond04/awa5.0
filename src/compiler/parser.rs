use crate::compiler::ast::*;
use crate::compiler::lexer::{Lexer, Token};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub message: String,
    pub position: Option<usize>,
}

impl ParseError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            position: None,
        }
    }
}

pub type ParseResult<T> = Result<T, ParseError>;

pub fn parse_program(source: &str) -> ParseResult<AstProgram> {
    let mut parser = Parser::from_source(source);
    let mut items = Vec::new();
    while !parser.is_eof() {
        if parser.peek() == &Token::DoubleSemicolon {
            parser.bump();
            continue;
        }
        // stop if only EOF remains
        if parser.peek() == &Token::EOF {
            break;
        }
        // parse next top-level item
        let item = parser.parse_item()?;
        items.push(item);
        // consume optional double semicolon separators
        if parser.peek() == &Token::DoubleSemicolon {
            parser.bump();
        }
    }
    Ok(AstProgram { items })
}

/// Simple dispatcher that recognizes a few top-level item forms and
/// delegates to the placeholder parsers added earlier.
pub fn parse_item(source: &str) -> ParseResult<AstItem> {
    let mut parser = Parser::from_source(source);
    parser.parse_item()
}

/// A small token-stream based parser used by the scaffold.
pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub fn from_source(source: &str) -> Self {
        Self { tokens: Lexer::new(source).tokenize_all(), pos: 0 }
    }

    fn is_eof(&self) -> bool {
        self.peek() == &Token::EOF
    }

    fn peek(&self) -> &Token {
        self.tokens.get(self.pos).unwrap_or(&Token::EOF)
    }

    fn peek_n(&self, n: usize) -> &Token {
        self.tokens.get(self.pos + n).unwrap_or(&Token::EOF)
    }

    fn bump(&mut self) -> Token {
        let t = self.peek().clone();
        if self.pos < self.tokens.len() { self.pos += 1; }
        t
    }

    fn error_here(&self, message: impl Into<String>) -> ParseError {
        ParseError { message: message.into(), position: Some(self.pos) }
    }

    fn expect_ident(&mut self) -> ParseResult<String> {
        match self.bump() {
            Token::Ident(s) => Ok(s),
            other => Err(ParseError { message: format!("expected identifier, got {:?}", other), position: Some(self.pos.saturating_sub(1)) }),
        }
    }

    fn expect_string(&mut self) -> ParseResult<String> {
        match self.bump() {
            Token::StringLit(s) => Ok(s),
            other => Err(ParseError { message: format!("expected string literal, got {:?}", other), position: Some(self.pos.saturating_sub(1)) }),
        }
    }

    fn parse_item(&mut self) -> ParseResult<AstItem> {
        match self.peek() {
            Token::Module if self.peek_n(1) == &Token::Type => self.parse_module_type_decl_from_tokens(),
            Token::Module => self.parse_module_decl_from_tokens(),
            Token::Include => {
                self.bump();
                let expr = self.parse_module_expr()?;
                Ok(AstItem::IncludeStmt(IncludeStmt { module_expr: expr }))
            }
            Token::Extern => self.parse_extern_decl_from_tokens(),
            Token::Let | Token::LetRec => self.parse_let_decl_from_tokens(),
            Token::Fn | Token::Func => self.parse_fn_decl_from_tokens(),
            Token::EOF => Err(self.error_here("unexpected EOF when parsing item")),
            _ => {
                // fallback: consume tokens into an ident-like module decl
                let name = match self.bump() {
                    Token::Ident(s) => s,
                    other => format!("__item_{:?}", other),
                };
                Ok(AstItem::ModuleDecl(ModuleDecl { name: name.clone(), expr: ModuleExpr::Ident(name) }))
            }
        }
    }

    fn parse_module_decl_from_tokens(&mut self) -> ParseResult<AstItem> {
        // consume 'module'
        let _ = self.bump();
        // next token should be name
        let name = self.expect_ident()?;
        // optionally expect '='
        if self.peek() == &Token::Eq {
            self.bump();
            let expr = self.parse_module_expr()?;
            Ok(AstItem::ModuleDecl(ModuleDecl { name, expr }))
        } else {
            Ok(AstItem::ModuleDecl(ModuleDecl { name: name.clone(), expr: ModuleExpr::Ident(name) }))
        }
    }

    fn parse_module_expr(&mut self) -> ParseResult<ModuleExpr> {
        match self.peek() {
            Token::Struct => {
                // consume 'struct'
                self.bump();
                // parse inner items until 'end'
                let mut items = Vec::new();
                while self.peek() != &Token::End && self.peek() != &Token::EOF {
                    let it = self.parse_item()?;
                    items.push(it);
                }
                // consume 'end' if present; otherwise report an unclosed struct body
                if self.peek() == &Token::End {
                    self.bump();
                } else {
                    return Err(self.error_here("expected `end` to close `struct`"));
                }
                Ok(ModuleExpr::Struct(items))
            }
            Token::Functor => {
                // simple functor parsing: functor (P : SIG) -> BODY
                self.bump();
                // skip optional '('
                if self.peek() == &Token::LParen { self.bump(); }
                let param_name = self.expect_ident()?;
                // skip ':' and sig
                if self.peek() == &Token::Colon { self.bump(); }
                let param_sig = match self.peek() {
                    Token::Ident(_) => ModuleSig::Ident(self.expect_ident()?),
                    _ => ModuleSig::Ident("__Psig".to_string()),
                };
                if self.peek() == &Token::RParen { self.bump(); }
                // skip arrow
                if self.peek() == &Token::Arrow { self.bump(); }
                let body = Box::new(ModuleExpr::Ident("__body".to_string()));
                Ok(ModuleExpr::Functor { param_name, param_sig, body })
            }
            Token::Ident(_) => {
                // identifier or apply
                if let Token::Ident(functor) = self.bump() {
                    if self.peek() == &Token::LParen {
                        // parse arg inside parens
                        self.bump();
                        let arg = if let Token::Ident(a) = self.bump() { a } else { "__arg".to_string() };
                        if self.peek() == &Token::RParen { self.bump(); }
                        Ok(ModuleExpr::Apply { functor: Box::new(ModuleExpr::Ident(functor)), arg: Box::new(ModuleExpr::Ident(arg)) })
                    } else {
                        Ok(ModuleExpr::Ident(functor))
                    }
                } else {
                    Err(ParseError::new("unexpected token while parsing module expr"))
                }
            }
            other => Err(ParseError { message: format!("unexpected token in module expr: {:?}", other), position: Some(self.pos) }),
        }
    }

    fn parse_module_type_decl_from_tokens(&mut self) -> ParseResult<AstItem> {
        // consume 'module' and 'type'
        self.bump(); // module
        if self.peek() == &Token::Type { self.bump(); }
        let name = self.expect_ident()?;
        if self.peek() == &Token::Eq { self.bump(); }
        let sig = if self.peek() == &Token::Sig {
            // consume sig ... end
            self.bump();
            // skip contents until 'end'
            while self.peek() != &Token::End && self.peek() != &Token::EOF { self.bump(); }
            if self.peek() == &Token::End {
                self.bump();
            } else {
                return Err(self.error_here("expected `end` to close `sig`"));
            }
            ModuleSig::Sig(Vec::new())
        } else {
            ModuleSig::Ident("__stub_sig".to_string())
        };
        Ok(AstItem::ModuleTypeDecl(ModuleTypeDecl { name, sig }))
    }

    fn parse_extern_decl_from_tokens(&mut self) -> ParseResult<AstItem> {
        self.bump(); // extern
        let name = if let Token::Ident(n) = self.bump() { n } else { "__extern".to_string() };
        // optional : TYPE
        let mut ty = TypeRef::Named("__extern_stub".to_string());
        if self.peek() == &Token::Colon {
            self.bump();
            ty = self.parse_type_signature()?;
        }
        // optional = "symbol"
        let mut symbol = name.clone();
        if self.peek() == &Token::Eq {
            self.bump();
            if let Token::StringLit(s) = self.bump() { symbol = s; }
        }
        Ok(AstItem::ExternDecl(ExternDecl { name, ty, symbol }))
    }

    fn parse_type_signature(&mut self) -> ParseResult<TypeRef> {
        // Parse a type signature like: int -> string -> unit
        // This builds up a TypeRef::Arrow with all the parts
        let mut parts = Vec::new();

        // Parse first type part
        if let Token::Ident(tn) = self.peek() {
            let tn_clone = tn.clone();
            self.bump();
            parts.push(parse_type_ref(&tn_clone)?);
        } else {
            return Err(ParseError {
                message: "expected type after :".to_string(),
                position: None,
            });
        }

        // Parse additional arrow-separated type parts
        while self.peek() == &Token::Arrow {
            self.bump(); // consume arrow
            if let Token::Ident(tn) = self.peek() {
                let tn_clone = tn.clone();
                self.bump();
                parts.push(parse_type_ref(&tn_clone)?);
            } else {
                return Err(ParseError {
                    message: "expected type after ->".to_string(),
                    position: None,
                });
            }
        }

        // If we only have one part, return it directly; otherwise wrap in Arrow
        if parts.len() == 1 {
            Ok(parts.into_iter().next().unwrap())
        } else {
            Ok(TypeRef::Arrow(parts))
        }
    }

    fn parse_let_decl_from_tokens(&mut self) -> ParseResult<AstItem> {
        let first = self.bump();
        let mut is_rec = false;
        let name = match first {
            Token::LetRec => { is_rec = true; self.expect_ident()? }
            Token::Let => {
                // maybe next is 'rec' (we also accept explicit LetRec token above)
                if self.peek() == &Token::LetRec { is_rec = true; self.bump(); }
                self.expect_ident()?
            }
            _ => "__let".to_string(),
        };
        // skip optional '=' and expression (stub: consume next token)
        if self.peek() == &Token::Eq { self.bump(); }
        // parse a full expression for the let value
        let value = self.parse_expr()?;
        Ok(AstItem::LetDecl(LetDecl { name, is_rec, params: Vec::new(), ty: None, value }))
    }

    fn parse_fn_decl_from_tokens(&mut self) -> ParseResult<AstItem> {
        self.bump(); // fn/func
        let name = if let Token::Ident(n) = self.bump() { n } else { "__fn".to_string() };

        let mut params = Vec::new();
        while !matches!(self.peek(), Token::Eq | Token::Arrow | Token::FatArrow | Token::DoubleSemicolon | Token::EOF) {
            params.push(self.parse_pattern()?);
            if self.peek() == &Token::Comma { self.bump(); }
        }

        if matches!(self.peek(), Token::Eq | Token::Arrow | Token::FatArrow) {
            self.bump();
        }

        let body_expr = if matches!(self.peek(), Token::DoubleSemicolon | Token::EOF) {
            Expr::Unit
        } else {
            self.parse_expr()?
        };
        Ok(AstItem::FnDecl(FnDecl {
            name,
            params,
            ret: TypeRef::Unit,
            body: Block { stmts: Vec::new(), tail: Some(body_expr) },
        }))
    }

    fn parse_expr(&mut self) -> ParseResult<Expr> {
        match self.peek() {
            Token::If => self.parse_if_expr(),
            Token::Fun => self.parse_fun_expr(),
            Token::Function => self.parse_function_expr(),
            Token::Match => self.parse_match_expr(),
            _ => self.parse_binary_expr(0),
        }
    }

    fn parse_if_expr(&mut self) -> ParseResult<Expr> {
        self.bump(); // if
        let cond = self.parse_expr()?;
        if self.peek() == &Token::Then { self.bump(); }
        let then_branch = self.parse_expr()?;
        if self.peek() == &Token::Else { self.bump(); }
        let else_branch = self.parse_expr()?;
        Ok(Expr::If {
            cond: Box::new(cond),
            then_branch: Box::new(then_branch),
            else_branch: Box::new(else_branch),
        })
    }

    fn parse_binary_expr(&mut self, min_prec: u8) -> ParseResult<Expr> {
        let mut lhs = self.parse_unary_expr()?;

        loop {
            let Some((op_prec, op_name, op_token_kind)) = self.binary_op_info(self.peek()) else {
                break;
            };

            if op_prec < min_prec {
                break;
            }

            self.bump();
            let rhs = self.parse_binary_expr(op_prec + 1)?;
            lhs = Self::make_binary_call(op_name, lhs, rhs, op_token_kind);
        }

        Ok(lhs)
    }

    fn parse_unary_expr(&mut self) -> ParseResult<Expr> {
        match self.peek() {
            Token::Minus => {
                self.bump();
                let expr = self.parse_unary_expr()?;
                Ok(Expr::Call { callee: "neg".to_string(), args: vec![expr] })
            }
            Token::Not => {
                self.bump();
                let expr = self.parse_unary_expr()?;
                Ok(Expr::Call { callee: "not".to_string(), args: vec![expr] })
            }
            _ => self.parse_postfix_expr(),
        }
    }

    fn parse_postfix_expr(&mut self) -> ParseResult<Expr> {
        let mut expr = self.parse_primary_expr()?;

        loop {
            match self.peek() {
                Token::LParen => {
                    self.bump();
                    let mut args = Vec::new();
                    while self.peek() != &Token::RParen && self.peek() != &Token::EOF {
                        args.push(self.parse_expr()?);
                        if matches!(self.peek(), Token::Comma | Token::Semicolon) { self.bump(); }
                    }
                    if self.peek() == &Token::RParen { self.bump(); }
                    expr = match expr {
                        Expr::Ident(name) => Expr::Call { callee: name, args },
                        other => Expr::Apply {
                            callee: Box::new(other),
                            args: args.into_iter().map(|value| Arg { label: ParamLabel::Plain, value }).collect(),
                        },
                    };
                }
                _ => break,
            }
        }

        Ok(expr)
    }

    fn parse_primary_expr(&mut self) -> ParseResult<Expr> {
        match self.bump() {
            Token::Number(s) => Ok(Expr::Int(s.parse().unwrap_or(0))),
            Token::StringLit(s) => Ok(Expr::String(s)),
            Token::LParen => {
                if self.peek() == &Token::RParen { self.bump(); return Ok(Expr::Unit); }
                let first = self.parse_expr()?;
                if self.peek() == &Token::Comma {
                    let mut items = vec![first];
                    while self.peek() == &Token::Comma {
                        self.bump();
                        items.push(self.parse_expr()?);
                    }
                    if self.peek() == &Token::RParen { self.bump(); }
                    Ok(Expr::Tuple(items))
                } else {
                    if self.peek() == &Token::RParen { self.bump(); }
                    Ok(first)
                }
            }
            Token::LBracket => {
                let mut items = Vec::new();
                while self.peek() != &Token::RBracket && self.peek() != &Token::EOF {
                    items.push(self.parse_expr()?);
                    if matches!(self.peek(), Token::Semicolon | Token::Comma) { self.bump(); }
                }
                if self.peek() == &Token::RBracket { self.bump(); }
                Ok(Expr::List(items))
            }
            Token::Ident(name) => {
                if name == "true" {
                    Ok(Expr::Bool(true))
                } else if name == "false" {
                    Ok(Expr::Bool(false))
                } else {
                    Ok(Expr::Ident(name))
                }
            }
            other => Err(ParseError { message: format!("unexpected token in expr: {:?}", other), position: Some(self.pos.saturating_sub(1)) }),
        }
    }

    fn parse_fun_expr(&mut self) -> ParseResult<Expr> {
        self.bump(); // fun
        let mut params = Vec::new();
        while !matches!(self.peek(), Token::Arrow | Token::FatArrow | Token::EOF) {
            params.push(self.parse_pattern()?);
        }
        if matches!(self.peek(), Token::Arrow | Token::FatArrow) { self.bump(); }
        let body = self.parse_expr()?;
        Ok(Expr::Fun { params, body: Box::new(body) })
    }

    fn parse_function_expr(&mut self) -> ParseResult<Expr> {
        self.bump(); // function
        let mut cases = Vec::new();
        while self.peek() != &Token::EOF {
            if self.peek() == &Token::Bar { self.bump(); }
            cases.push(self.parse_match_case()?);
            if self.peek() != &Token::Bar {
                break;
            }
        }
        Ok(Expr::Function { cases })
    }

    fn parse_match_expr(&mut self) -> ParseResult<Expr> {
        self.bump(); // match
        let value = self.parse_expr()?;
        if self.peek() == &Token::With { self.bump(); }
        let mut cases = Vec::new();
        while self.peek() != &Token::EOF {
            if self.peek() == &Token::Bar { self.bump(); }
            cases.push(self.parse_match_case()?);
            if self.peek() != &Token::Bar {
                break;
            }
        }
        Ok(Expr::Match { value: Box::new(value), cases })
    }

    fn parse_match_case(&mut self) -> ParseResult<MatchCase> {
        let pattern = self.parse_pattern()?;
        let guard = if self.peek() == &Token::When {
            self.bump();
            Some(self.parse_expr()?)
        } else {
            None
        };
        if matches!(self.peek(), Token::Arrow | Token::FatArrow) {
            self.bump();
        }
        let body = self.parse_expr()?;
        Ok(MatchCase { pattern, guard, body })
    }

    fn parse_pattern(&mut self) -> ParseResult<Pattern> {
        let mut pattern = match self.bump() {
            Token::Ident(name) => {
                if name == "_" {
                    Pattern::Wildcard
                } else if self.peek() == &Token::LParen {
                    // constructor with payload: Some(x)
                    self.bump();
                    let inner = if self.peek() == &Token::RParen {
                        None
                    } else {
                        Some(Box::new(self.parse_pattern()?))
                    };
                    if self.peek() == &Token::RParen { self.bump(); }
                    Pattern::Constructor { name, payload: inner }
                } else if is_uppercase_ident(&name) {
                    Pattern::Constructor { name, payload: None }
                } else {
                    Pattern::Ident(name)
                }
            }
            Token::Number(s) => Pattern::Int(s.parse().unwrap_or(0)),
            Token::LParen => {
                if self.peek() == &Token::RParen {
                    self.bump();
                    Pattern::Tuple(Vec::new())
                } else {
                    let mut parts = Vec::new();
                    parts.push(self.parse_pattern()?);
                    while self.peek() == &Token::Comma {
                        self.bump();
                        parts.push(self.parse_pattern()?);
                    }
                    if self.peek() == &Token::RParen { self.bump(); }
                    if parts.len() == 1 {
                        parts.into_iter().next().unwrap()
                    } else {
                        Pattern::Tuple(parts)
                    }
                }
            }
            Token::LBracket => {
                let mut items = Vec::new();
                while self.peek() != &Token::RBracket && self.peek() != &Token::EOF {
                    items.push(self.parse_pattern()?);
                    if self.peek() == &Token::Semicolon || self.peek() == &Token::Comma { self.bump(); }
                }
                if self.peek() == &Token::RBracket { self.bump(); }
                Pattern::List(items)
            }
            other => return Err(ParseError { message: format!("unexpected token in pattern: {:?}", other), position: Some(self.pos.saturating_sub(1)) }),
        };

        while self.peek() == &Token::As {
            self.bump();
            let alias = self.expect_ident()?;
            pattern = Pattern::As { pattern: Box::new(pattern), alias };
        }

        Ok(pattern)
    }

    fn binary_op_info(&self, token: &Token) -> Option<(u8, &'static str, &'static str)> {
        match token {
            Token::OrOr => Some((1, "||", "logical_or")),
            Token::AndAnd => Some((2, "&&", "logical_and")),
            Token::EqEq => Some((3, "==", "eq")),
            Token::NotEq => Some((3, "!=", "ne")),
            Token::Lt => Some((4, "<", "lt")),
            Token::LtEq => Some((4, "<=", "le")),
            Token::Gt => Some((4, ">", "gt")),
            Token::GtEq => Some((4, ">=", "ge")),
            Token::Plus => Some((5, "+", "add")),
            Token::Minus => Some((5, "-", "sub")),
            Token::Caret => Some((5, "^", "xor")),
            Token::Star => Some((6, "*", "mul")),
            Token::Slash => Some((6, "/", "div")),
            Token::Percent => Some((6, "%", "mod")),
            _ => None,
        }
    }

    fn make_binary_call(op: &'static str, lhs: Expr, rhs: Expr, _kind: &'static str) -> Expr {
        Expr::Call { callee: op.to_string(), args: vec![lhs, rhs] }
    }

}

pub fn parse_type_ref(name: &str) -> ParseResult<TypeRef> {
    Ok(match name {
        "int" | "i32" => TypeRef::Int,
        "float" | "f32" => TypeRef::Float,
        "bool" => TypeRef::Bool,
        "char" => TypeRef::Char,
        "string" | "cstr" => TypeRef::String,
        "awachar" | "achar" => TypeRef::AwaChar,
        "awastring" | "acstr" => TypeRef::AwaString,
        "bytes" => TypeRef::Bytes,
        "unit" => TypeRef::Unit,
        other => TypeRef::Named(other.to_string()),
    })
}

fn tokens_to_string(tokens: &[Token]) -> String {
    let mut out = String::new();
    for t in tokens {
        match t {
            Token::Ident(s) => { if !out.is_empty() { out.push(' '); } out.push_str(s); }
            Token::Number(n) => { if !out.is_empty() { out.push(' '); } out.push_str(n); }
            Token::StringLit(s) => { if !out.is_empty() { out.push(' '); } out.push('"'); out.push_str(s); out.push('"'); }
            Token::LParen => { if !out.is_empty() { out.push(' '); } out.push('('); }
            Token::RParen => { out.push(')'); }
            Token::LBrace => { if !out.is_empty() { out.push(' '); } out.push('{'); }
            Token::RBrace => { out.push('}'); }
            Token::LBracket => { if !out.is_empty() { out.push(' '); } out.push('['); }
            Token::RBracket => { out.push(']'); }
            Token::Eq => { if !out.is_empty() { out.push(' '); } out.push('='); }
            Token::Colon => { out.push(':'); }
            Token::Comma => { out.push(','); }
            Token::Dot => { out.push('.'); }
            Token::Semicolon => { out.push(';'); }
            Token::DoubleSemicolon => { out.push_str(";;"); }
            Token::Arrow => { if !out.is_empty() { out.push(' '); } out.push_str("->"); }
            Token::Module | Token::ModuleType | Token::Include | Token::Extern | Token::Let | Token::LetRec | Token::Fn | Token::Func | Token::Struct | Token::End | Token::Functor | Token::Sig | Token::Type | Token::If | Token::Then | Token::Else | Token::Match | Token::EOF => {}
            _ => {}
        }
    }
    out
}

/// Parse a module declaration placeholder.
/// Expected form (stub): `module NAME = <expr>`
pub fn parse_module_decl(source: &str) -> ParseResult<ModuleDecl> {
    let s = source.trim();
    if s.is_empty() {
        return Err(ParseError::new("parse_module_decl: empty source"));
    }

    // Expect form: NAME = EXPR  (both parts optional for stub)
    let parts: Vec<&str> = s.splitn(2, '=').collect();
    let name_part = parts[0].trim();
    let name = name_part.split_whitespace().next().unwrap_or("__module").to_string();

    let expr = if parts.len() > 1 {
        parse_module_expr(parts[1].trim())?
    } else {
        // No explicit expr given — treat as ident referring to a module
        ModuleExpr::Ident(name.clone())
    };

    Ok(ModuleDecl { name, expr })
}

/// Parse a module type (signature) declaration placeholder.
/// Expected form (stub): `module type NAME = sig ... end`
pub fn parse_module_type_decl(source: &str) -> ParseResult<ModuleTypeDecl> {
    let s = source.trim();
    if s.is_empty() {
        return Err(ParseError::new("parse_module_type_decl: empty source"));
    }

    // Expect form: NAME = SIG
    let parts: Vec<&str> = s.splitn(2, '=').collect();
    let name_part = parts[0].trim();
    let name = name_part.split_whitespace().next().unwrap_or("__mtype").to_string();
    let sig = if parts.len() > 1 {
        parse_module_sig(parts[1].trim())?
    } else {
        ModuleSig::Ident("__stub_sig".to_string())
    };

    Ok(ModuleTypeDecl { name, sig })
}

/// Parse an include statement placeholder.
/// Expected form (stub): `include MODULE_EXPR`
pub fn parse_include_stmt(source: &str) -> ParseResult<IncludeStmt> {
    let s = source.trim();
    if s.is_empty() {
        return Err(ParseError::new("parse_include_stmt: empty source"));
    }

    Ok(IncludeStmt {
        module_expr: ModuleExpr::Ident(s.to_string()),
    })
}

/// Parse a minimal module expression stub.
pub fn parse_module_expr(source: &str) -> ParseResult<ModuleExpr> {
    let s = source.trim();
    if s.is_empty() {
        return Err(ParseError::new("parse_module_expr: empty source"));
    }

    // struct ... end
    if s.starts_with("struct") {
        return Ok(ModuleExpr::Struct(Vec::new()));
    }

    // functor style stub (e.g. `functor (X : SIG) -> ...`)
    if s.starts_with("functor") || s.contains("->") {
        return Ok(ModuleExpr::Functor {
            param_name: "__P".to_string(),
            param_sig: ModuleSig::Ident("__Psig".to_string()),
            body: Box::new(ModuleExpr::Ident("__body".to_string())),
        });
    }

    // Apply form like Name(Arg)
    if let Some(idx) = s.find('(') {
        let functor = s[..idx].trim().to_string();
        if let Some(end) = s.rfind(')') {
            let arg = s[idx + 1..end].trim().to_string();
            return Ok(ModuleExpr::Apply {
                functor: Box::new(ModuleExpr::Ident(functor)),
                arg: Box::new(ModuleExpr::Ident(arg)),
            });
        }
    }

    // Fallback: identifier or first-class path
    Ok(ModuleExpr::Ident(s.to_string()))
}

/// Parse a minimal module signature stub.
pub fn parse_module_sig(source: &str) -> ParseResult<ModuleSig> {
    let s = source.trim();
    if s.is_empty() {
        return Err(ParseError::new("parse_module_sig: empty source"));
    }

    if s.starts_with("sig") {
        return Ok(ModuleSig::Sig(Vec::new()));
    }

    Ok(ModuleSig::Ident(s.to_string()))
}

fn is_uppercase_ident(name: &str) -> bool {
    name.chars().next().map(|c| c.is_ascii_uppercase()).unwrap_or(false)
}
