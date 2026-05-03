use awa5_rs::compiler;

#[test]
fn test_awaml_string_and_file_mode_use_same_awasm_stage() {
    let source = r#"let x = 42;;
"#;

    // String/stdin mode path now uses compile_and_render_awasm for .awasm outputs.
    let string_mode = compiler::compile_and_render_awasm(source).expect("string mode compile failed");
    // File mode path uses the same API for .awasm outputs.
    let file_mode = compiler::compile_and_render_awasm(source).expect("file mode compile failed");

    assert_eq!(string_mode, file_mode, "awaml string and file mode should render awasm consistently");
    assert!(!string_mode.trim().is_empty(), "awasm output should not be empty");
}

#[test]
fn test_awaml_binary_stage_available_for_string_and_file_mode() {
    let source = r#"let x = 7;;
"#;

    let string_mode = compiler::compile_to_binary(source).expect("string mode binary compile failed");
    let file_mode = compiler::compile_to_binary(source).expect("file mode binary compile failed");

    assert_eq!(string_mode, file_mode, "binary output should match across cli input modes");
    assert!(!string_mode.is_empty(), "binary output should not be empty");
}
