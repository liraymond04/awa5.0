#[path = "support/runtime_harness.rs"]
mod runtime_harness;

fn assert_successful_run(output: &std::process::Output) {
    assert!(
        output.status.success(),
        "interpreter execution failed: status={:?}, stderr={}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn baseline_non_recursive_state_threading_behavior_matches_expected_stdout() {
    let output = runtime_harness::run_awaml_fixture("tests/fixtures/awaml/baseline_state_threading.awaml");
    assert_successful_run(&output);
    let got = runtime_harness::stdout_string(&output);
    let expected = runtime_harness::read_fixture("tests/fixtures/expected/baseline_state_threading_prn.txt");
    assert_eq!(got, expected, "baseline behavior mismatch");
}

#[test]
fn recursive_tail_counter_behavior_matches_expected_stdout() {
    let output = runtime_harness::run_awaml_fixture("tests/fixtures/awaml/rec_tail_counter.awaml");
    assert_successful_run(&output);
    let got = runtime_harness::stdout_string(&output);
    let expected = runtime_harness::read_fixture("tests/fixtures/expected/rec_tail_counter_prn.txt");
    assert_eq!(
        got, expected,
        "tail recursion semantic mismatch (likely recursive call lowering/rematerialization gap)"
    );
}

#[test]
fn recursive_tuple_state_threading_behavior_matches_expected_stdout() {
    let output = runtime_harness::run_awaml_fixture("tests/fixtures/awaml/rec_tuple_state.awaml");
    assert_successful_run(&output);
    let got = runtime_harness::stdout_string(&output);
    let expected = runtime_harness::read_fixture("tests/fixtures/expected/rec_tuple_state_prn.txt");
    assert_eq!(
        got, expected,
        "tuple recursion semantic mismatch (likely tuple return/destructure threading gap)"
    );
}

#[test]
fn recursive_branch_both_paths_behavior_matches_expected_stdout() {
    let output = runtime_harness::run_awaml_fixture("tests/fixtures/awaml/rec_branch_both.awaml");
    assert_successful_run(&output);
    let got = runtime_harness::stdout_string(&output);
    let expected = runtime_harness::read_fixture("tests/fixtures/expected/rec_branch_both_prn.txt");
    assert_eq!(
        got, expected,
        "branch-recursion semantic mismatch (likely join rematerialization + recursive call lowering gap)"
    );
}

#[test]
fn recursive_counter_prints_incrementing_sequence() {
    let output = runtime_harness::run_awaml_fixture("tests/fixtures/awaml/rec_print_counter.awaml");
    assert_successful_run(&output);
    let got = runtime_harness::stdout_string(&output);
    let expected = runtime_harness::read_fixture("tests/fixtures/expected/rec_print_counter_stdout.txt");
    assert_eq!(
        got, expected,
        "recursive counter println output mismatch (state threading/recursive call semantics gap)"
    );
}

#[test]
fn recursive_fibonacci_prints_expected_sequence() {
    let output = runtime_harness::run_awaml_fixture("tests/fixtures/awaml/rec_print_fib.awaml");
    assert_successful_run(&output);
    let got = runtime_harness::stdout_string(&output);
    let expected = runtime_harness::read_fixture("tests/fixtures/expected/rec_print_fib_stdout.txt");
    assert_eq!(
        got, expected,
        "recursive fibonacci println output mismatch (recursive semantics/codegen gap)"
    );
}
