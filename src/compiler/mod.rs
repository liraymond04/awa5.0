pub mod ast;
pub mod ir;
pub mod parser;

pub use ast::*;
pub use ir::*;
pub use parser::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompileError {
	Parse(ParseError),
	TypeCheck(&'static str),
	Lowering(&'static str),
}

impl From<ParseError> for CompileError {
	fn from(value: ParseError) -> Self {
		CompileError::Parse(value)
	}
}

pub fn compile_source(source: &str) -> Result<TypedProgram, CompileError> {
	let ast = parse_program(source)?;
	type_check_program(&ast)?;
	lower_program(&ast)
}

fn type_check_program(_program: &AstProgram) -> Result<(), CompileError> {
	Err(CompileError::TypeCheck(
		"AwaML compiler scaffold only: type checking is not implemented yet",
	))
}

fn lower_program(_program: &AstProgram) -> Result<TypedProgram, CompileError> {
	Err(CompileError::Lowering(
		"AwaML compiler scaffold only: lowering is not implemented yet",
	))
}
