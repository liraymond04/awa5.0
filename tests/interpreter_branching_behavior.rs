use std::fs;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn handwritten_awasm_branching_runs_expected_path_with_prn() {
    // Handwritten control-flow sample:
    // - print `W` (blo 1 => AWA_SCII[1] == 'W')
    // - jump over the second print
    // - terminate
    let awasm = r#"entry:
	blo 1
	prn
	jro done
	blo 0
	prn
done:
	trm
"#;

    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock error")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("awa5_rs_branching_{ts}.awasm"));
    fs::write(&path, awasm).expect("failed to write temporary awasm file");

    let exe = env!("CARGO_BIN_EXE_awa5_rs");
    let output = Command::new(exe)
        .arg(path.to_string_lossy().as_ref())
        .output()
        .expect("failed to execute awa5_rs binary");

    let _ = fs::remove_file(&path);

    assert!(
        output.status.success(),
        "interpreter run failed: status={:?}, stderr={}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains('W'),
        "expected taken branch to print 'W', got stdout={stdout:?}"
    );
    assert!(
        !stdout.contains('A'),
        "expected skipped branch not to print 'A', got stdout={stdout:?}"
    );
}
