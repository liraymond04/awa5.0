pub mod ast;
pub mod ir;
pub mod parser;
pub mod lexer;

pub use ast::{AstItem, AstProgram, Expr, MatchCase, ModuleExpr, ModuleSig, Pattern, TypeRef};
pub use parser::*;
pub use lexer::*;

use ast::*;
use ir::*;
use std::collections::{HashMap, HashSet};

#[derive(Clone)]
struct ExternCatalog {
	ids: HashMap<String, ExternSigId>,
	sigs: Vec<ExternSig>,
}

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
	let ast = elaborate_modules_for_core(&ast)?;
	let ast = lift_if_out_of_let_in(&ast)?;
	type_check_program(&ast)?;
	lower_core_program(&ast)
}

// Phase-2 helper: when `let x = if cond then a else b in ...` occurs,
// lift the `if` outward by duplicating the remainder into both branches.
//
// This avoids needing a phi/merge node in the core IR yet, while still
// ensuring both branches are reachable/compiled.
fn lift_if_out_of_let_in(ast: &AstProgram) -> Result<AstProgram, CompileError> {
	fn lift_expr(expr: Expr) -> Expr {
		match expr {
			Expr::If { cond, then_branch, else_branch } => Expr::If {
				cond: Box::new(lift_expr(*cond)),
				then_branch: Box::new(lift_expr(*then_branch)),
				else_branch: Box::new(lift_expr(*else_branch)),
			},
			Expr::LetIn { is_rec, bindings, body } => {
				// Desugar multi-binding let-ins into nested single-binding let-ins,
				// then apply the if-lifting rule to the outermost binding.
				fn desugar(is_rec: bool, mut bindings: Vec<LetBinding>, body: Expr) -> Expr {
					if bindings.is_empty() {
						return body;
					}
					let first = bindings.remove(0);
					Expr::LetIn {
						is_rec,
						bindings: vec![first],
						body: Box::new(desugar(is_rec, bindings, body)),
					}
				}

				let desugared = desugar(is_rec, bindings, *body);
				match desugared {
					Expr::LetIn { is_rec, bindings, body } => {
						// Only a single binding at this point.
						let binding = bindings.into_iter().next().expect("desugared let-in binding missing");
						match binding.value {
							Expr::If { cond, then_branch, else_branch } => {
								let then_let = Expr::LetIn {
									is_rec,
									bindings: vec![LetBinding {
										pattern: binding.pattern.clone(),
										value: *then_branch,
									}],
									body: body.clone(),
								};
								let else_let = Expr::LetIn {
									is_rec,
									bindings: vec![LetBinding {
										pattern: binding.pattern,
										value: *else_branch,
									}],
									body,
								};
								Expr::If {
									cond: Box::new(lift_expr(*cond)),
									then_branch: Box::new(lift_expr(then_let)),
									else_branch: Box::new(lift_expr(else_let)),
								}
							}
							other => Expr::LetIn {
								is_rec,
								bindings: vec![LetBinding {
									pattern: binding.pattern,
									value: lift_expr(other),
								}],
								body: Box::new(lift_expr(*body)),
							},
						}
					}
					other => lift_expr(other),
				}
			}
			Expr::Tuple(items) => Expr::Tuple(items.into_iter().map(lift_expr).collect()),
			Expr::List(items) => Expr::List(items.into_iter().map(lift_expr).collect()),
			Expr::Call { callee, args } => Expr::Call {
				callee,
				args: args.into_iter().map(lift_expr).collect(),
			},
			Expr::Apply { callee, args } => Expr::Apply {
				callee: Box::new(lift_expr(*callee)),
				args: args
					.into_iter()
					.map(|a| Arg {
						label: a.label,
						value: lift_expr(a.value),
					})
					.collect(),
			},
			Expr::Fun { params, body } => Expr::Fun { params, body: Box::new(lift_expr(*body)) },
			Expr::Function { cases } => Expr::Function {
				cases: cases
					.into_iter()
					.map(|c| MatchCase {
						pattern: c.pattern,
						guard: c.guard.map(|g| lift_expr(g)),
						body: lift_expr(c.body),
					})
					.collect(),
			},
			Expr::LocalOpen { module_name, body } => Expr::LocalOpen { module_name, body: Box::new(lift_expr(*body)) },
			Expr::Cons { head, tail } => Expr::Cons { head: Box::new(lift_expr(*head)), tail: Box::new(lift_expr(*tail)) },
			Expr::RecordLiteral(fields) => Expr::RecordLiteral(
				fields
					.into_iter()
					.map(|f| RecordFieldValue { name: f.name, value: lift_expr(f.value) })
					.collect(),
			),
			Expr::RecordUpdate { base, fields } => Expr::RecordUpdate {
				base: Box::new(lift_expr(*base)),
				fields: fields
					.into_iter()
					.map(|f| RecordFieldValue { name: f.name, value: lift_expr(f.value) })
					.collect(),
			},
			Expr::PolyVariant { tag, payload } => Expr::PolyVariant { tag, payload: payload.map(|p| Box::new(lift_expr(*p))) },
			Expr::Pipe { value, target } => Expr::Pipe { value: Box::new(lift_expr(*value)), target },
			// base/literals
			other => other,
		}
	}

	fn lift_item(item: AstItem) -> AstItem {
		match item {
			AstItem::LetDecl(mut ld) => {
				ld.value = lift_expr(ld.value);
				AstItem::LetDecl(ld)
			}
			AstItem::FnDecl(fd) => {
				let body = fd.body;
				let body = Block {
					stmts: body.stmts,
					tail: body.tail.map(lift_expr),
				};
				AstItem::FnDecl(FnDecl { body, ..fd })
			}
			other => other,
		}
	}

	// Our AST types in this crate are re-exported; `ast::*` names are available here.
	let lifted = AstProgram { items: ast.items.clone().into_iter().map(lift_item).collect() };
	Ok(lifted)
}

fn pattern_idents(pattern: &Pattern) -> Vec<String> {
	match pattern {
		Pattern::Ident(name) => vec![name.clone()],
		Pattern::Tuple(items) => items.iter().flat_map(pattern_idents).collect(),
		_ => vec![],
	}
}

fn substitute_expr(expr: &Expr, subst: &HashMap<String, Expr>, shadow: &mut HashSet<String>) -> Expr {
	match expr {
		Expr::Ident(name) => {
			if shadow.contains(name) {
				Expr::Ident(name.clone())
			} else if let Some(repl) = subst.get(name) {
				repl.clone()
			} else {
				Expr::Ident(name.clone())
			}
		}
		Expr::ModulePath(path) => Expr::ModulePath(path.clone()),
		Expr::Int(v) => Expr::Int(*v),
		Expr::Float(v) => Expr::Float(v.clone()),
		Expr::Bool(v) => Expr::Bool(*v),
		Expr::Char(v) => Expr::Char(v.clone()),
		Expr::AwaChar(v) => Expr::AwaChar(v.clone()),
		Expr::String(v) => Expr::String(v.clone()),
		Expr::AwaString(v) => Expr::AwaString(v.clone()),
		Expr::Unit => Expr::Unit,
		Expr::Tuple(items) => Expr::Tuple(items.iter().map(|e| substitute_expr(e, subst, shadow)).collect()),
		Expr::List(items) => Expr::List(items.iter().map(|e| substitute_expr(e, subst, shadow)).collect()),
		Expr::Call { callee, args } => Expr::Call {
			callee: callee.clone(),
			args: args
				.iter()
				.map(|a| substitute_expr(a, subst, shadow))
				.collect(),
		},
		Expr::Apply { callee, args } => Expr::Apply {
			callee: Box::new(substitute_expr(callee, subst, shadow)),
			args: args
				.iter()
				.map(|a| Arg {
					label: a.label.clone(),
					value: substitute_expr(&a.value, subst, shadow),
				})
				.collect(),
		},
		Expr::LetIn { is_rec, bindings, body } => {
			let mut shadow = shadow.clone();
			let mut out_bindings = Vec::with_capacity(bindings.len());
			for binding in bindings {
				let value = substitute_expr(&binding.value, subst, &mut shadow);
				let mut pat_idents = pattern_idents(&binding.pattern);
				out_bindings.push(LetBinding {
					pattern: binding.pattern.clone(),
					value,
				});
				for n in pat_idents.drain(..) {
					shadow.insert(n);
				}
			}
			let body = Box::new(substitute_expr(body, subst, &mut shadow));
			Expr::LetIn { is_rec: *is_rec, bindings: out_bindings, body }
		}
		Expr::If { cond, then_branch, else_branch } => Expr::If {
			cond: Box::new(substitute_expr(cond, subst, shadow)),
			then_branch: Box::new(substitute_expr(then_branch, subst, shadow)),
			else_branch: Box::new(substitute_expr(else_branch, subst, shadow)),
		},
		Expr::Match { value, cases } => Expr::Match {
			value: Box::new(substitute_expr(value, subst, shadow)),
			cases: cases
				.iter()
				.map(|c| {
					// Conservative: do not substitute inside pattern bindings.
					let mut shadow = shadow.clone();
					for n in pattern_idents(&c.pattern) {
						shadow.insert(n);
					}
					MatchCase {
						pattern: c.pattern.clone(),
						guard: c.guard.as_ref().map(|g| substitute_expr(g, subst, &mut shadow)),
						body: substitute_expr(&c.body, subst, &mut shadow),
					}
				})
				.collect(),
		},
		Expr::Fun { params, body } => {
			let mut shadow = shadow.clone();
			for p in params {
				for n in pattern_idents(p) {
					shadow.insert(n);
				}
			}
			Expr::Fun { params: params.clone(), body: Box::new(substitute_expr(body, subst, &mut shadow)) }
		}
		Expr::Function { cases } => Expr::Function {
			cases: cases
				.iter()
				.map(|c| {
					let mut shadow = shadow.clone();
					for n in pattern_idents(&c.pattern) {
						shadow.insert(n);
					}
					MatchCase {
						pattern: c.pattern.clone(),
						guard: c.guard.as_ref().map(|g| substitute_expr(g, subst, &mut shadow)),
						body: substitute_expr(&c.body, subst, &mut shadow),
					}
				})
				.collect(),
		},
		Expr::LocalOpen { module_name, body } => Expr::LocalOpen {
			module_name: module_name.clone(),
			body: Box::new(substitute_expr(body, subst, shadow)),
		},
		Expr::Cons { head, tail } => Expr::Cons {
			head: Box::new(substitute_expr(head, subst, shadow)),
			tail: Box::new(substitute_expr(tail, subst, shadow)),
		},
		Expr::RecordLiteral(fields) => Expr::RecordLiteral(
			fields
				.iter()
				.map(|f| RecordFieldValue {
					name: f.name.clone(),
					value: substitute_expr(&f.value, subst, shadow),
				})
				.collect(),
		),
		Expr::RecordUpdate { base, fields } => Expr::RecordUpdate {
			base: Box::new(substitute_expr(base, subst, shadow)),
			fields: fields
				.iter()
				.map(|f| RecordFieldValue {
					name: f.name.clone(),
					value: substitute_expr(&f.value, subst, shadow),
				})
				.collect(),
		},
		Expr::PolyVariant { tag, payload } => Expr::PolyVariant {
			tag: tag.clone(),
			payload: payload
				.as_ref()
				.map(|p| Box::new(substitute_expr(p, subst, shadow))),
		},
		Expr::Pipe { value, target } => Expr::Pipe {
			value: Box::new(substitute_expr(value, subst, shadow)),
			target: target.clone(),
		},
	}
}

fn normalize_callee_name(callee: &str) -> String {
	callee
		.rsplit('.')
		.next()
		.map(|s| s.to_string())
		.unwrap_or_else(|| callee.to_string())
}

fn eval_const_int(expr: &Expr) -> Option<i64> {
	match expr {
		Expr::Int(v) => Some(*v),
		_ => None,
	}
}

fn eval_const_bool(expr: &Expr) -> Option<bool> {
	match expr {
		Expr::Bool(v) => Some(*v),
		Expr::Int(v) => Some(*v != 0),
		_ => None,
	}
}

fn is_simple_literal(expr: &Expr) -> bool {
	matches!(
		expr,
		Expr::Int(_)
			| Expr::Bool(_)
			| Expr::String(_)
			| Expr::AwaString(_)
			| Expr::Char(_)
			| Expr::AwaChar(_)
			| Expr::Unit
	)
}

fn all_simple_literals(args: &[Expr]) -> bool {
	args.iter().all(is_simple_literal)
}

fn simplify_for_recursion(
	expr: &Expr,
	let_fundefs: &HashMap<String, LetDecl>,
	active_rec: Option<&str>,
	fuel: usize,
) -> Expr {
	if fuel == 0 {
		return expr.clone();
	}
	match expr {
		Expr::Call { callee, args } => {
			let lookup = normalize_callee_name(callee);
			let sargs: Vec<Expr> = args
				.iter()
				.map(|a| simplify_for_recursion(a, let_fundefs, active_rec, fuel - 1))
				.collect();

			if sargs.len() == 2 {
				if let (Some(a), Some(b)) = (eval_const_int(&sargs[0]), eval_const_int(&sargs[1])) {
					let folded = match lookup.as_str() {
						"+" => Some(Expr::Int(a + b)),
						"-" => Some(Expr::Int(a - b)),
						"*" => Some(Expr::Int(a * b)),
						"/" => Some(Expr::Int(if b == 0 { 0 } else { a / b })),
						"==" => Some(Expr::Bool(a == b)),
						"<" => Some(Expr::Bool(a < b)),
						">" => Some(Expr::Bool(a > b)),
						"eq" => Some(Expr::Bool(a == b)),
						"lt" => Some(Expr::Bool(a < b)),
						"gt" => Some(Expr::Bool(a > b)),
						_ => None,
					};
					if let Some(v) = folded {
						return v;
					}
				}
			}

			if let Some(rec_name) = active_rec {
				if lookup == rec_name {
					if all_simple_literals(&sargs) {
						if let Some(fd) = let_fundefs.get(rec_name) {
						return expand_recursive_call(fd, &sargs, let_fundefs, fuel - 1)
							.unwrap_or(Expr::Call { callee: callee.clone(), args: sargs });
						}
					}
				}
			}

			Expr::Call {
				callee: callee.clone(),
				args: sargs,
			}
		}
		Expr::If {
			cond,
			then_branch,
			else_branch,
		} => {
			let sc = simplify_for_recursion(cond, let_fundefs, active_rec, fuel - 1);
			if let Some(true) = eval_const_bool(&sc) {
				return simplify_for_recursion(then_branch, let_fundefs, active_rec, fuel - 1);
			}
			if let Some(false) = eval_const_bool(&sc) {
				return simplify_for_recursion(else_branch, let_fundefs, active_rec, fuel - 1);
			}
			Expr::If {
				cond: Box::new(sc),
				then_branch: Box::new(simplify_for_recursion(
					then_branch,
					let_fundefs,
					active_rec,
					fuel - 1,
				)),
				else_branch: Box::new(simplify_for_recursion(
					else_branch,
					let_fundefs,
					active_rec,
					fuel - 1,
				)),
			}
		}
		Expr::LetIn {
			is_rec,
			bindings,
			body,
		} => {
			let mut out_bindings = Vec::with_capacity(bindings.len());
			let mut subst: HashMap<String, Expr> = HashMap::new();
			for b in bindings {
				let value = simplify_for_recursion(&b.value, let_fundefs, active_rec, fuel - 1);
				if let Pattern::Ident(name) = &b.pattern {
					if is_simple_literal(&value) {
						subst.insert(name.clone(), value.clone());
					}
				}
				out_bindings.push(LetBinding {
					pattern: b.pattern.clone(),
					value,
				});
			}
			let body0 = simplify_for_recursion(body, let_fundefs, active_rec, fuel - 1);
			if subst.is_empty() {
				Expr::LetIn {
					is_rec: *is_rec,
					bindings: out_bindings,
					body: Box::new(body0),
				}
			} else {
				let mut shadow = HashSet::new();
				let subbed = substitute_expr(&body0, &subst, &mut shadow);
				Expr::LetIn {
					is_rec: *is_rec,
					bindings: out_bindings,
					body: Box::new(subbed),
				}
			}
		}
		Expr::Tuple(items) => Expr::Tuple(
			items
				.iter()
				.map(|e| simplify_for_recursion(e, let_fundefs, active_rec, fuel - 1))
				.collect(),
		),
		Expr::Apply { callee, args } => {
			let scall = simplify_for_recursion(callee, let_fundefs, active_rec, fuel - 1);
			let sargs: Vec<Arg> = args
				.iter()
				.map(|a| Arg {
					label: a.label.clone(),
					value: simplify_for_recursion(&a.value, let_fundefs, active_rec, fuel - 1),
				})
				.collect();
			if let Expr::Ident(fn_name) = &scall {
				let call_args: Vec<Expr> = sargs.iter().map(|a| a.value.clone()).collect();
				if call_args.len() == 2 {
					if let (Some(a), Some(b)) =
						(eval_const_int(&call_args[0]), eval_const_int(&call_args[1]))
					{
						let folded = match fn_name.as_str() {
							"+" => Some(Expr::Int(a + b)),
							"-" => Some(Expr::Int(a - b)),
							"*" => Some(Expr::Int(a * b)),
							"/" => Some(Expr::Int(if b == 0 { 0 } else { a / b })),
							"==" | "eq" => Some(Expr::Bool(a == b)),
							"<" | "lt" => Some(Expr::Bool(a < b)),
							">" | "gt" => Some(Expr::Bool(a > b)),
							_ => None,
						};
						if let Some(v) = folded {
							return v;
						}
					}
				}
				if let Some(rec_name) = active_rec {
					if fn_name == rec_name && all_simple_literals(&call_args) {
						if let Some(fd) = let_fundefs.get(rec_name) {
							if let Some(expanded) =
								expand_recursive_call(fd, &call_args, let_fundefs, fuel - 1)
							{
								return expanded;
							}
						}
					}
				}
			}
			Expr::Apply {
				callee: Box::new(scall),
				args: sargs,
			}
		}
		_ => expr.clone(),
	}
}

fn expand_recursive_call(
	fd: &LetDecl,
	args: &[Expr],
	let_fundefs: &HashMap<String, LetDecl>,
	fuel: usize,
) -> Option<Expr> {
	if fuel == 0 || !fd.is_rec {
		return None;
	}
	if fd.params.len() != args.len() {
		return None;
	}
	let mut subst = HashMap::new();
	for (pat, arg) in fd.params.iter().zip(args.iter()) {
		if let Pattern::Ident(name) = pat {
			subst.insert(name.clone(), arg.clone());
		} else {
			return None;
		}
	}
	let mut shadow = HashSet::new();
	let subbed = substitute_expr(&fd.value, &subst, &mut shadow);
	Some(simplify_for_recursion(
		&subbed,
		let_fundefs,
		Some(&fd.name),
		fuel - 1,
	))
}

fn lower_tuple_pattern_binding(
	value_expr: &Expr,
	tuple_pats: &[Pattern],
	env: &mut HashMap<String, LocalId>,
	stmts: &mut Vec<CoreStmt>,
	externs: &ExternCatalog,
	let_fundefs: &HashMap<String, LetDecl>,
	next_local: &mut usize,
) -> Result<(), CompileError> {
	// Helper: bind tuple components if we can statically see that `value_expr`
	// produces a tuple. This is intentionally partial (enough for our examples).
	match value_expr {
		Expr::Tuple(items) => {
			for (pat, item_expr) in tuple_pats.iter().zip(items.iter()) {
				if let Pattern::Ident(name) = pat {
					let local = lower_expr_to_core(
						item_expr,
						stmts,
						env,
						externs,
						let_fundefs,
						next_local,
					)?;
					env.insert(name.clone(), local);
				}
			}
			Ok(())
		}
		Expr::LetIn { bindings, body, .. } => {
			// Case: `let <bindings> in (a, b)` inside the tuple binding.
			if let Expr::Tuple(items) = body.as_ref() {
				let mut inner_env = env.clone();
				for b in bindings {
					let local = lower_expr_to_core(
						&b.value,
						stmts,
						&inner_env,
						externs,
						let_fundefs,
						next_local,
					)?;
					if let Pattern::Ident(name) = &b.pattern {
						inner_env.insert(name.clone(), local);
					}
				}
				for (pat, item_expr) in tuple_pats.iter().zip(items.iter()) {
					if let Pattern::Ident(name) = pat {
						let local = lower_expr_to_core(
							item_expr,
							stmts,
							&inner_env,
							externs,
							let_fundefs,
							next_local,
						)?;
						env.insert(name.clone(), local);
					}
				}
				Ok(())
			} else {
				// Unknown; at least emit side effects by lowering once.
				let _ = lower_expr_to_core(value_expr, stmts, env, externs, let_fundefs, next_local)?;
				Ok(())
			}
		}
		Expr::Call { callee, args } => {
			let lookup_callee = normalize_callee_name(callee);

			if let Some(f) = let_fundefs.get(&lookup_callee) {
				if !f.is_rec {
					let mut subst: HashMap<String, Expr> = HashMap::new();
					for (idx, param) in f.params.iter().enumerate() {
						if idx >= args.len() {
							break;
						}
						if let Pattern::Ident(name) = param {
							subst.insert(name.clone(), args[idx].clone());
						}
					}
					let mut shadow = HashSet::new();
					let inlined = substitute_expr(&f.value, &subst, &mut shadow);
					return lower_tuple_pattern_binding(
						&inlined,
						tuple_pats,
						env,
						stmts,
						externs,
						let_fundefs,
						next_local,
					);
				}
			}

			// Fallback: emit side effects, but don't bind components.
			let _ = lower_expr_to_core(value_expr, stmts, env, externs, let_fundefs, next_local)?;
			Ok(())
		}
		_ => {
			// Fallback: emit side effects, but don't bind components.
			let _ = lower_expr_to_core(value_expr, stmts, env, externs, let_fundefs, next_local)?;
			Ok(())
		}
	}
}

// Elaboration pass: flatten a small subset of module/functor/include constructs into
// top-level items so lowering/codegen can “see” raylib’s functor body.
//
// This is intentionally minimal and focused on the shapes used in our examples.
fn elaborate_modules_for_core(ast: &AstProgram) -> Result<AstProgram, CompileError> {
	use std::collections::{HashMap, HashSet};

	// This is intentionally limited to what `lower_expr_to_core(...)` and `lower_core_value(...)`
	// can currently handle. We use it to avoid hoisting unsupported constructs (like
	// `Expr::Function` / `Expr::Match`) before those lowering features exist.
	fn is_supported_expr_for_core_lowering(expr: &Expr) -> bool {
		matches!(
			expr,
			Expr::Ident(_)
				| Expr::ModulePath(_)
				| Expr::Int(_)
				| Expr::Float(_)
				| Expr::Bool(_)
				| Expr::Char(_)
				| Expr::AwaChar(_)
				| Expr::String(_)
				| Expr::AwaString(_)
				| Expr::Unit
				| Expr::Call { .. }
				| Expr::LetIn { .. }
				| Expr::If { .. }
				| Expr::Tuple(_)
				| Expr::Apply { .. }
				| Expr::Function { .. }
				| Expr::Match { .. }
		)
	}

	// Collect module struct items and functor definitions.
	let mut module_structs: HashMap<String, Vec<AstItem>> = HashMap::new();
	let mut functors: HashMap<String, (String, ModuleExpr)> = HashMap::new(); // name -> (param_name, body_module_expr)

	for item in &ast.items {
		if let AstItem::ModuleDecl(md) = item {
			match &md.expr {
				ModuleExpr::Struct(items) => {
					module_structs.insert(md.name.clone(), items.clone());
				}
				ModuleExpr::Functor { param_name, body, .. } => {
					functors.insert(md.name.clone(), (param_name.clone(), (**body).clone()));
				}
				_ => {}
			}
		}
	}

	// Helper: rewrite member-access callees like `R.iskeydown` -> `iskeydown`
	// inside the instantiated functor body.
	fn rewrite_member_calls(expr: &mut Expr, param_name: &str) {
		match expr {
			Expr::Call { callee, args } => {
				if callee.starts_with(&(param_name.to_string() + ".")) {
					*callee = callee
						.rsplit('.')
						.next()
						.map(|s| s.to_string())
						.unwrap_or_else(|| callee.clone());
				}
				for arg in args {
					rewrite_member_calls(arg, param_name);
				}
			}
			Expr::Apply { callee, args } => {
				rewrite_member_calls(callee, param_name);
				for arg in args {
					rewrite_member_calls(&mut arg.value, param_name);
				}
			}
			Expr::LetIn { bindings, body, .. } => {
				for b in bindings {
					rewrite_member_calls(&mut b.value, param_name);
				}
				rewrite_member_calls(body, param_name);
			}
			Expr::If { cond, then_branch, else_branch } => {
				rewrite_member_calls(cond, param_name);
				rewrite_member_calls(then_branch, param_name);
				rewrite_member_calls(else_branch, param_name);
			}
			Expr::Tuple(items) => {
				for it in items {
					rewrite_member_calls(it, param_name);
				}
			}
			Expr::List(items) => {
				for it in items {
					rewrite_member_calls(it, param_name);
				}
			}
			Expr::Match { value, cases } => {
				rewrite_member_calls(value, param_name);
				for c in cases {
					if let Some(g) = &mut c.guard {
						rewrite_member_calls(g, param_name);
					}
					rewrite_member_calls(&mut c.body, param_name);
				}
			}
			Expr::Fun { body, .. } | Expr::LocalOpen { body, .. } => {
				rewrite_member_calls(body, param_name);
			}
			Expr::Function { cases } => {
				for c in cases {
					if let Some(g) = &mut c.guard {
						rewrite_member_calls(g, param_name);
					}
					rewrite_member_calls(&mut c.body, param_name);
				}
			}
			Expr::RecordLiteral(fields) => {
				for f in fields {
					rewrite_member_calls(&mut f.value, param_name);
				}
			}
			Expr::RecordUpdate { base, fields } => {
				rewrite_member_calls(base, param_name);
				for f in fields {
					rewrite_member_calls(&mut f.value, param_name);
				}
			}
			Expr::PolyVariant { payload, .. } => {
				if let Some(p) = payload.as_mut() {
					rewrite_member_calls(p, param_name);
				}
			}
			Expr::Pipe { value, .. } => rewrite_member_calls(value, param_name),
			Expr::Cons { head, tail } => {
				rewrite_member_calls(head, param_name);
				rewrite_member_calls(tail, param_name);
			}
			// literals/idents: nothing to rewrite
			_ => {}
		}
	}

	fn rewrite_member_calls_in_item(item: &mut AstItem, param_name: &str) {
		match item {
			AstItem::LetDecl(ld) => rewrite_member_calls(&mut ld.value, param_name),
			AstItem::FnDecl(fd) => {
				for stmt in &mut fd.body.stmts {
					// only Stmt::Let/Assign/Expr/Return exist in our scaffold
					match stmt {
						crate::compiler::ast::Stmt::Let(ld) => rewrite_member_calls(&mut ld.value, param_name),
						crate::compiler::ast::Stmt::Assign { value, .. } => rewrite_member_calls(value, param_name),
						crate::compiler::ast::Stmt::Expr(e) => rewrite_member_calls(e, param_name),
						crate::compiler::ast::Stmt::Return(Some(e)) => rewrite_member_calls(e, param_name),
						crate::compiler::ast::Stmt::Return(None) => {}
						_ => {}
					}
				}
				if let Some(tail) = &mut fd.body.tail {
					rewrite_member_calls(tail, param_name);
				}
			}
			_ => {}
		}
	}

	let mut out_items: Vec<AstItem> = Vec::new();
	let mut seen_hoisted: HashSet<String> = HashSet::new();

	for item in &ast.items {
		// Keep original top-level items (metadata) intact.
		out_items.push(item.clone());

		match item {
			AstItem::ModuleDecl(md) => {
				if let ModuleExpr::Struct(inner_items) = &md.expr {
					// Hoist non-extern definitions for now (lets/fns/types).
					for inner in inner_items {
						let key = format!("hoist:{}:{}", md.name, match inner {
							AstItem::LetDecl(ld) => ld.name.clone(),
							AstItem::FnDecl(fd) => fd.name.clone(),
							AstItem::TypeDecl(td) => td.name.clone(),
							_ => "__skip__".to_string(),
						});

						if seen_hoisted.contains(&key) {
							continue;
						}

						match inner {
							AstItem::LetDecl(ld) => {
								if is_supported_expr_for_core_lowering(&ld.value) {
									out_items.push(inner.clone());
									seen_hoisted.insert(key);
								}
							}
							AstItem::FnDecl(fd) => {
								if fd.body.tail.as_ref().map_or(true, |e| is_supported_expr_for_core_lowering(e)) {
									out_items.push(inner.clone());
									seen_hoisted.insert(key);
								}
							}
							AstItem::TypeDecl(_) => {
								out_items.push(inner.clone());
								seen_hoisted.insert(key);
							}
							_ => {}
						}
					}
				}
			}
			AstItem::IncludeStmt(inc) => {
				// Functor application instantiation: `include MakeApp(Raylib);;`
				if let ModuleExpr::Apply { functor, arg } = &inc.module_expr {
					let (functor_name, arg_name) = match (&**functor, &**arg) {
						(ModuleExpr::Ident(f), ModuleExpr::Ident(a)) => (f.clone(), a.clone()),
						_ => ("functor".to_string(), "arg".to_string()),
					};

					let (param_name, body_expr) = match functors.get(&functor_name) {
						Some(v) => v.clone(),
						None => continue,
					};

					// We only support functor bodies expressed as `struct ... end`.
					let body_items = match &body_expr {
						ModuleExpr::Struct(items) => items.clone(),
						_ => continue,
					};

					for mut inst_item in body_items {
						rewrite_member_calls_in_item(&mut inst_item, &param_name);
						match inst_item {
							AstItem::LetDecl(_) | AstItem::FnDecl(_) | AstItem::TypeDecl(_) | AstItem::ExternDecl(_) => {
								// Avoid hoisting the same generated top-level definition twice.
								let key = format!(
									"inst:{}({})::{}",
									functor_name,
									arg_name,
									match &inst_item {
										AstItem::LetDecl(ld) => ld.name.clone(),
										AstItem::FnDecl(fd) => fd.name.clone(),
										AstItem::TypeDecl(td) => td.name.clone(),
										AstItem::ExternDecl(ed) => ed.name.clone(),
										_ => "__skip__".to_string(),
									}
								);
								if seen_hoisted.contains(&key) {
									continue;
								}
								out_items.push(inst_item);
								seen_hoisted.insert(key);
							}
							_ => {}
						}
					}
				}
			}
			_ => {}
		}
	}

	Ok(AstProgram { items: out_items })
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
	let extern_sigs = collect_extern_sigs(program)?;
	let mut extern_ids = HashMap::new();
	for (idx, sig) in extern_sigs.iter().enumerate() {
		extern_ids.insert(sig.local_name.clone(), ExternSigId(idx));
	}
	let extern_catalog = ExternCatalog { ids: extern_ids, sigs: extern_sigs.clone() };

	// Catalog of `let`-defined functions for non-extern call inlining.
	// This is a minimal facility used by Phase-3.
	let mut let_fundefs: HashMap<String, LetDecl> = HashMap::new();
	for item in &program.items {
		if let AstItem::LetDecl(ld) = item {
			let_fundefs.insert(ld.name.clone(), ld.clone());
		}
	}

	let mut functions = Vec::new();
	for item in &program.items {
		match item {
			AstItem::FnDecl(fd) => functions.push(lower_fn_decl_to_core(fd, &extern_catalog, &let_fundefs)?),
			AstItem::LetDecl(ld) => functions.push(lower_let_decl_to_core(ld, &extern_catalog, &let_fundefs)?),
			AstItem::ModuleDecl(md) => functions.push(lower_module_decl_to_core(md)?),
			AstItem::ModuleTypeDecl(mtd) => functions.push(lower_module_type_decl_to_core(mtd)?),
			AstItem::IncludeStmt(inc) => functions.push(lower_include_stmt_to_core(inc)?),
			AstItem::ExternDecl(ed) => functions.push(lower_extern_decl_to_core(ed)?),
			AstItem::TypeDecl(td) => functions.push(lower_type_decl_to_core(td)?),
		}
	}

	Ok(CoreProgram { extern_sigs, functions })
}

fn collect_extern_sigs(program: &AstProgram) -> Result<Vec<ExternSig>, CompileError> {
	let mut sigs = Vec::new();
	for item in &program.items {
		collect_extern_sigs_from_ast_item(item, &mut sigs)?;
	}
	Ok(sigs)
}

fn collect_extern_sigs_from_ast_item(item: &AstItem, sigs: &mut Vec<ExternSig>) -> Result<(), CompileError> {
	match item {
		AstItem::ExternDecl(ed) => {
			sigs.push(lower_extern_decl(ed)?);
		}
		AstItem::ModuleDecl(md) => {
			collect_extern_sigs_from_module_expr(&md.expr, sigs)?;
		}
		AstItem::ModuleTypeDecl(_) => {}
		AstItem::IncludeStmt(_) => {}
		AstItem::FnDecl(_) => {}
		AstItem::LetDecl(_) => {}
		AstItem::TypeDecl(_) => {}
	}
	Ok(())
}

fn collect_extern_sigs_from_module_expr(expr: &ModuleExpr, sigs: &mut Vec<ExternSig>) -> Result<(), CompileError> {
	match expr {
		ModuleExpr::Struct(items) => {
			for item in items {
				collect_extern_sigs_from_ast_item(item, sigs)?;
			}
		}
		ModuleExpr::Functor { body, .. } => {
			collect_extern_sigs_from_module_expr(body, sigs)?;
		}
		ModuleExpr::Apply { functor, arg } => {
			collect_extern_sigs_from_module_expr(functor, sigs)?;
			collect_extern_sigs_from_module_expr(arg, sigs)?;
		}
		ModuleExpr::Ident(_) => {}
		ModuleExpr::FirstClass(_) => {}
	}
	Ok(())
}

fn lower_fn_decl_to_core(
	fd: &FnDecl,
	externs: &ExternCatalog,
	let_fundefs: &HashMap<String, LetDecl>,
) -> Result<CoreFunc, CompileError> {
	let mut param_env: HashMap<String, LocalId> = HashMap::new();
	for (idx, pattern) in fd.params.iter().enumerate() {
		if let Pattern::Ident(name) = pattern {
			param_env.insert(name.clone(), LocalId(idx));
		}
	}

	let mut cfg = CoreCfgBuilder::new(fd.params.len());
	let entry = cfg.entry_block();
	if let Some(expr) = &fd.body.tail {
		lower_tail_expr_to_core(expr, &param_env, externs, let_fundefs, &mut cfg, entry)?;
	} else {
		cfg.blocks[entry.0].term = CoreTerminator::Return(None);
	}
	Ok(CoreFunc {
		name: fd.name.clone(),
		params: (0..fd.params.len()).map(LocalId).collect(),
		ret: Type::Unit,
		blocks: cfg.blocks,
	})
}

fn lower_let_decl_to_core(
	ld: &LetDecl,
	externs: &ExternCatalog,
	let_fundefs: &HashMap<String, LetDecl>,
) -> Result<CoreFunc, CompileError> {
	let mut params = Vec::new();
	let mut env = HashMap::new();
	for (idx, pattern) in ld.params.iter().enumerate() {
		if let Pattern::Ident(name) = pattern {
			let id = LocalId(idx);
			params.push(id);
			env.insert(name.clone(), id);
		}
	}

	let mut cfg = CoreCfgBuilder::new(params.len());
	let entry = cfg.entry_block();
	lower_tail_expr_to_core(&ld.value, &env, externs, let_fundefs, &mut cfg, entry)?;

	Ok(CoreFunc {
		name: format!("let_{}", ld.name),
		params,
		ret: Type::Unit,
		blocks: cfg.blocks,
	})
}

struct CoreCfgBuilder {
	blocks: Vec<CoreBlock>,
	next_local: usize,
	entry: BlockId,
}

impl CoreCfgBuilder {
	fn new(next_local: usize) -> Self {
		let entry = BlockId(0);
		Self {
			blocks: vec![CoreBlock {
				id: entry,
				stmts: Vec::new(),
				term: CoreTerminator::Return(None),
			}],
			next_local,
			entry,
		}
	}

	fn entry_block(&self) -> BlockId {
		self.entry
	}

	fn new_block(&mut self) -> BlockId {
		let id = BlockId(self.blocks.len());
		self.blocks.push(CoreBlock {
			id,
			stmts: Vec::new(),
			term: CoreTerminator::Return(None),
		});
		id
	}
}

fn lower_tail_expr_to_core(
	expr: &Expr,
	env: &HashMap<String, LocalId>,
	externs: &ExternCatalog,
	let_fundefs: &HashMap<String, LetDecl>,
	cfg: &mut CoreCfgBuilder,
	bb: BlockId,
) -> Result<(), CompileError> {
	match expr {
		Expr::If { cond, then_branch, else_branch } => {
			// Lower condition into the current block.
			let cond_local = {
				let stmts = &mut cfg.blocks[bb.0].stmts;
				lower_expr_to_core(cond, stmts, env, externs, let_fundefs, &mut cfg.next_local)?
			};
			let then_bb = cfg.new_block();
			let else_bb = cfg.new_block();
			cfg.blocks[bb.0].term = CoreTerminator::If {
				cond: cond_local,
				then_bb,
				else_bb,
			};
			lower_tail_expr_to_core(then_branch, env, externs, let_fundefs, cfg, then_bb)?;
			lower_tail_expr_to_core(else_branch, env, externs, let_fundefs, cfg, else_bb)?;
			Ok(())
		}
		Expr::LetIn { bindings, body, .. } => {
			let mut tmp_env = env.clone();
			// Lower bindings in the current block.
			for binding in bindings {
				let local = {
					let stmts = &mut cfg.blocks[bb.0].stmts;
					lower_expr_to_core(&binding.value, stmts, &tmp_env, externs, let_fundefs, &mut cfg.next_local)?
				};
				if let Pattern::Ident(name) = &binding.pattern {
					tmp_env.insert(name.clone(), local);
				}
			}
			lower_tail_expr_to_core(body, &tmp_env, externs, let_fundefs, cfg, bb)
		}
		_ => {
			let local = {
				let stmts = &mut cfg.blocks[bb.0].stmts;
				lower_expr_to_core(expr, stmts, env, externs, let_fundefs, &mut cfg.next_local)?
			};
			cfg.blocks[bb.0].term = CoreTerminator::Return(Some(local));
			Ok(())
		}
	}
}

fn lower_module_decl_to_core(md: &ModuleDecl) -> Result<CoreFunc, CompileError> {
	let mut stmts = vec![CoreStmt::ModuleDecl { name: md.name.clone() }];
	match &md.expr {
		ModuleExpr::Struct(items) => {
			for item in items {
				if let AstItem::IncludeStmt(inc) = item {
					let inc_name = match &inc.module_expr {
						ModuleExpr::Ident(n) => n.clone(),
						_ => "included".to_string(),
					};
					stmts.push(CoreStmt::IncludeModule { name: inc_name });
				}
			}
		}
		ModuleExpr::Functor { param_name, .. } => {
			stmts.push(CoreStmt::ModuleDecl { name: format!("functor_param_{param_name}") });
		}
		ModuleExpr::Apply { functor, arg } => {
			let f = match &**functor {
				ModuleExpr::Ident(n) => n.clone(),
				_ => "functor".to_string(),
			};
			let a = match &**arg {
				ModuleExpr::Ident(n) => n.clone(),
				_ => "arg".to_string(),
			};
			stmts.push(CoreStmt::IncludeModule { name: format!("{f}({a})") });
		}
		_ => {}
	}
	Ok(CoreFunc {
		name: format!("module_{}", md.name),
		params: Vec::new(),
		ret: Type::Unit,
		blocks: vec![CoreBlock {
			id: BlockId(0),
			stmts,
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
		ModuleExpr::Apply { functor, arg } => {
			let f = match &**functor {
				ModuleExpr::Ident(n) => n.clone(),
				_ => "functor".to_string(),
			};
			let a = match &**arg {
				ModuleExpr::Ident(n) => n.clone(),
				_ => "arg".to_string(),
			};
			format!("{f}({a})")
		}
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
		Expr::Float(v) => CoreValue { kind: CoreValueKind::ConstFloat(v.clone()), ty: Type::F32 },
		Expr::Bool(v) => CoreValue { kind: CoreValueKind::ConstBool(*v), ty: Type::Bool },
		Expr::String(v) => CoreValue { kind: CoreValueKind::ConstString(v.clone()), ty: Type::StrAscii },
		Expr::AwaString(v) => CoreValue { kind: CoreValueKind::ConstString(v.clone()), ty: Type::StrAwa },
		Expr::Char(v) => CoreValue {
			kind: CoreValueKind::ConstInt(v.chars().next().unwrap_or('\0') as i64),
			ty: Type::CharAscii,
		},
		Expr::AwaChar(v) => CoreValue {
			kind: CoreValueKind::ConstInt(v.chars().next().unwrap_or('\0') as i64),
			ty: Type::CharAwa,
		},
		Expr::Unit => CoreValue { kind: CoreValueKind::ConstInt(0), ty: Type::Unit },
		Expr::Ident(name) => CoreValue {
			kind: env.get(name).copied().map(CoreValueKind::Local).unwrap_or_else(|| CoreValueKind::Local(LocalId(hash_name(name)))),
			ty: Type::Named("unknown".to_string()),
		},
		Expr::ModulePath(path) => CoreValue { kind: CoreValueKind::ModulePath(path.clone()), ty: Type::Named("unknown".to_string()) },
		_ => return Err(CompileError::Lowering("unsupported expression for CoreValue lowering")),
	})
}

fn lower_expr_to_core(
	expr: &Expr,
	stmts: &mut Vec<CoreStmt>,
	env: &HashMap<String, LocalId>,
	externs: &ExternCatalog,
	let_fundefs: &HashMap<String, LetDecl>,
	next_local: &mut usize,
) -> Result<LocalId, CompileError> {
	match expr {
		Expr::Ident(name) => {
			if let Some(local) = env.get(name).copied() {
				return Ok(local);
			}
			let value = lower_core_value(expr, env)?;
			let dst = LocalId(*next_local);
			*next_local += 1;
			stmts.push(CoreStmt::Let { dst, value });
			Ok(dst)
		}
		Expr::Call { callee, args } => {
			let lookup_callee = callee
				.rsplit('.')
				.next()
				.map(|s| s.to_string())
				.unwrap_or_else(|| callee.clone());

			// Builtin print intrinsics:
			// - `print expr`   : print without newline and return expr value
			// - `println expr` : print with newline and return expr value
			if (lookup_callee == "print" || lookup_callee == "println") && args.len() == 1 {
				let src = lower_expr_to_core(&args[0], stmts, env, externs, let_fundefs, next_local)?;
				stmts.push(CoreStmt::Print {
					src,
					newline: lookup_callee == "println",
				});
				return Ok(src);
			}
			if let Some(f) = let_fundefs.get(&lookup_callee) {
				if f.is_rec {
					if all_simple_literals(args) {
						if let Some(expanded) = expand_recursive_call(f, args, let_fundefs, 256) {
						return lower_expr_to_core(
							&expanded,
							stmts,
							env,
							externs,
							let_fundefs,
							next_local,
						);
						}
					}
				}
			}
			if externs.ids.contains_key(&lookup_callee) {
			let mut lowered_args = Vec::new();
			for arg in args {
				let local = lower_expr_to_core(arg, stmts, env, externs, let_fundefs, next_local)?;
				lowered_args.push(local);
			}
			let sig_id = *externs.ids.get(&lookup_callee).expect("checked contains_key");
			let decode = externs
				.sigs
				.get(sig_id.0)
				.map(|sig| decode_for_type(&sig.ret))
				.unwrap_or(ReturnDecode::Bytes);
			let dst = LocalId(*next_local);
			*next_local += 1;
			stmts.push(CoreStmt::ExternCall {
				dst: Some(dst),
				call: CoreExternCall {
					sig: sig_id,
					args: lowered_args,
					decode,
				},
			});
			Ok(dst)
			} else {
				// Non-extern call:
				// 1) Minimal function dispatch for `let f = function | PAT -> EXPR ...`.
				if args.len() == 1 {
					if let Some(f) = let_fundefs.get(&lookup_callee) {
						if !f.is_rec {
							if let Expr::Function { cases } = &f.value {
								let input_expr = &args[0];
								let value_ctor_opt = match input_expr {
									Expr::Ident(s) => Some(s.clone()),
									Expr::ModulePath(p) => p.last().cloned(),
									_ => None,
								};

								if let Some(value_ctor) = value_ctor_opt {
									let mut selected: Option<&MatchCase> = None;
									for case in cases {
										match &case.pattern {
											Pattern::Constructor { name, payload: None } if name == &value_ctor => {
												selected = Some(case);
												break;
											}
											Pattern::Wildcard => selected = Some(case),
											_ => {}
										}
									}
									if let Some(case) = selected {
										let dst_local = lower_expr_to_core(
											&case.body,
											stmts,
											env,
											externs,
											let_fundefs,
											next_local,
										)?;
										return Ok(dst_local);
									}
								}
							}
						}
					}
				}

				// 2) Minimal non-recursive inlining for `let`-defined functions.
				if let Some(f) = let_fundefs.get(&lookup_callee) {
					if !f.is_rec {
						let mut subst: HashMap<String, Expr> = HashMap::new();
						for (idx, param) in f.params.iter().enumerate() {
							if idx >= args.len() {
								break;
							}
							if let Pattern::Ident(name) = param {
								subst.insert(name.clone(), args[idx].clone());
							}
						}

						let mut shadow = HashSet::new();
						let inlined = substitute_expr(&f.value, &subst, &mut shadow);
						return lower_expr_to_core(&inlined, stmts, env, externs, let_fundefs, next_local);
					}
				}

				// Operator lowering: parser emits operator calls like `+`/`-`/`eq`/`lt` with exactly 2 args.
				if args.len() == 2 {
					let op = match lookup_callee.as_str() {
						"+" => Some(CoreBinOp::Add),
						"-" => Some(CoreBinOp::Sub),
						"*" => Some(CoreBinOp::Mul),
						"/" => Some(CoreBinOp::Div),
						"eq" => Some(CoreBinOp::Eql),
						"lt" => Some(CoreBinOp::Lss),
						"gt" => Some(CoreBinOp::Gr8),
						_ => None,
					};

					if let Some(op) = op {
						let lhs = lower_expr_to_core(&args[0], stmts, env, externs, let_fundefs, next_local)?;
						let rhs = lower_expr_to_core(&args[1], stmts, env, externs, let_fundefs, next_local)?;

						let dst = LocalId(*next_local);
						*next_local += 1;
						stmts.push(CoreStmt::OpBinary { dst, op, lhs, rhs });
						return Ok(dst);
					}
				}

				// Fallback scaffold: evaluate only the first argument.
				if !args.is_empty() {
					let first = lower_expr_to_core(&args[0], stmts, env, externs, let_fundefs, next_local)?;
					for arg in args.iter().skip(1) {
						let _ = lower_expr_to_core(arg, stmts, env, externs, let_fundefs, next_local)?;
					}
					Ok(first)
				} else {
					let dst = LocalId(*next_local);
					*next_local += 1;
					stmts.push(CoreStmt::Let {
						dst,
						value: CoreValue { kind: CoreValueKind::ConstInt(0), ty: Type::I32 },
					});
					Ok(dst)
				}
			}
		}
		Expr::LetIn { bindings, body, .. } => {
			let mut tmp_env = env.clone();
			for binding in bindings {
				match &binding.pattern {
					Pattern::Ident(name) => {
						let local = lower_expr_to_core(&binding.value, stmts, &tmp_env, externs, let_fundefs, next_local)?;
						tmp_env.insert(name.clone(), local);
					}
					Pattern::Tuple(tuple_pats) => {
						lower_tuple_pattern_binding(&binding.value, tuple_pats, &mut tmp_env, stmts, externs, let_fundefs, next_local)?;
					}
					_ => {
						// Unknown binding pattern; still lower to preserve side effects.
						let _ = lower_expr_to_core(&binding.value, stmts, &tmp_env, externs, let_fundefs, next_local)?;
					}
				}
			}
			lower_expr_to_core(body, stmts, &tmp_env, externs, let_fundefs, next_local)
		}
		// Expression-level lowering: evaluate condition and both branches for side effects.
		// This does not yet create a merge/phi; tail position uses `lower_tail_expr_to_core`
		// which creates proper CFG blocks.
		Expr::If { cond, then_branch, else_branch } => {
			let _cond_local = lower_expr_to_core(cond, stmts, env, externs, let_fundefs, next_local)?;
			let then_local = lower_expr_to_core(then_branch, stmts, env, externs, let_fundefs, next_local)?;
			let _else_local = lower_expr_to_core(else_branch, stmts, env, externs, let_fundefs, next_local)?;
			let _ = _cond_local;
			Ok(then_local)
		}
		Expr::Tuple(items) => {
			if let Some(first) = items.first() {
				lower_expr_to_core(first, stmts, env, externs, let_fundefs, next_local)
			} else {
				let dst = LocalId(*next_local);
				*next_local += 1;
				stmts.push(CoreStmt::Let {
					dst,
					value: CoreValue { kind: CoreValueKind::ConstInt(0), ty: Type::Unit },
				});
				Ok(dst)
			}
		}
		Expr::Apply { callee, args } => {
			if let Expr::Ident(fn_name) = callee.as_ref() {
				if let Some(f) = let_fundefs.get(fn_name) {
					if f.is_rec {
						let call_args: Vec<Expr> = args.iter().map(|a| a.value.clone()).collect();
						if all_simple_literals(&call_args) {
							if let Some(expanded) = expand_recursive_call(f, &call_args, let_fundefs, 256) {
								return lower_expr_to_core(
									&expanded,
									stmts,
									env,
									externs,
									let_fundefs,
									next_local,
								);
							}
						}
					}
				}
			}
			// Attempt minimal function/match dispatch for `Expr::Function` values stored
			// in `let` bindings. This is enough for raylib’s `keycode_of_key`.
			if let Expr::Ident(fn_name) = callee.as_ref() {
				if let Some(f) = let_fundefs.get(fn_name) {
					// Function literals should have been elaborated/hoisted as `let name = function | ... -> ...`.
					if let Expr::Function { cases } = &f.value {
						if args.len() == 1 {
							let input_expr = &args[0].value;
							// Support constructor constants for patterns (e.g. KEY_W).
							// Depending on parsing/elaboration, such values can show up as:
							// - `Expr::Ident("KEY_W")`
							// - `Expr::ModulePath([...,"KEY_W"])`
							let value_ctor_opt = match input_expr {
								Expr::Ident(s) => Some(s.clone()),
								Expr::ModulePath(p) => p.last().cloned(),
								_ => None,
							};

							if let Some(value_ctor) = value_ctor_opt {
								// Find matching case by constructor name (wildcard supported).
								let mut selected: Option<&MatchCase> = None;
								for case in cases {
									match &case.pattern {
										Pattern::Constructor { name, payload: None } if name == &value_ctor => {
											selected = Some(case);
											break;
										}
										Pattern::Wildcard => {
											selected = Some(case);
										}
										_ => {}
									}
								}

								if let Some(case) = selected {
									let dst_local = lower_expr_to_core(&case.body, stmts, env, externs, let_fundefs, next_local)?;
									return Ok(dst_local);
								}
							}
						}
					}
				}
			}

			// Fallback: evaluate callee and then first arg only.
			let _ = lower_expr_to_core(callee, stmts, env, externs, let_fundefs, next_local)?;
			if let Some(first) = args.first() {
				lower_expr_to_core(&first.value, stmts, env, externs, let_fundefs, next_local)
			} else {
				let dst = LocalId(*next_local);
				*next_local += 1;
				stmts.push(CoreStmt::Let {
					dst,
					value: CoreValue { kind: CoreValueKind::ConstInt(0), ty: Type::I32 },
				});
				Ok(dst)
			}
		}
		// Function/match values are first-class in the surface language, but the core
		// scaffold backend doesn’t yet represent them. We keep them as placeholders
		// when lowered as plain values; actual dispatch is handled in `Expr::Apply`
		// (and later we’ll add real lowering).
		Expr::Function { .. } | Expr::Match { .. } => {
			let dst = LocalId(*next_local);
			*next_local += 1;
			stmts.push(CoreStmt::Let {
				dst,
				value: CoreValue { kind: CoreValueKind::ConstInt(0), ty: Type::I32 },
			});
			Ok(dst)
		}
		_ => {
			let value = lower_core_value(expr, env)?;
			let dst = LocalId(*next_local);
			*next_local += 1;
			stmts.push(CoreStmt::Let { dst, value });
			Ok(dst)
		}
	}
}

fn decode_for_type(ty: &Type) -> ReturnDecode {
	match ty {
		Type::Unit => ReturnDecode::None,
		// The VM `Lib` path for single-byte returns already leaves a `Bubble::Simple`
		// on top of the bubble abyss; for conditional-skip patterns we must not
		// clobber that value with extra decoding bubbles.
		Type::U8 | Type::Bool | Type::CharAscii | Type::CharAwa => ReturnDecode::None,
		Type::I32 | Type::S32 => ReturnDecode::DecodeI32Le,
		Type::F32 => ReturnDecode::DecodeF32Le,
		_ => ReturnDecode::Bytes,
	}
}

fn extern_tag_for_type(ty: &Type) -> u8 {
	match ty {
		Type::I32 | Type::F32 => 0x0,
		Type::CharAwa => 0x1,
		Type::CharAscii => 0x2,
		Type::StrAwa => 0x3,
		Type::StrAscii => 0x4,
		Type::S32 | Type::U8 | Type::Bool => 0x5,
		_ => 0x0,
	}
}

fn emit_cstring_symbol(symbol: &str, instructions: &mut Vec<Awatism>) {
	for byte in symbol.as_bytes() {
		instructions.push(Awatism::Blo(*byte));
	}
	// C-string terminator for lib symbol name payload.
	instructions.push(Awatism::Blo(0));
	let count = symbol.len().saturating_add(1).min(u8::MAX as usize) as u8;
	instructions.push(Awatism::Srn(count));
}

fn resolve_const_kind(mut id: LocalId, locals: &HashMap<LocalId, CoreValue>) -> Option<CoreValueKind> {
	use std::collections::HashSet;

	let mut visited: HashSet<LocalId> = HashSet::new();
	loop {
		if !visited.insert(id) {
			return None;
		}
		let cur = locals.get(&id)?.kind.clone();
		match cur {
			CoreValueKind::Local(next) => {
				id = next;
			}
			CoreValueKind::ConstInt(_)
			| CoreValueKind::ConstFloat(_)
			| CoreValueKind::ConstBool(_)
			| CoreValueKind::ConstString(_) => return Some(cur),
			_ => return None,
		}
	}
}

fn emit_typed_arg_from_local(
	tag: u8,
	_expected_ty: Option<&Type>,
	local: LocalId,
	locals: &HashMap<LocalId, CoreValue>,
	instructions: &mut Vec<Awatism>,
) {
	// Emit a typed argument bubble in the exact VM shape that the AWASM macros use:
	// - !*_i32: `Blo(tag)` + payloadDouble + `Srn(2)` where payloadDouble is a `Srn(4)` result
	// - !*_chr: `Blo(tag)` + payloadSimple + `Srn(2)`
	// - !*_str: `Blo(tag)` + payloadDoubleBuiltLikeProcessStr + `Srn(2)`
	//
	// The VM `dynlib::parse_fn_args` expects an outer Double whose bubbles are `[tag, payloadBubble]`.

	fn emit_payload_as_i32_le_double(i: i32, instructions: &mut Vec<Awatism>) {
		for byte in i.to_le_bytes() {
			instructions.push(Awatism::Blo(byte));
		}
		// payloadDouble
		instructions.push(Awatism::Srn(4));
	}

	fn emit_payload_as_simple_i32(i: i32, instructions: &mut Vec<Awatism>) {
		instructions.push(Awatism::Blo(i as u8));
	}

	fn emit_payload_as_string_double(s: &str, awascii: bool, instructions: &mut Vec<Awatism>) {
		// Mirror `parser::awasm::process_str` chunking:
		// - split into <= 31-byte chunks
		// - for each chunk: push bytes in reverse order, `Srn(chunk_len)`
		// - merge chunks with `Mrg` between them
		let chunk_size: usize = 31;

		let mut bytes: Vec<u8> = s.bytes().collect();

		if awascii {
			bytes = bytes
				.iter()
				.map(|b| {
					let ch = *b as char;
					crate::AWA_SCII
						.chars()
						.position(|c| c == ch)
						.map(|idx| idx as u8)
						.unwrap_or(0)
				})
				.collect();
		}

		let mut i = 0u32;
		while !bytes.is_empty() {
			let len = bytes.len();
			let end = if len > chunk_size { len - chunk_size } else { 0 };
			for _ in end..len {
				let b = bytes.pop().unwrap();
				instructions.push(Awatism::Blo(b));
			}
			instructions.push(Awatism::Srn((len - end) as u8));
			if i > 0 {
				instructions.push(Awatism::Mrg);
			}
			i += 1;
		}
	}

	let resolved_kind = resolve_const_kind(local, locals);

	// Non-constant locals: best-effort stack-based payload serialization.
	// We assume the runtime bubble for this `local` is currently at the top of
	// the bubble abyss at this point in codegen (stack-aware locals/joins will
	// make this assumption reliable).
	if resolved_kind.is_none() && matches!(tag, 0x0 | 0x1 | 0x2 | 0x5) {
		instructions.push(Awatism::Dpl); // payloadDup
		instructions.push(Awatism::Blo(tag)); // tag on top
		instructions.push(Awatism::Sbm(1)); // insert tag below payloadDup
		instructions.push(Awatism::Srn(2)); // [tag, payloadDup]
		return;
	}

	// Constant path: emit `tag`, then build the payload bubble.
	instructions.push(Awatism::Blo(tag));

	// Phase-6 helper: emit the VM *payload bubble* for ABI typed packets.
	// This leaves the payload bubble on top of the stack above `Blo(tag)`.
	fn emit_value_bubble_payload_for_tag(
		tag: u8,
		resolved_kind: Option<&CoreValueKind>,
		local: LocalId,
		instructions: &mut Vec<Awatism>,
	) {
		match (tag, resolved_kind) {
			(0x0, Some(CoreValueKind::ConstInt(v))) => emit_payload_as_i32_le_double(*v as i32, instructions),
			(0x0, Some(CoreValueKind::ConstFloat(v))) => {
				let f = v.parse::<f32>().unwrap_or(0.0);
				emit_payload_as_i32_le_double(f.to_bits() as i32, instructions)
			}
			(0x0, _) => emit_payload_as_i32_le_double(local.0 as i32, instructions),

			(0x5, Some(CoreValueKind::ConstBool(b))) => {
				emit_payload_as_simple_i32(if *b { 1 } else { 0 }, instructions)
			}
			(0x5, Some(CoreValueKind::ConstInt(v))) => emit_payload_as_simple_i32(*v as i32, instructions),
			(0x5, _) => emit_payload_as_simple_i32(local.0 as i32, instructions),

			(0x1, Some(CoreValueKind::ConstInt(v))) => {
				let ch = (*v as u8) as char;
				let idx = crate::AWA_SCII
					.chars()
					.position(|c| c == ch)
					.map(|i| i as u8)
					.unwrap_or(0);
				instructions.push(Awatism::Blo(idx));
			}
			(0x1, _) => instructions.push(Awatism::Blo(0)),

			(0x2, Some(CoreValueKind::ConstInt(v))) => instructions.push(Awatism::Blo(*v as u8)),
			(0x2, _) => instructions.push(Awatism::Blo(0)),

			(0x3, Some(CoreValueKind::ConstString(s))) => emit_payload_as_string_double(s, true, instructions),
			(0x4, Some(CoreValueKind::ConstString(s))) => emit_payload_as_string_double(s, false, instructions),
			(0x3 | 0x4, _) => {
				// Fallback: empty string payload.
				instructions.push(Awatism::Blo(0));
				instructions.push(Awatism::Srn(1));
			}

			// If types/tags don’t line up yet, fall back to i32-ish payload shape.
			(_, _) => emit_payload_as_i32_le_double(local.0 as i32, instructions),
		}
	}

	emit_value_bubble_payload_for_tag(tag, resolved_kind.as_ref(), local, instructions);

	// Outer typed arg bubble: Double([tag, payloadBubble])
	instructions.push(Awatism::Srn(2));
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
		Expr::AwaChar(v) => (TypedExprKind::Char(v.clone()), Type::CharAwa),
		Expr::String(v) => (TypedExprKind::String(v.clone()), Type::StrAscii),
		Expr::AwaString(v) => (TypedExprKind::String(v.clone()), Type::StrAwa),
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
		CoreStmt::CallUser { dst, callee, args } => format!(
			"{}call {}({})",
			dst.map(|d| format!("%{} = ", d.0)).unwrap_or_default(),
			callee,
			args.iter()
				.map(|a| format!("%{}", a.0))
				.collect::<Vec<_>>()
				.join(", ")
		),
		CoreStmt::Print { src, newline } => format!("print %{}{}", src.0, if *newline { " \\n" } else { "" }),
		CoreStmt::ExternCall { dst, call } => format!("{}extern_call sig={} args=[{}] decode={:?}", dst.map(|d| format!("%{} = ", d.0)).unwrap_or_default(), call.sig.0, call.args.iter().map(|a| format!("%{}", a.0)).collect::<Vec<_>>().join(", "), call.decode),
		CoreStmt::OpBinary { dst, op, lhs, rhs } => {
			format!("let %{} = op{:?} %{} %{}", dst.0, op, lhs.0, rhs.0)
		}
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
		CoreValueKind::ConstFloat(v) => v.clone(),
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

// AWASM Code Generation
use crate::Awatism;

pub fn compile_to_awasm(source: &str) -> Result<Vec<Awatism>, CompileError> {
	let program = compile_to_core(source)?;
	codegen_program(&program)
}

fn codegen_program(program: &CoreProgram) -> Result<Vec<Awatism>, CompileError> {
	let mut instructions = Vec::new();

	// Start execution at `main` when present. Otherwise preserve existing
	// behavior and begin at the first emitted function.
	if let Some(main_idx) = program
		.functions
		.iter()
		.position(|f| f.name == "let_main" || f.name == "main")
	{
		instructions.push(Awatism::JmpRelStr(format!("fn_{main_idx}")));
	}
	
	for (idx, func) in program.functions.iter().enumerate() {
		instructions.push(Awatism::StrLbl(format!("fn_{idx}")));
		codegen_function(func, &program.extern_sigs, &mut instructions)?;
	}

	// Ensure a final termination for the entire program.
	
	instructions.push(Awatism::Trm);
	
	Ok(instructions)
}

fn codegen_function(func: &CoreFunc, extern_sigs: &[ExternSig], instructions: &mut Vec<Awatism>) -> Result<(), CompileError> {
	for block in &func.blocks {
		instructions.push(Awatism::StrLbl(format!("{}_bb{}", func.name, block.id.0)));
		codegen_block(&func.name, block, extern_sigs, instructions, &format!("{}_end", func.name))?;
	}
	let end_label = format!("{}_end", func.name);
	instructions.push(Awatism::StrLbl(end_label));
	instructions.push(Awatism::Nop);

	Ok(())
}

fn codegen_block(
	func_name: &str,
	block: &CoreBlock,
	extern_sigs: &[ExternSig],
	instructions: &mut Vec<Awatism>,
	end_label: &str,
) -> Result<(), CompileError> {
	let mut locals: HashMap<LocalId, CoreValue> = HashMap::new();
	// Generate code for each statement in the block
	for stmt in &block.stmts {
		codegen_statement(stmt, &mut locals, extern_sigs, instructions)?;
	}
	
	// Handle the terminator
	match &block.term {
		CoreTerminator::Return(None) => {
			// Prevent branch fallthrough: once a block "returns", jump to function end.
			instructions.push(Awatism::JmpRelStr(end_label.to_string()));
		}
		CoreTerminator::Return(Some(_local)) => {
			// Prevent branch fallthrough: once a block "returns", jump to function end.
			instructions.push(Awatism::JmpRelStr(end_label.to_string()));
		}
		CoreTerminator::Goto(bb) => {
			instructions.push(Awatism::JmpRelStr(format!("{}_bb{}", func_name, bb.0)));
		}
		CoreTerminator::If { cond, then_bb, else_bb } => {
			// If the condition is statically known, emit a single deterministic jump.
			let selected = match resolve_const_kind(*cond, &locals) {
				Some(CoreValueKind::ConstBool(true)) => Some(*then_bb),
				Some(CoreValueKind::ConstBool(false)) => Some(*else_bb),
				Some(CoreValueKind::ConstInt(v)) => Some(if v == 0 { *else_bb } else { *then_bb }),
				_ => None,
			};
			if let Some(bb) = selected {
				instructions.push(Awatism::JmpRelStr(format!("{}_bb{}", func_name, bb.0)));
			} else {
				// Dynamic conditional-skip lowering.
				// Assumption: `cond` bubble is currently on top of the bubble abyss
				// (true for our CFG builder which lowers `cond` immediately before
				// the `If` terminator).
				instructions.push(Awatism::Blo(0));
				instructions.push(Awatism::Eql);
				instructions.push(Awatism::JmpRelStr(format!("{}_bb{}", func_name, else_bb.0)));
			}
		}
	}
	
	Ok(())
}

fn codegen_statement(
	stmt: &CoreStmt,
	locals: &mut HashMap<LocalId, CoreValue>,
	extern_sigs: &[ExternSig],
	instructions: &mut Vec<Awatism>,
) -> Result<(), CompileError> {
	match stmt {
		CoreStmt::Let { dst, value } => {
			locals.insert(*dst, value.clone());
		}
		CoreStmt::Assign { dst, value } => {
			locals.insert(*dst, value.clone());
		}
		CoreStmt::CallUser { dst, .. } => {
			if let Some(dst) = dst {
				locals.insert(
					*dst,
					CoreValue {
						kind: CoreValueKind::Local(*dst),
						ty: Type::I32,
					},
				);
			}
		}
		CoreStmt::Print { src, newline } => {
			// Print without consuming source by duplicating top-like value.
			let src_kind = locals
				.get(src)
				.map(|v| v.kind.clone())
				.unwrap_or(CoreValueKind::Local(*src));
			codegen_value(&CoreValue { kind: src_kind, ty: Type::I32 }, instructions)?;
			let print_op = match locals.get(src).map(|v| &v.ty) {
				Some(Type::StrAscii | Type::StrAwa | Type::CharAscii | Type::CharAwa) => Awatism::Prn,
				_ => Awatism::Pr1,
			};
			instructions.push(print_op);
			if *newline {
				let nl_idx = crate::AWA_SCII
					.chars()
					.position(|c| c == '\n')
					.unwrap_or(0) as u8;
				instructions.push(Awatism::Blo(nl_idx));
				instructions.push(Awatism::Prn);
			}
		}
		CoreStmt::ExternCall { dst: _, call } => {
			let sig = extern_sigs
				.get(call.sig.0)
				.ok_or(CompileError::Lowering("invalid extern signature id"))?;

			emit_cstring_symbol(&sig.symbol_name, instructions);

			for (idx, arg) in call.args.iter().enumerate() {
				let tag = sig
					.params
					.get(idx)
					.and_then(|p| p.explicit_tag.or(Some(extern_tag_for_type(&p.ty))))
					.unwrap_or(0x0);
				let expected_ty = sig.params.get(idx).map(|p| &p.ty);
				emit_typed_arg_from_local(tag, expected_ty, *arg, locals, instructions);
			}

			if !call.args.is_empty() {
				let argc = call.args.len().min(u8::MAX as usize) as u8;
				instructions.push(Awatism::Srn(argc));
				instructions.push(Awatism::Srn(2));
			} else {
				instructions.push(Awatism::Srn(1));
			}

			instructions.push(Awatism::Lib);
			match call.decode {
				ReturnDecode::None => {}
				ReturnDecode::Bytes => instructions.push(Awatism::Pr1),
				ReturnDecode::DecodeU8 => instructions.push(Awatism::Blo(0)),
				ReturnDecode::DecodeI32Le => instructions.push(Awatism::Srn(4)),
				ReturnDecode::DecodeF32Le => instructions.push(Awatism::Srn(4)),
			}
		}
		CoreStmt::OpBinary { dst, op, lhs, rhs } => {
			// Emit VM opcode. This is currently “structural”: without stack-aware
			// locals this may not be runtime-correct yet, but it enables fixtures/opcodes.
			// Materialize operands from `locals` when they are compile-time constants.
			// Otherwise we conservatively fall back to `Local` and keep the existing
			// `Dpl`-based behavior.
			let lhs_kind = locals
				.get(lhs)
				.map(|v| v.kind.clone())
				.unwrap_or(CoreValueKind::Local(*lhs));
			let rhs_kind = locals
				.get(rhs)
				.map(|v| v.kind.clone())
				.unwrap_or(CoreValueKind::Local(*rhs));
			codegen_value(&CoreValue { kind: lhs_kind, ty: Type::I32 }, instructions)?;
			codegen_value(&CoreValue { kind: rhs_kind, ty: Type::I32 }, instructions)?;
			let awop = match op {
				CoreBinOp::Add => Awatism::Add,
				CoreBinOp::Sub => Awatism::Sub,
				CoreBinOp::Mul => Awatism::Mul,
				CoreBinOp::Div => Awatism::Div,
				CoreBinOp::Eql => Awatism::Eql,
				CoreBinOp::Lss => Awatism::Lss,
				CoreBinOp::Gr8 => Awatism::Gr8,
			};
			instructions.push(awop);

			// Conservative value tracking for later ABI packing: if both operands
			// are compile-time constants, fold and record. Otherwise record Local.
			let lhs_kind = resolve_const_kind(*lhs, locals);
			let rhs_kind = resolve_const_kind(*rhs, locals);
			if let (Some(CoreValueKind::ConstInt(a)), Some(CoreValueKind::ConstInt(b))) = (lhs_kind, rhs_kind) {
				let folded = match op {
					CoreBinOp::Add => Some(a + b),
					CoreBinOp::Sub => Some(a - b),
					CoreBinOp::Mul => Some(a * b),
					CoreBinOp::Div => Some(if b == 0 { 0 } else { a / b }),
					CoreBinOp::Eql => Some(if a == b { 1 } else { 0 }),
					CoreBinOp::Lss => Some(if a < b { 1 } else { 0 }),
					CoreBinOp::Gr8 => Some(if a > b { 1 } else { 0 }),
				};
				if let Some(v) = folded {
					let ty = match op {
						CoreBinOp::Eql | CoreBinOp::Lss | CoreBinOp::Gr8 => Type::Bool,
						_ => Type::I32,
					};
					locals.insert(*dst, CoreValue { kind: CoreValueKind::ConstInt(v), ty });
				} else {
					locals.insert(*dst, CoreValue { kind: CoreValueKind::Local(*dst), ty: Type::I32 });
				}
			} else {
				locals.insert(*dst, CoreValue { kind: CoreValueKind::Local(*dst), ty: Type::I32 });
			}
		}
		CoreStmt::ModuleDecl { .. } => {
			// Module declarations don't generate code, they're metadata
		}
		CoreStmt::IncludeModule { .. } => {
			// Include statements don't generate code
		}
	}
	Ok(())
}

#[allow(dead_code)]
fn codegen_value(value: &CoreValue, instructions: &mut Vec<Awatism>) -> Result<(), CompileError> {
	match &value.kind {
		CoreValueKind::Local(_id) => {
			instructions.push(Awatism::Dpl);
		}
		CoreValueKind::ConstInt(n) => {
			instructions.push(Awatism::Blo((*n as i8) as u8));
		}
		CoreValueKind::ConstFloat(v) => {
			for byte in v.parse::<f32>().unwrap_or(0.0).to_le_bytes() {
				instructions.push(Awatism::Blo(byte));
			}
			instructions.push(Awatism::Srn(4));
		}
		CoreValueKind::ConstBool(b) => {
			instructions.push(Awatism::Blo(if *b { 1 } else { 0 }));
		}
		CoreValueKind::ConstString(s) => {
			let len = s.len().min(u8::MAX as usize) as u8;
			instructions.push(Awatism::Blo(len));
		}
		CoreValueKind::ModulePath(_path) => {
			instructions.push(Awatism::Nop);
		}
	}
	Ok(())
}

/// Convenience function: Compile AwaML source to AWASM binary object code
pub fn compile_to_binary(source: &str) -> Result<Vec<u8>, CompileError> {
	let awasm_instructions = compile_to_awasm(source)?;
	let instructions: Vec<crate::Instruction> = awasm_instructions
		.iter()
		.cloned()
		.map(|awatism| crate::Instruction { awatism })
		.collect();
	Ok(crate::assembler::make_object_vec(&instructions))
}

pub fn render_awasm_via_object(instructions: &[Awatism]) -> String {
	let object = crate::assembler::make_object_vec(
		&instructions
			.iter()
			.cloned()
			.map(|awatism| crate::Instruction { awatism })
			.collect::<Vec<_>>(),
	);
	crate::assembler::object_to_awasm(&object)
}

fn escape_awasm_string(value: &str) -> String {
	value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn try_render_macro_packet(
	instructions: &[Awatism],
	start: usize,
) -> Option<(usize, String)> {
	// !str "..."
	if let Awatism::Blo(_) = instructions.get(start)? {
		let mut bytes = Vec::new();
		let mut idx = start;
		while let Some(Awatism::Blo(b)) = instructions.get(idx) {
			bytes.push(*b);
			idx += 1;
		}
		if let Some(Awatism::Srn(n)) = instructions.get(idx) {
			let expected = bytes.len().min(u8::MAX as usize) as u8;
			let printable = bytes
				.get(..bytes.len().saturating_sub(1))
				.unwrap_or(&[])
				.iter()
				.all(|b| b.is_ascii_graphic() || *b == b' ');
			if *n == expected && printable && !bytes.is_empty() && bytes.last() == Some(&0) {
				let content = String::from_utf8_lossy(&bytes[..bytes.len() - 1]).to_string();
				return Some((idx + 1, format!("!str \"{}\"", escape_awasm_string(&content))));
			}
		}
	}

	// Typed packet: `Blo(tag)` + payloadBubble + `Srn(2)` where payloadBubble is:
	// - i32/f32: `Srn(4)` (payloadDouble)
	// - chr: single `Blo(payload)` (payloadSimple)
	// - str: a `Srn(n)` payloadDouble (no further `Mrg` for our short strings in fixtures)
	let tag = match instructions.get(start)? {
		Awatism::Blo(tag) => *tag,
		_ => return None,
	};

	match tag {
		0x0 => {
			// `Blo(0) + Blo[4] + Srn(4) + Srn(2)`
			let b0 = match instructions.get(start + 1) { Some(Awatism::Blo(b)) => *b, _ => return None };
			let b1 = match instructions.get(start + 2) { Some(Awatism::Blo(b)) => *b, _ => return None };
			let b2 = match instructions.get(start + 3) { Some(Awatism::Blo(b)) => *b, _ => return None };
			let b3 = match instructions.get(start + 4) { Some(Awatism::Blo(b)) => *b, _ => return None };
			match instructions.get(start + 5) { Some(Awatism::Srn(4)) => {} , _ => return None }
			match instructions.get(start + 6) { Some(Awatism::Srn(2)) => {} , _ => return None }
			let val = i32::from_le_bytes([b0, b1, b2, b3]);
			Some((start + 7, format!("!_i32 {val}")))
		}
		0x1 => {
			// awascii char: `Blo(1) + Blo(idx) + Srn(2)`
			let payload_idx = match instructions.get(start + 1) { Some(Awatism::Blo(b)) => *b, _ => return None };
			match instructions.get(start + 2) { Some(Awatism::Srn(2)) => {} , _ => return None }
			let ch = crate::AWA_SCII.chars().nth(payload_idx as usize)?;
			Some((start + 3, format!("!_chr a'{}'", ch)))
		}
		0x2 => {
			// ascii char: `Blo(2) + Blo(byte) + Srn(2)`
			let payload_byte = match instructions.get(start + 1) { Some(Awatism::Blo(b)) => *b, _ => return None };
			match instructions.get(start + 2) { Some(Awatism::Srn(2)) => {} , _ => return None }
			let ch = payload_byte as char;
			Some((start + 3, format!("!_chr '{}'", ch)))
		}
		0x3 | 0x4 => {
			// str payload (short fixture strings): `Blo(tag) + Blo[<=31] + Srn(n) + Srn(2)`
			// We only attempt the single-chunk case (no `Mrg`) here.
			let mut idx = start + 1;
			let mut payload_bytes: Vec<u8> = Vec::new();
			while let Some(Awatism::Blo(b)) = instructions.get(idx) {
				payload_bytes.push(*b);
				idx += 1;
				if payload_bytes.len() > 64 {
					return None;
				}
			}
			let n = match instructions.get(idx) { Some(Awatism::Srn(n)) => *n as usize, _ => return None };
			if n != payload_bytes.len() {
				return None;
			}
			match instructions.get(idx + 1) { Some(Awatism::Srn(2)) => {} , _ => return None }

			// Our payloadDouble is built so bytes appear in reverse chunk order.
			// For these short strings we reverse to recover original order.
			payload_bytes.reverse();

			let content = if tag == 0x3 {
				// awascii index to AWA_SCII char
				let mut s = String::new();
				for b in payload_bytes {
					let ch = crate::AWA_SCII.chars().nth(b as usize)?;
					s.push(ch);
				}
				s
			} else {
				let bytes = payload_bytes;
				bytes.iter().map(|b| *b as char).collect::<String>()
			};
			if tag == 0x3 {
				Some((idx + 2, format!("!_str a\"{}\"", escape_awasm_string(&content))))
			} else {
				Some((idx + 2, format!("!_str \"{}\"", escape_awasm_string(&content))))
			}
		}
		_ => None,
	}
}

pub fn render_awasm_macro_style(instructions: &[Awatism]) -> String {
	let mut out = Vec::new();
	let mut i = 0usize;
	while i < instructions.len() {
		if let Some((next, rendered)) = try_render_macro_packet(instructions, i) {
			out.push(format!("\t{rendered}"));
			i = next;
			continue;
		}
		match &instructions[i] {
			Awatism::StrLbl(label) => out.push(format!("{label}:")),
			Awatism::Nop => {}
			Awatism::Srn(arg) => out.push(format!("\tsrn {arg}")),
			Awatism::Lib => out.push("\tlib".to_string()),
			Awatism::Pr1 => out.push("\tpr1".to_string()),
			Awatism::Trm => out.push("\ttrm".to_string()),
			other => out.push(render_awasm(&[other.clone()])),
		}
		i += 1;
	}
	out.join("\n")
}

/// Render AWASM instructions as text (human-readable .awasm file format)
pub fn render_awasm(instructions: &[Awatism]) -> String {
	let mut result = String::new();
	for (idx, instr) in instructions.iter().enumerate() {
		if idx > 0 {
			result.push('\n');
		}
		let is_label = matches!(instr, Awatism::StrLbl(_));
		if !is_label {
			result.push('\t');
		}
		match instr {
			Awatism::Nop => result.push_str("nop"),
			Awatism::Prn => result.push_str("prn"),
			Awatism::Pr1 => result.push_str("pr1"),
			Awatism::Red => result.push_str("red"),
			Awatism::R3d => result.push_str("r3d"),
			Awatism::Blo(arg) => result.push_str(&format!("blo {}", arg)),
			Awatism::Sbm(arg) => result.push_str(&format!("sbm {}", arg)),
			Awatism::Pop => result.push_str("pop"),
			Awatism::Dpl => result.push_str("dpl"),
			Awatism::Srn(arg) => result.push_str(&format!("srn {}", arg)),
			Awatism::Mrg => result.push_str("mrg"),
			Awatism::Add => result.push_str("4dd"),
			Awatism::Sub => result.push_str("sub"),
			Awatism::Mul => result.push_str("mul"),
			Awatism::Div => result.push_str("div"),
			Awatism::Cnt => result.push_str("cnt"),
			Awatism::Lbl(arg) => result.push_str(&format!("lbl {}", arg)),
			Awatism::Jmp(arg) => result.push_str(&format!("jmp {}", arg)),
			Awatism::Eql => result.push_str("eql"),
			Awatism::Lss => result.push_str("lss"),
			Awatism::Gr8 => result.push_str("gr8"),
			Awatism::Lib => result.push_str("lib"),
			Awatism::Call(is_string, name) => {
				if *is_string {
					result.push_str(&format!("call \"{}\"", name));
				} else {
					result.push_str("call");
				}
			}
			Awatism::Ret => result.push_str("ret"),
			Awatism::Trm => result.push_str("trm"),
			Awatism::StrLbl(label) => result.push_str(&format!("{label}:")),
			Awatism::JmpRel => result.push_str("jro"),
			Awatism::JmpRelStr(label) => result.push_str(&format!("jro {}", label)),
		}
	}
	result
}

/// Compile AwaML source directly to AWASM text format
pub fn compile_and_render_awasm(source: &str) -> Result<String, CompileError> {
	let instructions = compile_to_awasm(source)?;
	let mut rendered = render_awasm_macro_style(&instructions);
	// For human-facing .awasm parity with checked-in examples, hide trailing terminator line.
	let mut lines: Vec<&str> = rendered.lines().collect();
	if lines.len() > 1 && matches!(lines.last(), Some(&"trm")) {
		lines.pop();
		rendered = lines.join("\n");
		if !rendered.is_empty() {
			rendered.push('\n');
		}
	}
	Ok(rendered)
}

