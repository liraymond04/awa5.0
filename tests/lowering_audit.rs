use awa5_rs::compiler::{compile_to_awasm, compile_to_core};
use awa5_rs::compiler::ir::{CoreFunc, CoreProgram, CoreStmt, CoreValueKind, LocalId};
use awa5_rs::Awatism;

fn collect_called_extern_symbols(core: &CoreProgram) -> Vec<String> {
	let mut out = Vec::new();
	for func in &core.functions {
		for block in &func.blocks {
			for stmt in &block.stmts {
				if let CoreStmt::ExternCall { call, .. } = stmt {
					let sig = &core.extern_sigs[call.sig.0];
					out.push(sig.symbol_name.clone());
				}
			}
		}
	}
	out
}

fn collect_local_consts_in_fn(func: &CoreFunc) -> std::collections::HashMap<LocalId, CoreValueKind> {
	let mut out = std::collections::HashMap::new();
	for block in &func.blocks {
		for stmt in &block.stmts {
			if let CoreStmt::Let { dst, value } = stmt {
				out.insert(*dst, value.kind.clone());
			}
		}
	}
	out
}

fn find_first_extern_call_in_fn(func: &CoreFunc) -> Option<(usize, Vec<LocalId>)> {
	for block in &func.blocks {
		for stmt in &block.stmts {
			if let CoreStmt::ExternCall { call, .. } = stmt {
				return Some((call.sig.0, call.args.clone()));
			}
		}
	}
	None
}

#[test]
fn audit_if_expr_lowers_both_branches_now() {
	// Phase-2: `Expr::If` lowering should now lower `cond`, `then_branch`,
	// and `else_branch` (at least for emitting extern calls).
	let source = r#"
external foo : unit = "foo";;
external bar : unit = "bar";;
let x = if true then foo() else bar();;
"#;

	let core = compile_to_core(source).expect("compile_to_core should succeed");
	let called = collect_called_extern_symbols(&core);

	assert!(
		called.iter().any(|s| s == "foo"),
		"expected foo extern call to be present"
	);
	assert!(
		called.iter().any(|s| s == "bar"),
		"expected bar extern call to be present (else-branch lowered)"
	);
}

#[test]
fn audit_builtin_plus_does_not_lower_to_vm_add_yet() {
	// The parser builds `+` as an operator call shape, and builtin-op lowering
	// should now emit VM arithmetic ops.
	let source = r#"let x = 1 + 2;;"#;
	let awasm = compile_to_awasm(source).expect("compile_to_awasm should succeed");

	let has_arith = awasm.iter().any(|a| {
		matches!(
			a,
			Awatism::Add | Awatism::Sub | Awatism::Mul | Awatism::Div | Awatism::Eql | Awatism::Lss
				| Awatism::Gr8
		)
	});
	assert!(has_arith, "expected arithmetic VM ops from '+'");
}

#[test]
fn audit_tuple_expression_keeps_only_first_item_yet() {
	// Current scaffold: tuple expression lowering uses only the first element.
	let source = r#"
external foo : int -> unit = "foo";;
let x = foo((1, 2));;
"#;

	let core = compile_to_core(source).expect("compile_to_core should succeed");
	let func = core
		.functions
		.iter()
		.find(|f| f.name == "let_x")
		.expect("expected lowered function `let_x`");

	let local_consts = collect_local_consts_in_fn(func);
	let (_sig_id, args) = find_first_extern_call_in_fn(func).expect("expected an extern call");
	assert_eq!(args.len(), 1, "expected foo to receive one argument");

	let arg_local = args[0];
	let arg_kind = local_consts
		.get(&arg_local)
		.expect("expected argument local to have a known constant");
	assert!(
		matches!(arg_kind, CoreValueKind::ConstInt(1)),
		"expected tuple `(1,2)` to lower to argument constant 1 currently"
	);
}

#[test]
fn audit_member_access_callee_normalization_works_for_extern_lookup() {
	// Stage B: `R.iskeydown` style callees should resolve to extern named `iskeydown`.
	let source = r#"
external iskeydown : int -> bool = "iskeydown";;
let x = R.iskeydown 256;;
"#;

	let core = compile_to_core(source).expect("compile_to_core should succeed");
	let called = collect_called_extern_symbols(&core);
	assert!(
		called.iter().any(|s| s == "iskeydown"),
		"expected member-access callee to resolve to extern iskeydown"
	);
}

