use std::fs;
use std::path::Path;
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn read_fixture(path: &str) -> String {
    fs::read_to_string(path).unwrap_or_else(|e| panic!("failed reading fixture {path}: {e}"))
}

pub fn compile_awaml_to_awasm_text(source: &str) -> String {
    awa5_rs::compiler::compile_and_render_awasm(source)
        .unwrap_or_else(|e| panic!("compile_and_render_awasm failed: {e:?}"))
}

pub fn run_awasm_text_via_cli(awasm_text: &str) -> Output {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock error")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("awa5_rs_runtime_{ts}.awasm"));
    fs::write(&path, awasm_text).unwrap_or_else(|e| panic!("failed writing temp awasm: {e}"));

    let exe = env!("CARGO_BIN_EXE_awa5_rs");
    let output = Command::new(exe)
        .arg(path.to_string_lossy().as_ref())
        .output()
        .unwrap_or_else(|e| panic!("failed executing awa5_rs: {e}"));

    let _ = fs::remove_file(&path);
    output
}

pub fn run_awaml_fixture<P: AsRef<Path>>(awaml_fixture: P) -> Output {
    let source = fs::read_to_string(awaml_fixture.as_ref())
        .unwrap_or_else(|e| panic!("failed reading awaml fixture {}: {e}", awaml_fixture.as_ref().display()));
    let awasm = compile_awaml_to_awasm_text(&source);
    run_awasm_text_via_cli(&awasm)
}

pub fn stdout_string(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).to_string()
}
