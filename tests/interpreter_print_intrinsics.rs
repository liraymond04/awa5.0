#[path = "support/runtime_harness.rs"]
mod runtime_harness;

#[test]
fn print_and_println_awaml_intrinsics_produce_observable_output() {
    let source = r#"
let main = let x = 5 in let _ = print x in println x;;
"#;
    let awasm = awa5_rs::compiler::compile_and_render_awasm(source)
        .expect("compile_and_render_awasm failed");
    let output = runtime_harness::run_awasm_text_via_cli(&awasm);

    assert!(
        output.status.success(),
        "interpreter execution failed: status={:?}, stderr={}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = runtime_harness::stdout_string(&output);
    assert!(
        stdout.contains("5"),
        "expected printed numeric output to contain '5', got stdout={stdout:?}"
    );
    assert!(
        stdout.contains('\n'),
        "expected println to include newline, got stdout={stdout:?}"
    );
}
