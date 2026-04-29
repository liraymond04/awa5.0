use awa5_rs::compiler::compile_to_awasm;
use awa5_rs::Awatism;

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
fn test_compile_includes_return() {
    let source = r#"let x = 1;;
"#;
    
    let awasm = compile_to_awasm(source).expect("compile_to_awasm failed");
    
    // Should include return before termination
    let has_ret = awasm.iter().any(|a| matches!(a, Awatism::Ret));
    let has_trm = awasm.iter().any(|a| matches!(a, Awatism::Trm));
    assert!(has_ret, "missing return");
    assert!(has_trm, "missing termination");
}
