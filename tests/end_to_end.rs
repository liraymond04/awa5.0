/// Integration test demonstrating end-to-end AwaML → Core-IR → AWASM compilation
use awa5_rs::compiler::{compile_to_awasm, compile_to_core, compile_and_render, compile_and_render_awasm};
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
    assert!(awasm.len() > 2, "should produce multiple instructions");
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

#[test]
fn test_raylib_example_compiles_to_awasm_and_binary() {
    let source = fs::read_to_string("examples/awaml/raylib.awaml")
        .expect("failed to read raylib.awaml");

    let awasm = compile_to_awasm(&source).expect("compile_to_awasm failed for raylib");
    assert!(!awasm.is_empty(), "raylib awasm must not be empty");
    assert!(awasm.iter().any(|a| matches!(a, Awatism::Trm)), "raylib should terminate");
    assert!(
        awasm.iter().any(|a| matches!(a, Awatism::Lib)),
        "raylib should emit at least one lib extern call"
    );

    let binary = awa5_rs::compiler::compile_to_binary(&source).expect("compile_to_binary failed for raylib");
    assert!(!binary.is_empty(), "raylib binary output must not be empty");
}

#[test]
fn test_raylib_fixture_symbols_present_in_macro_render() {
    // Phase-7 structural fixture smoke:
    // ensure we emit the major raylib extern calls (not just the initial two),
    // and that conditional-skip based `if` codegen shows up (eql + jump).
    let source = fs::read_to_string("examples/awaml/raylib.awaml")
        .expect("failed to read raylib.awaml");

    let rendered = compile_and_render_awasm(&source).expect("compile_and_render_awasm failed for raylib");
    let fixture = fs::read_to_string("examples/awasm/raylib.awasm").expect("failed to read raylib.awasm fixture");

    for sym in [
        "initwindow",
        "settargetfps",
        "BeginDrawing",
        "clearbackground",
        "drawtext",
        "drawcircle",
        "EndDrawing",
        "iskeydown",
    ] {
        assert!(
            rendered.contains(&format!("!str \"{}\"", sym)),
            "missing extern symbol `{}' in raylib macro render",
            sym
        );
    }

    // Ensure we emit more than just initwindow/settargetfps.
    let lib_count = rendered.lines().filter(|l| l.trim() == "lib").count();
    assert!(lib_count >= 6, "expected more than a couple `lib` calls, got {}", lib_count);

    // Ensure the conditional-skip idiom is present (we emit `blo 0`, `eql`, then `jro`).
    assert!(
        rendered.contains("\n\tblo 0\n")
            && rendered.contains("\n\teql\n")
            && rendered.contains("\n\tjro "),
        "expected conditional-skip pattern (`blo 0` + `eql` + `jro ...`) to appear"
    );

    // Ensure operator lowering shows up (raylib uses arithmetic/comparisons in the step/update logic).
    let add_count = rendered.lines().filter(|l| l.trim() == "4dd").count();
    let sub_count = rendered.lines().filter(|l| l.trim() == "sub").count();
    let eql_count = rendered.lines().filter(|l| l.trim() == "eql").count();
    assert!(add_count >= 1, "expected `4dd` (add) to appear, got {add_count}");
    assert!(sub_count >= 1, "expected `sub` to appear, got {sub_count}");
    assert!(eql_count >= 1, "expected `eql` to appear, got {eql_count}");

    // Ensure keycode-of-key table dispatch is producing the expected numeric payloads.
    for k in ["87", "65", "83", "68", "256"] {
        assert!(
            rendered.contains(&format!("!_i32 {k}")),
            "missing keycode payload `!_i32 {k}` in raylib macro render"
        );
    }

    // ABI packing for `drawcircle`'s x/y comes from dynamic locals (stack plumbing),
    // not from the static `_i32 78/79` placeholders we previously emitted.
    if let Some(idx) = rendered.find("!str \"drawcircle\"") {
        let window = &rendered[idx..std::cmp::min(rendered.len(), idx + 1200)];
        assert!(
            !window.contains("!_i32 78") && !window.contains("!_i32 79"),
            "expected drawcircle x/y locals to be serialized dynamically (no `_i32 78/79` placeholders)"
        );

		// Be tolerant of whitespace: check the line neighborhood instead of exact substring spacing.
		let lines: Vec<&str> = rendered.lines().collect();
		let draw_line_idx = lines.iter().position(|l| l.contains("!str \"drawcircle\"")).unwrap_or(0);
		let after_draw = lines[draw_line_idx..std::cmp::min(lines.len(), draw_line_idx + 40)].join("\n");
        assert!(
            after_draw.contains("\tdpl")
                && after_draw.contains("\tsbm 1")
                && after_draw.contains("\tsrn 2"),
            "expected drawcircle ABI argument stack plumbing (`dpl` + `sbm 1` + `srn 2`) to appear"
        );
    }

    // Fixture sanity: ensure our test fixture itself still contains the expected conditional-skip opcode.
    assert!(
        fixture.contains("eql"),
        "fixture raylib.awasm should contain `eql`"
    );
}

