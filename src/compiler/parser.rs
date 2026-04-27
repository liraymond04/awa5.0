use crate::compiler::ast::*;

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
    let _ = source;
    Err(ParseError::new(
        "AwaML compiler scaffold only: parse_program() is not implemented yet",
    ))
}

pub fn parse_item(_source: &str) -> ParseResult<AstItem> {
    Err(ParseError::new(
        "AwaML compiler scaffold only: parse_item() is not implemented yet",
    ))
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
