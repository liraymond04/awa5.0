use awa5_rs::compiler::compile_and_render_awasm;

#[test]
fn test_small_if_dynamic_condition_emits_jro_and_labels() {
    let source = r#"
external iskeydown : int -> bool = "iskeydown";;
external foo : unit = "foo";;
external bar : unit = "bar";;
let main = if iskeydown(1) then foo() else bar();;
"#;

    let rendered = compile_and_render_awasm(source).expect("compile_and_render_awasm failed");

    assert!(rendered.contains("let_main_bb0:"), "missing let_main_bb0 label");
    assert!(rendered.contains("let_main_bb1:"), "missing then block label");
    assert!(rendered.contains("let_main_bb2:"), "missing else block label");
    assert!(rendered.contains("\n\tblo 0\n"), "expected `blo 0` in if lowering");
    assert!(rendered.contains("\n\teql\n"), "expected `eql` in if lowering");
    assert!(rendered.contains("\n\tjro let_main_bb"), "expected jro to labeled else block");
}

#[test]
fn test_render_indents_instructions_under_labels() {
    let source = r#"
external foo : unit = "foo";;
let main = foo();;
"#;
    let rendered = compile_and_render_awasm(source).expect("compile_and_render_awasm failed");
    let lines: Vec<&str> = rendered.lines().collect();

    assert!(
        lines.iter().any(|l| l.ends_with(':')),
        "expected at least one label line ending with ':'"
    );
    assert!(
        lines.iter().any(|l| l.starts_with('\t') && !l.trim().is_empty()),
        "expected at least one instruction line with tab indentation"
    );
}

#[test]
fn test_handwritten_if_codegen_matches_expected_labels_and_jro() {
    let source = r#"
external cond : int -> bool = "iskeydown";;
external thenf : unit = "thenf";;
external elsef : unit = "elsef";;
let main = if cond(1) then thenf() else elsef();;
"#;

    let rendered = compile_and_render_awasm(source).expect("compile_and_render_awasm failed");
    let lines: Vec<&str> = rendered.lines().collect();
    let start = lines
        .iter()
        .position(|l| *l == "let_main_bb0:")
        .expect("missing let_main_bb0 label");
    let end = lines
        .iter()
        .position(|l| *l == "let_main_end:")
        .expect("missing let_main_end label");

    let region = lines[start..=end].join("\n");
    let expected = r#"let_main_bb0:
	!str "iskeydown"
	!_i32 1
	srn 1
	srn 2
	lib
	blo 0
	eql
	jro let_main_bb2
let_main_bb1:
	!str "thenf"
	srn 1
	lib
	jro let_main_end
let_main_bb2:
	!str "elsef"
	srn 1
	lib
	jro let_main_end
let_main_end:"#;

    assert_eq!(region, expected, "conditional codegen no longer matches handwritten expected AWASM");
}

#[test]
fn test_handwritten_nested_if_codegen_matches_expected_labels_and_jro() {
    let source = r#"
external cond : int -> bool = "iskeydown";;
external a : unit = "a";;
external b : unit = "b";;
external c : unit = "c";;
let main =
  if cond(1) then
    if cond(2) then a() else b()
  else
    c();;
"#;

    let rendered = compile_and_render_awasm(source).expect("compile_and_render_awasm failed");
    let lines: Vec<&str> = rendered.lines().collect();
    let start = lines
        .iter()
        .position(|l| *l == "let_main_bb0:")
        .expect("missing let_main_bb0 label");
    let end = lines
        .iter()
        .position(|l| *l == "let_main_end:")
        .expect("missing let_main_end label");

    let region = lines[start..=end].join("\n");
    let expected = r#"let_main_bb0:
	!str "iskeydown"
	!_i32 1
	srn 1
	srn 2
	lib
	blo 0
	eql
	jro let_main_bb2
let_main_bb1:
	!str "iskeydown"
	!_i32 2
	srn 1
	srn 2
	lib
	blo 0
	eql
	jro let_main_bb4
let_main_bb2:
	!str "c"
	srn 1
	lib
	jro let_main_end
let_main_bb3:
	!str "a"
	srn 1
	lib
	jro let_main_end
let_main_bb4:
	!str "b"
	srn 1
	lib
	jro let_main_end
let_main_end:"#;

    assert_eq!(
        region, expected,
        "nested conditional codegen no longer matches handwritten expected AWASM"
    );
}

#[test]
fn test_handwritten_if_with_unit_branch_matches_expected_labels_and_jro() {
    let source = r#"
external cond : int -> bool = "iskeydown";;
external tick : unit = "tick";;
let main = if cond(1) then tick() else ();;
"#;

    let rendered = compile_and_render_awasm(source).expect("compile_and_render_awasm failed");
    let lines: Vec<&str> = rendered.lines().collect();
    let start = lines
        .iter()
        .position(|l| *l == "let_main_bb0:")
        .expect("missing let_main_bb0 label");
    let end = lines
        .iter()
        .position(|l| *l == "let_main_end:")
        .expect("missing let_main_end label");

    let region = lines[start..=end].join("\n");
    let expected = r#"let_main_bb0:
	!str "iskeydown"
	!_i32 1
	srn 1
	srn 2
	lib
	blo 0
	eql
	jro let_main_bb2
let_main_bb1:
	!str "tick"
	srn 1
	lib
	jro let_main_end
let_main_bb2:
	jro let_main_end
let_main_end:"#;

    assert_eq!(
        region, expected,
        "if-with-unit-else codegen no longer matches handwritten expected AWASM"
    );
}

#[test]
fn test_handwritten_module_functor_include_shape_matches_expected_awasm() {
    let source = r#"
module type S = sig val ping : unit -> unit end;;
module M = struct external ping : unit -> unit = "ping";; end;;
module F = functor (R : S) -> struct let main = R.ping() ;; end;;
include F(M);;
"#;

    let rendered = compile_and_render_awasm(source).expect("compile_and_render_awasm failed");
    let expected = r#"	jro fn_4
fn_0:
module_type_S_bb0:
	jro module_type_S_end
module_type_S_end:
fn_1:
module_M_bb0:
	jro module_M_end
module_M_end:
fn_2:
module_F_bb0:
	jro module_F_end
module_F_end:
fn_3:
include_F(M)_bb0:
	jro include_F(M)_end
include_F(M)_end:
fn_4:
let_main_bb0:
	!str "ping"
	srn 1
	lib
	jro let_main_end
let_main_end:"#;

    let lines: Vec<&str> = rendered.lines().collect();
    let expected_lines: Vec<&str> = expected.lines().collect();
    assert_eq!(
        &lines[..expected_lines.len()],
        &expected_lines[..],
        "module/functor/include shape no longer matches handwritten expected AWASM prefix"
    );
}

#[test]
fn test_handwritten_let_rec_shape_matches_expected_current_scaffold() {
    let source = r#"
external tick : unit = "tick";;
let rec loop x = if x == 0 then () else let () = tick() in loop (x - 1);;
let main = loop 2;;
"#;

    let rendered = compile_and_render_awasm(source).expect("compile_and_render_awasm failed");
    let lines: Vec<&str> = rendered.lines().collect();
    let start = lines
        .iter()
        .position(|l| *l == "let_loop_bb0:")
        .expect("missing let_loop_bb0 label");
    let end = lines
        .iter()
        .position(|l| *l == "let_loop_end:")
        .expect("missing let_loop_end label");
    let region = lines[start..=end].join("\n");

    let expected = r#"let_loop_bb0:
	blo 0
	eql
	jro let_loop_bb2
let_loop_bb1:
	jro let_loop_end
let_loop_bb2:
	!str "tick"
	srn 1
	lib
	dpl
	blo 1
	sub
	jro let_loop_end
let_loop_end:"#;
    assert_eq!(
        region, expected,
        "let rec scaffold shape changed; update lowering or this expected snapshot intentionally"
    );
}

#[test]
fn test_handwritten_nested_let_chain_matches_expected_awasm() {
    let source = r#"
external foo : int -> unit = "foo";;
let main = let a = 1 in let b = a + 2 in let c = b + 3 in foo(c);;
"#;

    let rendered = compile_and_render_awasm(source).expect("compile_and_render_awasm failed");
    let lines: Vec<&str> = rendered.lines().collect();
    let start = lines
        .iter()
        .position(|l| *l == "let_main_bb0:")
        .expect("missing let_main_bb0 label");
    let end = lines
        .iter()
        .position(|l| *l == "let_main_end:")
        .expect("missing let_main_end label");
    let region = lines[start..=end].join("\n");

    let expected = r#"let_main_bb0:
	blo 1
	blo 2
	4dd
	blo 3
	blo 3
	4dd
	!str "foo"
	!_i32 6
	srn 1
	srn 2
	lib
	jro let_main_end
let_main_end:"#;
    assert_eq!(
        region, expected,
        "nested let-in chain no longer matches handwritten expected AWASM"
    );
}

#[test]
fn test_handwritten_rec_tuple_branch_shape_matches_expected_current_scaffold() {
    let source = r#"
let rec step x y = if x == 0 then (x, y) else (x - 1, y + 1);;
let main = let (a,b) = step 2 3 in if a == b then (a,b) else (b,a);;
"#;

    let rendered = compile_and_render_awasm(source).expect("compile_and_render_awasm failed");
    let lines: Vec<&str> = rendered.lines().collect();

    let step_start = lines
        .iter()
        .position(|l| *l == "let_step_bb0:")
        .expect("missing let_step_bb0 label");
    let step_end = lines
        .iter()
        .position(|l| *l == "let_step_end:")
        .expect("missing let_step_end label");
    let step_region = lines[step_start..=step_end].join("\n");

    let main_start = lines
        .iter()
        .position(|l| *l == "let_main_bb0:")
        .expect("missing let_main_bb0 label");
    let main_end = lines
        .iter()
        .position(|l| *l == "let_main_end:")
        .expect("missing let_main_end label");
    let main_region = lines[main_start..=main_end].join("\n");

    let expected_step = r#"let_step_bb0:
	blo 0
	eql
	jro let_step_bb2
let_step_bb1:
	jro let_step_end
let_step_bb2:
	dpl
	blo 1
	sub
	jro let_step_end
let_step_end:"#;
    let expected_main = r#"let_main_bb0:
	blo 0
	eql
	jro let_main_bb2
let_main_bb1:
	jro let_main_end
let_main_bb2:
	jro let_main_end
let_main_end:"#;

    assert_eq!(
        step_region, expected_step,
        "recursive tuple function scaffold shape changed unexpectedly"
    );
    assert_eq!(
        main_region, expected_main,
        "main tuple/branch scaffold shape changed unexpectedly"
    );
}

#[test]
fn test_print_and_println_intrinsics_emit_print_ops_and_newline() {
    let source = r#"
let main = let x = 42 in let _ = print x in println x;;
"#;
    let rendered = compile_and_render_awasm(source).expect("compile_and_render_awasm failed");

    // `print` and `println` should emit print ops; println should also emit newline char + prn.
    assert!(
        rendered.contains("\tpr1"),
        "expected numeric print op (`pr1`) in print/println lowering"
    );
    let nl_idx = awa5_rs::AWA_SCII
        .chars()
        .position(|c| c == '\n')
        .expect("AWA_SCII should contain newline");
    assert!(
        rendered.contains(&format!("\tblo {nl_idx}\n\tprn")),
        "expected println lowering to append newline (`blo {nl_idx}` + `prn`)"
    );
}
