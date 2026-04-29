use awa5_rs::compiler::compile_and_render;
use std::fs;

#[test]
fn test_compile_and_render_golden() {
    let source = r#"
module M = struct end;;
include Foo;;
extern foo : int = "foo";;
let x = 1;;
fn f x = x;;
"#;

    let rendered = compile_and_render(source).expect("compile_and_render failed");
    let expected = include_str!("fixtures/inline_expected.ir");
    assert_eq!(rendered, expected);
}

#[test]
fn test_compile_simple_awaml_file() {
    let source = fs::read_to_string("examples/awaml/simple.awaml")
        .expect("failed to read simple.awaml");
    
    let rendered = compile_and_render(&source).expect("compile_and_render failed");
    let expected = include_str!("fixtures/simple_expected.ir");
    
    assert_eq!(rendered, expected, "simple.awaml output mismatch");
}

#[test]
fn test_compile_libfoo_awaml_file() {
    let source = fs::read_to_string("examples/awaml/libfoo.awaml")
        .expect("failed to read libfoo.awaml");
    
    let rendered = compile_and_render(&source).expect("compile_and_render failed");
    let expected = include_str!("fixtures/libfoo_expected.ir");
    
    assert_eq!(rendered, expected, "libfoo.awaml output mismatch");
}

#[test]
fn test_compile_minimal_external() {
    let source = r#"external foo : int -> unit = "foo";;
"#;
    
    let rendered = compile_and_render(source).expect("compile_and_render failed");
    
    // Should contain the extern function declaration
    assert!(rendered.contains("Func extern_foo()"), "missing extern_foo function");
    assert!(rendered.contains("return"), "missing return statement");
}

#[test]
fn test_compile_let_binding() {
    let source = r#"let x = 42;;
"#;
    
    let rendered = compile_and_render(source).expect("compile_and_render failed");
    
    // Should contain a let binding function
    assert!(rendered.contains("Func let_x()"), "missing let_x function");
    assert!(rendered.contains("let %0 = 42 : I32"), "missing let binding with constant");
}

#[test]
fn test_compile_function_definition() {
    let source = r#"fn add x y = x + y;;
"#;
    
    let rendered = compile_and_render(source).expect("compile_and_render failed");
    
    // Should contain function declaration with parameters
    assert!(rendered.contains("Func add("), "missing add function");
}

#[test]
fn test_compile_module_declaration() {
    let source = r#"module Math = struct end;;
"#;
    
    let rendered = compile_and_render(source).expect("compile_and_render failed");
    
    // Should contain module declaration
    assert!(rendered.contains("Func module_Math()"), "missing module declaration");
}

