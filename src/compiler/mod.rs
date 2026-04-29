pub mod ast;
pub mod ir;
pub mod parser;
pub mod lexer;

pub use ast::*;
pub use ir::*;
pub use parser::*;
pub use lexer::*;

use std::collections::HashMap;

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

pub fn compile_and_render(source: &str) -> Result<String, CompileError> {
	let program = compile_to_core(source)?;
	Ok(render_core_program(&program))
}

pub fn compile_to_core(source: &str) -> Result<CoreProgram, CompileError> {
	let ast = parse_program(source)?;
	type_check_program(&ast)?;
	lower_core_program(&ast)
}

fn type_check_program(_program: &AstProgram) -> Result<(), CompileError> {
	Ok(())
}

fn lower_program(program: &AstProgram) -> Result<TypedProgram, CompileError> {
	Ok(TypedProgram {
		items: program
			.items
			.iter()
			.map(lower_item)
			.collect::<Result<Vec<_>, _>>()?,
	})
}

fn lower_core_program(program: &AstProgram) -> Result<CoreProgram, CompileError> {
	let mut functions = Vec::new();
	for item in &program.items {
		match item {
			AstItem::FnDecl(fd) => functions.push(lower_fn_decl_to_core(fd)?),
			AstItem::LetDecl(ld) => functions.push(lower_let_decl_to_core(ld)?),
			AstItem::ModuleDecl(md) => functions.push(lower_module_decl_to_core(md)?),
			AstItem::ModuleTypeDecl(mtd) => functions.push(lower_module_type_decl_to_core(mtd)?),
			AstItem::IncludeStmt(inc) => functions.push(lower_include_stmt_to_core(inc)?),
			AstItem::ExternDecl(ed) => functions.push(lower_extern_decl_to_core(ed)?),
			AstItem::TypeDecl(td) => functions.push(lower_type_decl_to_core(td)?),
		}
	}

	Ok(CoreProgram { functions })
}

fn lower_fn_decl_to_core(fd: &FnDecl) -> Result<CoreFunc, CompileError> {
	let mut param_env: HashMap<String, LocalId> = HashMap::new();
	for (idx, pattern) in fd.params.iter().enumerate() {
		if let Pattern::Ident(name) = pattern {
			param_env.insert(name.clone(), LocalId(idx));
		}
	}

	let mut block = CoreBlock {
		id: BlockId(0),
		stmts: Vec::new(),
		term: CoreTerminator::Return(None),
	};
	if let Some(expr) = &fd.body.tail {
		match expr {
			Expr::Ident(name) => {
				if let Some(local) = param_env.get(name).copied() {
					block.term = CoreTerminator::Return(Some(local));
				} else {
					let local = LocalId(0);
					block.stmts.push(CoreStmt::Let { dst: local, value: lower_core_value(expr, &param_env)? });
					block.term = CoreTerminator::Return(Some(local));
				}
			}
			_ => {
				let local = LocalId(0);
				block.stmts.push(CoreStmt::Let { dst: local, value: lower_core_value(expr, &param_env)? });
				block.term = CoreTerminator::Return(Some(local));
			}
		}
	}
	Ok(CoreFunc {
		name: fd.name.clone(),
		params: (0..fd.params.len()).map(LocalId).collect(),
		ret: Type::Unit,
		blocks: vec![block],
	})
}

fn lower_let_decl_to_core(ld: &LetDecl) -> Result<CoreFunc, CompileError> {
	let local = LocalId(0);
	let value = lower_core_value(&ld.value, &HashMap::new())?;
	Ok(CoreFunc {
		name: format!("let_{}", ld.name),
		params: Vec::new(),
		ret: Type::Unit,
		blocks: vec![CoreBlock {
			id: BlockId(0),
			stmts: vec![CoreStmt::Let { dst: local, value }],
			term: CoreTerminator::Return(Some(local)),
		}],
	})
}

fn lower_module_decl_to_core(md: &ModuleDecl) -> Result<CoreFunc, CompileError> {
	Ok(CoreFunc {
		name: format!("module_{}", md.name),
		params: Vec::new(),
		ret: Type::Unit,
		blocks: vec![CoreBlock {
			id: BlockId(0),
			stmts: vec![CoreStmt::ModuleDecl { name: md.name.clone() }],
			term: CoreTerminator::Return(None),
		}],
	})
}

fn lower_module_type_decl_to_core(mtd: &ModuleTypeDecl) -> Result<CoreFunc, CompileError> {
	Ok(CoreFunc {
		name: format!("module_type_{}", mtd.name),
		params: Vec::new(),
		ret: Type::Unit,
		blocks: vec![CoreBlock {
			id: BlockId(0),
			stmts: Vec::new(),
			term: CoreTerminator::Return(None),
		}],
	})
}

fn lower_include_stmt_to_core(inc: &IncludeStmt) -> Result<CoreFunc, CompileError> {
	let name = match &inc.module_expr {
		ModuleExpr::Ident(n) => n.clone(),
		_ => "included".to_string(),
	};
	Ok(CoreFunc {
		name: format!("include_{}", name),
		params: Vec::new(),
		ret: Type::Unit,
		blocks: vec![CoreBlock {
			id: BlockId(0),
			stmts: vec![CoreStmt::IncludeModule { name }],
			term: CoreTerminator::Return(None),
		}],
	})
}

fn lower_extern_decl_to_core(ed: &ExternDecl) -> Result<CoreFunc, CompileError> {
	Ok(CoreFunc {
		name: format!("extern_{}", ed.name),
		params: Vec::new(),
		ret: Type::Unit,
		blocks: vec![CoreBlock {
			id: BlockId(0),
			stmts: Vec::new(),
			term: CoreTerminator::Return(None),
		}],
	})
}

fn lower_type_decl_to_core(td: &TypeDecl) -> Result<CoreFunc, CompileError> {
	Ok(CoreFunc {
		name: format!("type_{}", td.name),
		params: Vec::new(),
		ret: Type::Unit,
		blocks: vec![CoreBlock { id: BlockId(0), stmts: Vec::new(), term: CoreTerminator::Return(None) }],
	})
}

fn lower_core_value(expr: &Expr, env: &HashMap<String, LocalId>) -> Result<CoreValue, CompileError> {
	Ok(match expr {
		Expr::Int(v) => CoreValue { kind: CoreValueKind::ConstInt(*v), ty: Type::I32 },
		Expr::Bool(v) => CoreValue { kind: CoreValueKind::ConstBool(*v), ty: Type::Bool },
		Expr::String(v) => CoreValue { kind: CoreValueKind::ConstString(v.clone()), ty: Type::StrAscii },
		Expr::Ident(name) => CoreValue {
			kind: env.get(name).copied().map(CoreValueKind::Local).unwrap_or_else(|| CoreValueKind::Local(LocalId(hash_name(name)))),
			ty: Type::Named("unknown".to_string()),
		},
		Expr::ModulePath(path) => CoreValue { kind: CoreValueKind::ModulePath(path.clone()), ty: Type::Named("unknown".to_string()) },
		other => CoreValue { kind: CoreValueKind::ConstString(format!("<{other:?}>") ), ty: Type::Named("unknown".to_string()) },
	})
}

fn hash_name(name: &str) -> usize {
	name.bytes().fold(0usize, |acc, b| acc.wrapping_mul(31).wrapping_add(b as usize))
}

fn lower_item(item: &AstItem) -> Result<TypedItem, CompileError> {
	Ok(match item {
		AstItem::TypeDecl(td) => TypedItem::TypeDecl(TypedTypeDecl {
			name: td.name.clone(),
			type_params: td.type_params.clone(),
			def: lower_type_def(&td.def)?,
		}),
		AstItem::ExternDecl(ed) => TypedItem::ExternDecl(lower_extern_decl(ed)?),
		AstItem::ModuleDecl(md) => TypedItem::ModuleDecl(TypedModuleDecl {
			name: md.name.clone(),
			expr: lower_module_expr(&md.expr)?,
		}),
		AstItem::ModuleTypeDecl(mtd) => TypedItem::ModuleTypeDecl(TypedModuleTypeDecl {
			name: mtd.name.clone(),
			sig: lower_module_sig(&mtd.sig)?,
		}),
		AstItem::IncludeStmt(inc) => TypedItem::Include(TypedIncludeStmt {
			module_expr: lower_module_expr(&inc.module_expr)?,
		}),
		AstItem::FnDecl(fd) => TypedItem::FnDecl(TypedFn {
			name: fd.name.clone(),
			params: fd.params.iter().map(lower_pattern).collect(),
			ret: Type::Unit,
			body: lower_block(&fd.body)?,
		}),
		AstItem::LetDecl(ld) => TypedItem::LetDecl(TypedLet {
			name: ld.name.clone(),
			is_rec: ld.is_rec,
			params: ld.params.iter().map(lower_pattern).collect(),
			ty: ld.ty.as_ref().map(lower_type_ref),
			value: lower_expr(&ld.value)?,
		}),
	})
}

fn lower_extern_decl(ed: &ExternDecl) -> Result<ExternSig, CompileError> {
	let (params, ret) = lower_extern_type(&ed.ty)?;
	Ok(ExternSig {
		local_name: ed.name.clone(),
		symbol_name: ed.symbol.clone(),
		params,
		ret,
	})
}

fn lower_extern_type(ty: &TypeRef) -> Result<(Vec<ExternParam>, Type), CompileError> {
	match ty {
		TypeRef::Arrow(parts) if !parts.is_empty() => {
			let mut params = Vec::new();
			for (idx, part) in parts.iter().enumerate() {
				let lowered = lower_type_ref(part);
				if idx + 1 == parts.len() {
					return Ok((params, lowered));
				}
				params.push(ExternParam { name: format!("arg{idx}"), ty: lowered, explicit_tag: None });
			}
			Ok((params, Type::Unit))
		}
		other => Ok((Vec::new(), lower_type_ref(other))),
	}
}

fn lower_type_ref(ty: &TypeRef) -> Type {
	match ty {
		TypeRef::Int => Type::I32,
		TypeRef::Float => Type::F32,
		TypeRef::Bool => Type::Bool,
		TypeRef::Char | TypeRef::AwaChar => Type::CharAwa,
		TypeRef::String => Type::StrAscii,
		TypeRef::AwaString => Type::StrAwa,
		TypeRef::Bytes => Type::Bytes,
		TypeRef::Unit => Type::Unit,
		TypeRef::List(inner) => Type::List(Box::new(lower_type_ref(inner))),
		TypeRef::Option(inner) => Type::Option(Box::new(lower_type_ref(inner))),
		TypeRef::Result(ok, err) => Type::Result(Box::new(lower_type_ref(ok)), Box::new(lower_type_ref(err))),
		TypeRef::Tuple(items) => Type::Tuple(items.iter().map(lower_type_ref).collect()),
		TypeRef::Arrow(items) => Type::Arrow(items.iter().map(lower_type_ref).collect()),
		TypeRef::PolyVariant { rows, open } => Type::PolyVariant {
			rows: rows
				.iter()
				.map(|row| ir::PolyVariantTypeRow { tag: row.tag.clone(), payload: row.payload.as_ref().map(lower_type_ref) })
				.collect(),
			open: *open,
		},
		TypeRef::Named(name) => Type::Named(name.clone()),
	}
}

fn lower_type_def(def: &TypeDef) -> Result<TypedTypeDef, CompileError> {
	Ok(match def {
		TypeDef::Alias(ty) => TypedTypeDef::Alias(lower_type_ref(ty)),
		TypeDef::Variant(ctors) => TypedTypeDef::Variant(ctors.iter().map(|ctor| TypedConstructorDecl {
			name: ctor.name.clone(),
			payload: ctor.payload.as_ref().map(lower_type_ref),
		}).collect()),
		TypeDef::Record(fields) => TypedTypeDef::Record(fields.iter().map(|field| TypedRecordFieldDecl {
			name: field.name.clone(),
			ty: lower_type_ref(&field.ty),
		}).collect()),
		TypeDef::PolyVariant { rows, open } => TypedTypeDef::PolyVariant {
			rows: rows
				.iter()
				.map(|row| ir::PolyVariantTypeRow { tag: row.tag.clone(), payload: row.payload.as_ref().map(lower_type_ref) })
				.collect(),
			open: *open,
		},
	})
}

fn lower_module_expr(expr: &ModuleExpr) -> Result<TypedModuleExpr, CompileError> {
	Ok(match expr {
		ModuleExpr::Ident(name) => TypedModuleExpr::Ident(name.clone()),
		ModuleExpr::Struct(items) => TypedModuleExpr::Struct(items.iter().map(lower_item).collect::<Result<Vec<_>, _>>()?),
		ModuleExpr::Functor { param_name, param_sig, body } => TypedModuleExpr::Functor {
			param_name: param_name.clone(),
			param_sig: lower_module_sig(param_sig)?,
			body: Box::new(lower_module_expr(body)?),
		},
		ModuleExpr::Apply { functor, arg } => TypedModuleExpr::Apply {
			functor: Box::new(lower_module_expr(functor)?),
			arg: Box::new(lower_module_expr(arg)?),
		},
		ModuleExpr::FirstClass(name) => TypedModuleExpr::FirstClass(name.clone()),
	})
}

fn lower_module_sig(sig: &ModuleSig) -> Result<TypedModuleSig, CompileError> {
	Ok(match sig {
		ModuleSig::Ident(name) => TypedModuleSig::Ident(name.clone()),
		ModuleSig::Sig(items) => TypedModuleSig::Sig(items.iter().map(lower_sig_item).collect::<Result<Vec<_>, _>>()?),
	})
}

fn lower_sig_item(item: &SigItem) -> Result<TypedSigItem, CompileError> {
	Ok(match item {
		SigItem::Val { name, ty } => TypedSigItem::Val { name: name.clone(), ty: lower_type_ref(ty) },
		SigItem::Type { name, type_params, def } => TypedSigItem::Type {
			name: name.clone(),
			type_params: type_params.clone(),
			def: def.as_ref().map(lower_type_def).transpose()?,
		},
	})
}

fn lower_block(block: &Block) -> Result<TypedBlock, CompileError> {
	Ok(TypedBlock {
		stmts: block.stmts.iter().map(lower_stmt).collect::<Result<Vec<_>, _>>()?,
		tail_expr: block.tail.as_ref().map(lower_expr).transpose()?,
	})
}

fn lower_stmt(stmt: &Stmt) -> Result<TypedStmt, CompileError> {
	Ok(match stmt {
		Stmt::Let(ld) => TypedStmt::Let(TypedLet {
			name: ld.name.clone(),
			is_rec: ld.is_rec,
			params: ld.params.iter().map(lower_pattern).collect(),
			ty: ld.ty.as_ref().map(lower_type_ref),
			value: lower_expr(&ld.value)?,
		}),
		Stmt::Include(inc) => TypedStmt::Include(TypedIncludeStmt { module_expr: lower_module_expr(&inc.module_expr)? }),
		Stmt::Assign { name, value } => TypedStmt::Assign { name: name.clone(), value: lower_expr(value)? },
		Stmt::Expr(expr) => TypedStmt::Expr(lower_expr(expr)?),
		Stmt::Return(expr) => TypedStmt::Return(expr.as_ref().map(lower_expr).transpose()?),
	})
}

fn lower_expr(expr: &Expr) -> Result<TypedExpr, CompileError> {
	let (kind, ty) = match expr {
		Expr::Ident(name) => (TypedExprKind::Ident(name.clone()), Type::Named("unknown".to_string())),
		Expr::ModulePath(path) => (TypedExprKind::ModulePath(path.clone()), Type::Named("unknown".to_string())),
		Expr::Int(v) => (TypedExprKind::Int(*v), Type::I32),
		Expr::Float(v) => (TypedExprKind::Float(v.clone()), Type::F32),
		Expr::Bool(v) => (TypedExprKind::Bool(*v), Type::Bool),
		Expr::Char(v) => (TypedExprKind::Char(v.clone()), Type::CharAscii),
		Expr::String(v) => (TypedExprKind::String(v.clone()), Type::StrAscii),
		Expr::Unit => (TypedExprKind::Unit, Type::Unit),
		Expr::Tuple(items) => (
			TypedExprKind::Tuple(items.iter().map(lower_expr).collect::<Result<Vec<_>, _>>()?),
			Type::Tuple(Vec::new()),
		),
		Expr::List(items) => (
			TypedExprKind::List(items.iter().map(lower_expr).collect::<Result<Vec<_>, _>>()?),
			Type::List(Box::new(Type::Named("unknown".to_string()))),
		),
		Expr::Call { callee, args } => (
			TypedExprKind::Call { callee: callee.clone(), args: args.iter().map(lower_expr).collect::<Result<Vec<_>, _>>()? },
			Type::Named("unknown".to_string()),
		),
		Expr::Apply { callee, args } => (
			TypedExprKind::Apply {
				callee: Box::new(lower_expr(callee)?),
				args: args
					.iter()
					.map(|arg| Ok(TypedArg { label: lower_param_label(&arg.label), value: lower_expr(&arg.value)? }))
					.collect::<Result<Vec<_>, CompileError>>()?,
			},
			Type::Named("unknown".to_string()),
		),
		Expr::LetIn { is_rec, bindings, body } => (
			TypedExprKind::LetIn {
				is_rec: *is_rec,
				bindings: bindings
					.iter()
					.map(|binding| Ok(TypedLetBinding { pattern: lower_pattern(&binding.pattern), value: lower_expr(&binding.value)? }))
					.collect::<Result<Vec<_>, CompileError>>()?,
				body: Box::new(lower_expr(body)?),
			},
			Type::Named("unknown".to_string()),
		),
		Expr::If { cond, then_branch, else_branch } => (
			TypedExprKind::If {
				cond: Box::new(lower_expr(cond)?),
				then_branch: Box::new(lower_expr(then_branch)?),
				else_branch: Box::new(lower_expr(else_branch)?),
			},
			Type::Named("unknown".to_string()),
		),
		Expr::Match { value, cases } => (
			TypedExprKind::Match {
				value: Box::new(lower_expr(value)?),
				cases: cases
					.iter()
					.map(|case| Ok(TypedMatchCase {
						pattern: lower_pattern(&case.pattern),
						guard: case.guard.as_ref().map(lower_expr).transpose()?,
						body: lower_expr(&case.body)?,
					}))
					.collect::<Result<Vec<_>, CompileError>>()?,
			},
			Type::Named("unknown".to_string()),
		),
		Expr::Fun { params, body } => (
			TypedExprKind::Fun { params: params.iter().map(lower_pattern).collect(), body: Box::new(lower_expr(body)? ) },
			Type::Arrow(Vec::new()),
		),
		Expr::Function { cases } => (
			TypedExprKind::Function {
				cases: cases
					.iter()
					.map(|case| Ok(TypedMatchCase {
						pattern: lower_pattern(&case.pattern),
						guard: case.guard.as_ref().map(lower_expr).transpose()?,
						body: lower_expr(&case.body)?,
					}))
					.collect::<Result<Vec<_>, CompileError>>()?,
			},
			Type::Arrow(Vec::new()),
		),
		Expr::LocalOpen { module_name, body } => (
			TypedExprKind::LocalOpen { module_name: module_name.clone(), body: Box::new(lower_expr(body)? ) },
			Type::Named("unknown".to_string()),
		),
		Expr::Cons { head, tail } => (
			TypedExprKind::Cons { head: Box::new(lower_expr(head)?), tail: Box::new(lower_expr(tail)? ) },
			Type::List(Box::new(Type::Named("unknown".to_string()))),
		),
		Expr::RecordLiteral(fields) => (
			TypedExprKind::RecordLiteral(fields.iter().map(|field| Ok(TypedRecordFieldValue { name: field.name.clone(), value: lower_expr(&field.value)? })).collect::<Result<Vec<_>, CompileError>>()?),
			Type::Named("unknown".to_string()),
		),
		Expr::RecordUpdate { base, fields } => (
			TypedExprKind::RecordUpdate {
				base: Box::new(lower_expr(base)?),
				fields: fields.iter().map(|field| Ok(TypedRecordFieldValue { name: field.name.clone(), value: lower_expr(&field.value)? })).collect::<Result<Vec<_>, CompileError>>()?,
			},
			Type::Named("unknown".to_string()),
		),
		Expr::PolyVariant { tag, payload } => (
			TypedExprKind::PolyVariant { tag: tag.clone(), payload: payload.as_ref().map(|expr| lower_expr(expr)).transpose()?.map(Box::new) },
			Type::PolyVariant { rows: vec![], open: false },
		),
		Expr::Pipe { value, target } => (
			TypedExprKind::Pipe { value: Box::new(lower_expr(value)?), target: target.clone() },
			Type::Named("unknown".to_string()),
		),
	};

	Ok(TypedExpr { kind, ty, span: None })
}

fn lower_pattern(pattern: &Pattern) -> TypedPattern {
	match pattern {
		Pattern::Wildcard => TypedPattern::Wildcard,
		Pattern::Ident(name) => TypedPattern::Ident(name.clone()),
		Pattern::Int(v) => TypedPattern::Int(*v),
		Pattern::Char(v) => TypedPattern::Char(v.clone()),
		Pattern::String(v) => TypedPattern::String(v.clone()),
		Pattern::Constructor { name, payload } => TypedPattern::Constructor {
			name: name.clone(),
			payload: payload.as_ref().map(|p| Box::new(lower_pattern(p))),
		},
		Pattern::PolyVariant { tag, payload } => TypedPattern::PolyVariant {
			tag: tag.clone(),
			payload: payload.as_ref().map(|p| Box::new(lower_pattern(p))),
		},
		Pattern::List(items) => TypedPattern::List(items.iter().map(lower_pattern).collect()),
		Pattern::Tuple(items) => TypedPattern::Tuple(items.iter().map(lower_pattern).collect()),
		Pattern::As { pattern, alias } => TypedPattern::As {
			pattern: Box::new(lower_pattern(pattern)),
			alias: alias.clone(),
		},
	}
}

fn lower_param_label(label: &ast::ParamLabel) -> ir::ParamLabel {
	match label {
		ast::ParamLabel::Plain => ir::ParamLabel::Plain,
		ast::ParamLabel::Labeled(name) => ir::ParamLabel::Labeled(name.clone()),
		ast::ParamLabel::Optional(name) => ir::ParamLabel::Optional(name.clone()),
	}
}

pub fn render_core_program(program: &CoreProgram) -> String {
	let mut out = String::new();
	out.push_str("CoreProgram\n");
	for func in &program.functions {
		render_core_func(func, 1, &mut out);
	}
	out
}

fn render_core_func(func: &CoreFunc, indent: usize, out: &mut String) {
	let pad = "  ".repeat(indent);
	out.push_str(&format!("{pad}Func {}({}) -> {}\n", func.name, func.params.iter().map(|id| format!("%{}", id.0)).collect::<Vec<_>>().join(", "), render_type(&func.ret)));
	for block in &func.blocks {
		render_core_block(block, indent + 1, out);
	}
}

fn render_core_block(block: &CoreBlock, indent: usize, out: &mut String) {
	let pad = "  ".repeat(indent);
	out.push_str(&format!("{pad}Block {}\n", block.id.0));
	for stmt in &block.stmts {
		out.push_str(&format!("{pad}  {}\n", render_core_stmt(stmt)));
	}
	out.push_str(&format!("{pad}  {}\n", render_core_terminator(&block.term)));
}

fn render_core_stmt(stmt: &CoreStmt) -> String {
	match stmt {
		CoreStmt::Let { dst, value } => format!("let %{} = {} : {}", dst.0, render_core_value_kind(&value.kind), render_type(&value.ty)),
		CoreStmt::Assign { dst, value } => format!("%{} = {} : {}", dst.0, render_core_value_kind(&value.kind), render_type(&value.ty)),
		CoreStmt::ExternCall { dst, call } => format!("{}extern_call sig={} args=[{}] decode={:?}", dst.map(|d| format!("%{} = ", d.0)).unwrap_or_default(), call.sig.0, call.args.iter().map(|a| format!("%{}", a.0)).collect::<Vec<_>>().join(", "), call.decode),
		CoreStmt::ModuleDecl { name } => format!("module {}", name),
		CoreStmt::IncludeModule { name } => format!("include {}", name),
	}
}

fn render_core_terminator(term: &CoreTerminator) -> String {
	match term {
		CoreTerminator::Goto(bb) => format!("goto bb{}", bb.0),
		CoreTerminator::If { cond, then_bb, else_bb } => format!("if %{} then bb{} else bb{}", cond.0, then_bb.0, else_bb.0),
		CoreTerminator::Return(Some(id)) => format!("return %{}", id.0),
		CoreTerminator::Return(None) => "return".to_string(),
	}
}

fn render_core_value_kind(kind: &CoreValueKind) -> String {
	match kind {
		CoreValueKind::Local(id) => format!("%{}", id.0),
		CoreValueKind::ModulePath(path) => path.join("."),
		CoreValueKind::ConstInt(v) => v.to_string(),
		CoreValueKind::ConstBool(v) => v.to_string(),
		CoreValueKind::ConstString(v) => format!("\"{}\"", v),
	}
}

pub fn render_typed_program(program: &TypedProgram) -> String {
	let mut out = String::new();
	out.push_str("TypedProgram\n");
	for item in &program.items {
		render_typed_item(item, 1, &mut out);
	}
	out
}

fn render_typed_item(item: &TypedItem, indent: usize, out: &mut String) {
	let pad = "  ".repeat(indent);
	match item {
		TypedItem::TypeDecl(td) => {
			out.push_str(&format!("{pad}TypeDecl {} = {}\n", td.name, render_typed_type_def(&td.def)));
		}
		TypedItem::ExternDecl(sig) => {
			out.push_str(&format!("{pad}Extern {} -> {}\n", sig.local_name, render_type(&sig.ret)));
		}
		TypedItem::ModuleDecl(md) => {
			out.push_str(&format!("{pad}Module {} = {}\n", md.name, render_typed_module_expr(&md.expr, indent + 1)));
		}
		TypedItem::ModuleTypeDecl(mtd) => {
			out.push_str(&format!("{pad}ModuleType {} = {}\n", mtd.name, render_typed_module_sig(&mtd.sig, indent + 1)));
		}
		TypedItem::Include(inc) => {
			out.push_str(&format!("{pad}Include {}\n", render_typed_module_expr(&inc.module_expr, indent + 1)));
		}
		TypedItem::FnDecl(fd) => {
			out.push_str(&format!("{pad}Fn {}({}) -> {}\n", fd.name, fd.params.iter().map(render_typed_pattern).collect::<Vec<_>>().join(", "), render_type(&fd.ret)));
			render_typed_block(&fd.body, indent + 1, out);
		}
		TypedItem::LetDecl(ld) => {
			out.push_str(&format!("{pad}Let {}{} = {}\n", if ld.is_rec { "rec " } else { "" }, ld.name, render_typed_expr(&ld.value, indent + 1)));
		}
	}
}

fn render_typed_module_expr(expr: &TypedModuleExpr, indent: usize) -> String {
	match expr {
		TypedModuleExpr::Ident(name) => name.clone(),
		TypedModuleExpr::FirstClass(name) => format!("<module {}>", name),
		TypedModuleExpr::Apply { functor, arg } => format!("apply({}, {})", render_typed_module_expr(functor, indent), render_typed_module_expr(arg, indent)),
		TypedModuleExpr::Functor { param_name, param_sig, body } => format!("functor({}: {}) -> {}", param_name, render_typed_module_sig(param_sig, indent + 1), render_typed_module_expr(body, indent + 1)),
		TypedModuleExpr::Struct(items) => {
			let mut out = String::from("struct\n");
			for item in items { render_typed_item(item, indent + 1, &mut out); }
			out.push_str(&format!("{}end", "  ".repeat(indent)));
			out
		}
	}
}

fn render_typed_module_sig(sig: &TypedModuleSig, indent: usize) -> String {
	match sig {
		TypedModuleSig::Ident(name) => name.clone(),
		TypedModuleSig::Sig(items) => {
			let mut out = String::from("sig\n");
			for item in items { out.push_str(&format!("{}{}\n", "  ".repeat(indent), render_typed_sig_item(item))); }
			out.push_str(&format!("{}end", "  ".repeat(indent.saturating_sub(1))));
			out
		}
	}
}

fn render_typed_sig_item(item: &TypedSigItem) -> String {
	match item {
		TypedSigItem::Val { name, ty } => format!("val {} : {}", name, render_type(ty)),
		TypedSigItem::Type { name, .. } => format!("type {}", name),
	}
}

fn render_typed_type_def(def: &TypedTypeDef) -> String {
	match def {
		TypedTypeDef::Alias(ty) => render_type(ty),
		TypedTypeDef::Variant(ctors) => format!("variant({})", ctors.iter().map(|c| c.name.clone()).collect::<Vec<_>>().join(" | ")),
		TypedTypeDef::Record(fields) => format!("record({})", fields.iter().map(|f| format!("{}:{}", f.name, render_type(&f.ty))).collect::<Vec<_>>().join(", ")),
		TypedTypeDef::PolyVariant { rows, open } => format!("polyvariant{}({})", if *open { "+" } else { "" }, rows.iter().map(|r| r.tag.clone()).collect::<Vec<_>>().join(" | ")),
	}
}

fn render_typed_block(block: &TypedBlock, indent: usize, out: &mut String) {
	let pad = "  ".repeat(indent);
	out.push_str(&format!("{pad}Block\n"));
	for stmt in &block.stmts {
		out.push_str(&format!("{pad}  {}\n", render_typed_stmt(stmt, indent + 1)));
	}
	if let Some(expr) = &block.tail_expr {
		out.push_str(&format!("{pad}  tail = {}\n", render_typed_expr(expr, indent + 1)));
	}
}

fn render_typed_stmt(stmt: &TypedStmt, indent: usize) -> String {
	match stmt {
		TypedStmt::Let(ld) => format!("let {} = {}", ld.name, render_typed_expr(&ld.value, indent)),
		TypedStmt::Include(inc) => format!("include {}", render_typed_module_expr(&inc.module_expr, indent)),
		TypedStmt::Assign { name, value } => format!("{} = {}", name, render_typed_expr(value, indent)),
		TypedStmt::Expr(expr) => render_typed_expr(expr, indent),
		TypedStmt::Return(expr) => match expr { Some(expr) => format!("return {}", render_typed_expr(expr, indent)), None => "return".to_string() },
	}
}

fn render_typed_expr(expr: &TypedExpr, indent: usize) -> String {
	let _ = indent;
	match &expr.kind {
		TypedExprKind::Ident(name) => format!("{} : {}", name, render_type(&expr.ty)),
		TypedExprKind::ModulePath(path) => format!("{} : {}", path.join("."), render_type(&expr.ty)),
		TypedExprKind::Int(v) => format!("{} : {}", v, render_type(&expr.ty)),
		TypedExprKind::Float(v) => format!("{} : {}", v, render_type(&expr.ty)),
		TypedExprKind::Bool(v) => format!("{} : {}", v, render_type(&expr.ty)),
		TypedExprKind::Char(v) => format!("'{}' : {}", v, render_type(&expr.ty)),
		TypedExprKind::String(v) => format!("\"{}\" : {}", v, render_type(&expr.ty)),
		TypedExprKind::Unit => format!("() : {}", render_type(&expr.ty)),
		TypedExprKind::Tuple(items) => format!("({}) : {}", items.iter().map(|e| render_typed_expr(e, indent)).collect::<Vec<_>>().join(", "), render_type(&expr.ty)),
		TypedExprKind::List(items) => format!("[{}] : {}", items.iter().map(|e| render_typed_expr(e, indent)).collect::<Vec<_>>().join("; "), render_type(&expr.ty)),
		TypedExprKind::Call { callee, args } => format!("{}({}) : {}", callee, args.iter().map(|e| render_typed_expr(e, indent)).collect::<Vec<_>>().join(", "), render_type(&expr.ty)),
		TypedExprKind::Apply { callee, args } => format!("apply({}, {}) : {}", render_typed_expr(callee, indent), args.iter().map(|a| render_typed_expr(&a.value, indent)).collect::<Vec<_>>().join(", "), render_type(&expr.ty)),
		TypedExprKind::LetIn { .. } => "let-in".to_string(),
		TypedExprKind::If { .. } => "if".to_string(),
		TypedExprKind::Match { .. } => "match".to_string(),
		TypedExprKind::Fun { .. } => "fun".to_string(),
		TypedExprKind::Function { .. } => "function".to_string(),
		TypedExprKind::LocalOpen { module_name, .. } => format!("open {}", module_name),
		TypedExprKind::Cons { .. } => "cons".to_string(),
		TypedExprKind::RecordLiteral(_) => "record".to_string(),
		TypedExprKind::RecordUpdate { .. } => "record-update".to_string(),
		TypedExprKind::PolyVariant { tag, .. } => format!("`{}" , tag),
		TypedExprKind::Pipe { value, target } => format!("{} |> {}", render_typed_expr(value, indent), target),
	}
}

fn render_typed_pattern(pattern: &TypedPattern) -> String {
	match pattern {
		TypedPattern::Wildcard => "_".to_string(),
		TypedPattern::Ident(name) => name.clone(),
		TypedPattern::Int(v) => v.to_string(),
		TypedPattern::Char(v) => format!("'{}'", v),
		TypedPattern::String(v) => format!("\"{}\"", v),
		TypedPattern::Constructor { name, .. } => name.clone(),
		TypedPattern::PolyVariant { tag, .. } => format!("`{}", tag),
		TypedPattern::List(items) => format!("[{}]", items.iter().map(render_typed_pattern).collect::<Vec<_>>().join("; ")),
		TypedPattern::Tuple(items) => format!("({})", items.iter().map(render_typed_pattern).collect::<Vec<_>>().join(", ")),
		TypedPattern::As { pattern, alias } => format!("{} as {}", render_typed_pattern(pattern), alias),
	}
}

fn render_type(ty: &Type) -> String {
	match ty {
		Type::I32 => "I32".to_string(),
		Type::F32 => "F32".to_string(),
		Type::Bool => "Bool".to_string(),
		Type::CharAscii => "CharAscii".to_string(),
		Type::CharAwa => "CharAwa".to_string(),
		Type::StrAscii => "StrAscii".to_string(),
		Type::StrAwa => "StrAwa".to_string(),
		Type::S32 => "S32".to_string(),
		Type::U8 => "U8".to_string(),
		Type::Bytes => "Bytes".to_string(),
		Type::Unit => "Unit".to_string(),
		Type::List(inner) => format!("List<{}>", render_type(inner)),
		Type::Option(inner) => format!("Option<{}>", render_type(inner)),
		Type::Result(ok, err) => format!("Result<{}, {}>", render_type(ok), render_type(err)),
		Type::Tuple(items) => format!("({})", items.iter().map(render_type).collect::<Vec<_>>().join(", ")),
		Type::Arrow(items) => format!("fn({})", items.iter().map(render_type).collect::<Vec<_>>().join(", ")),
		Type::PolyVariant { rows, open } => format!("poly{}<{}>", if *open { "+" } else { "" }, rows.iter().map(|r| r.tag.clone()).collect::<Vec<_>>().join(" | ")),
		Type::Named(name) => name.clone(),
	}
}
