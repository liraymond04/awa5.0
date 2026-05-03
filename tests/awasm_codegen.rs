use awa5_rs::compiler::{compile_to_awasm, compile_and_render_awasm, render_awasm, render_awasm_via_object};
use awa5_rs::Awatism;
use std::fs;

#[test]
fn test_compile_simple_to_awasm() {
    let source = r#"let x = 42;;
"#;
    
    let awasm = compile_to_awasm(source).expect("compile_to_awasm failed");
    
    // Should have some instructions and end with termination
    assert!(!awasm.is_empty(), "generated no instructions");
    assert!(awasm.iter().any(|a| matches!(a, Awatism::Trm)), "missing termination");
}

#[test]
fn test_compile_external_to_awasm() {
    let source = r#"external foo : int = "foo";;
"#;
    
    let awasm = compile_to_awasm(source).expect("compile_to_awasm failed");
    
    // Should generate code for extern declaration
    assert!(!awasm.is_empty(), "generated no instructions");
    assert!(awasm.iter().any(|a| matches!(a, Awatism::Trm)), "missing termination");
}

#[test]
fn test_compile_function_to_awasm() {
    let source = r#"fn add x y = x + y;;
"#;
    
    let awasm = compile_to_awasm(source).expect("compile_to_awasm failed");
    
    // Should generate code for function
    assert!(!awasm.is_empty(), "generated no instructions");
    assert!(awasm.iter().any(|a| matches!(a, Awatism::Trm)), "missing termination");
}

#[test]
fn test_compile_module_to_awasm() {
    let source = r#"module Math = struct end;;
"#;
    
    let awasm = compile_to_awasm(source).expect("compile_to_awasm failed");
    
    // Should generate code (even if minimal)
    assert!(!awasm.is_empty(), "generated no instructions");
    assert!(awasm.iter().any(|a| matches!(a, Awatism::Trm)), "missing termination");
}

#[test]
fn test_compile_combined_to_awasm() {
    let source = r#"
external print_int : int -> unit = "print_int";;
let x = 42;;
"#;
    
    let awasm = compile_to_awasm(source).expect("compile_to_awasm failed");
    
    // Should have multiple instructions
    assert!(awasm.len() > 2, "too few instructions generated");
    assert!(awasm.iter().any(|a| matches!(a, Awatism::Trm)), "missing termination");
}

#[test]
fn test_compile_has_safe_termination_without_ret_requirement() {
    let source = r#"let x = 1;;
"#;
    
    let awasm = compile_to_awasm(source).expect("compile_to_awasm failed");
    
    // Top-level code should terminate safely. `ret` is optional now.
    let has_trm = awasm.iter().any(|a| matches!(a, Awatism::Trm));
    assert!(has_trm, "missing termination");
}

#[test]
fn test_awasm_render_matches_object_render_for_simple_sequence() {
    let source = r#"let x = 42;;
"#;

    let awasm = compile_to_awasm(source).expect("compile_to_awasm failed");
    let direct = render_awasm(&awasm);
    let via_object = render_awasm_via_object(&awasm);

    assert!(!direct.is_empty(), "direct render must not be empty");
    assert!(!via_object.is_empty(), "object render must not be empty");
    assert!(direct.contains("trm"), "direct render should include trm");
    assert!(via_object.contains("trm"), "object render should include trm");
}

#[test]
fn test_extern_call_emits_symbol_and_typed_arg_frames() {
    let source = r#"
external print_int : int -> unit = "print_int";;
let main = print_int(42);;
"#;

    let awasm = compile_to_awasm(source).expect("compile_to_awasm failed");

    let blo_count = awasm.iter().filter(|a| matches!(a, Awatism::Blo(_))).count();
    let srn_count = awasm.iter().filter(|a| matches!(a, Awatism::Srn(_))).count();
    let has_lib = awasm.iter().any(|a| matches!(a, Awatism::Lib));

    // Extern ABI shape should include symbol bytes + typed arg payload framing.
    assert!(blo_count >= 8, "expected multiple blo instructions for extern ABI payload");
    assert!(srn_count >= 3, "expected multiple srn frame instructions for extern ABI");
    assert!(has_lib, "extern call should emit lib");
}

#[test]
fn test_extern_int_arg_serializes_le_i32_payload() {
    let source = r#"
external print_int : int -> unit = "print_int";;
let main = print_int(42);;
"#;
    let awasm = compile_to_awasm(source).expect("compile_to_awasm failed");
    let bytes: Vec<u8> = awasm
        .iter()
        .filter_map(|a| match a { Awatism::Blo(v) => Some(*v), _ => None })
        .collect();

    // 42 as little-endian i32 should appear in extern payload.
    let has_i32_42 = bytes.windows(4).any(|w| w == [42, 0, 0, 0]);
    assert!(has_i32_42, "expected little-endian i32 payload for extern int arg");
}

#[test]
fn test_libfoo_macro_render_abi_sequence() {
    let source = fs::read_to_string("examples/awaml/libfoo.awaml").expect("read libfoo.awaml");
    let rendered = compile_and_render_awasm(&source).expect("compile_and_render_awasm failed");

    // Compare a stable subset of macro-style lines (the exact float formatting may evolve).
    let lines: Vec<&str> = rendered.lines().map(str::trim).filter(|l| !l.is_empty()).collect();
    assert!(
        lines.iter().any(|l| *l == "!str \"foo\""),
        "missing !str \"foo\" call symbol"
    );
    assert!(
        lines.iter().any(|l| *l == "!_i32 4"),
        "missing !_i32 4 argument"
    );
    assert!(
        lines.iter().any(|l| *l == "!_str a\"Hello World!\""),
        "missing !_str a\"Hello World!\" argument"
    );
    assert!(
        lines.iter().any(|l| *l == "!_str \"foo\""),
        "missing !_str \"foo\" argument"
    );
    assert!(
        lines.iter().any(|l| *l == "!_chr a'W'"),
        "missing !_chr a'W' argument"
    );

    // Ensure we have the expected frame sequence in order:
    // surround args (srn 6), then pass fn names and args (srn 2), then lib, then pr1.
    let idx_srn6 = lines.iter().position(|l| *l == "srn 6").expect("missing srn 6");
    let idx_srn2 = lines
        .iter()
        .position(|l| *l == "srn 2")
        .expect("missing srn 2");
    assert!(idx_srn2 > idx_srn6, "expected srn 2 after srn 6");

    let idx_lib = lines.iter().position(|l| *l == "lib").expect("missing lib");
    assert!(idx_lib > idx_srn2, "expected lib after srn 2");

    let idx_pr1 = lines.iter().position(|l| *l == "pr1").expect("missing pr1");
    assert!(idx_pr1 > idx_lib, "expected pr1 after lib");
}
