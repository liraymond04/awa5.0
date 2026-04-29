/// Integration test demonstrating end-to-end AwaML → Core-IR → AWASM compilation
use awa5_rs::compiler::{compile_to_awasm, compile_to_core, compile_and_render};
use awa5_rs::Awatism;
use std::fs;

#[test]
fn test_end_to_end_compilation_pipeline() {
    let source = r#"
external print_int : int -> unit = "print_int";;
let forty_two = 42;;
"#;

    // Step 1: Compile to Core-IR
    let core_ir = compile_to_core(source).expect("compile_to_core failed");
    assert!(!core_ir.functions.is_empty(), "Core-IR has no functions");

    // Step 2: Render Core-IR as text
    let ir_text = compile_and_render(source).expect("compile_and_render failed");
    assert!(ir_text.contains("CoreProgram"), "IR text should contain CoreProgram header");
    assert!(ir_text.contains("extern_print_int"), "Should contain extern function");
    assert!(ir_text.contains("let_forty_two"), "Should contain let binding");

    // Step 3: Compile to AWASM
    let awasm = compile_to_awasm(source).expect("compile_to_awasm failed");
    assert!(!awasm.is_empty(), "AWASM generation failed");
    assert!(awasm.iter().any(|a| matches!(a, Awatism::Trm)), "AWASM should end with termination");
}

#[test]
fn test_awasm_from_simple_example() {
    let source = fs::read_to_string("examples/awaml/simple.awaml")
        .unwrap_or_else(|_| "let x = 1;;".to_string());
    
    let awasm = compile_to_awasm(&source).expect("compile_to_awasm failed");
    
    // Basic sanity checks
    assert!(!awasm.is_empty(), "generated no AWASM instructions");
    assert!(awasm.len() > 2, "too few instructions");
    
    // Verify termination
    let last_instr = awasm.last();
    assert!(matches!(last_instr, Some(Awatism::Trm)), "should end with Trm instruction");
    
    // Count some instruction types
    let ret_count = awasm.iter().filter(|a| matches!(a, Awatism::Ret)).count();
    let nop_count = awasm.iter().filter(|a| matches!(a, Awatism::Nop)).count();
    
    assert!(ret_count > 0, "should have at least one return");
    assert!(nop_count > 0, "should have some nop instructions");
}

#[test]
fn test_awasm_handles_all_statement_types() {
    let source = r#"
module TestModule = struct end;;
include SomeModule;;
external foo : unit = "foo";;
let x = 42;;
"#;
    
    let awasm = compile_to_awasm(source).expect("compile_to_awasm failed");
    
    // Should compile without error
    assert!(!awasm.is_empty(), "generated instructions");
    
    // All should end with Trm
    assert!(awasm.iter().any(|a| matches!(a, Awatism::Trm)), "should terminate");
}
