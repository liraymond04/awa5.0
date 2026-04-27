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
        "i32" => TypeRef::I32,
        "f32" => TypeRef::F32,
        "bool" => TypeRef::Bool,
        "char" => TypeRef::Char,
        "achar" => TypeRef::AChar,
        "cstr" => TypeRef::CStr,
        "acstr" => TypeRef::ACStr,
        "s32" => TypeRef::S32,
        "u8" => TypeRef::U8,
        "bytes" => TypeRef::Bytes,
        "unit" => TypeRef::Unit,
        other => TypeRef::Named(other.to_string()),
    })
}
