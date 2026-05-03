# AwaML Compiler

This directory contains the current AwaML frontend and backend pipeline:

- Lexer and parser (`lexer.rs`, `parser.rs`)
- AST and IR definitions (`ast.rs`, `ir.rs`)
- Lowering + AWASM codegen (`mod.rs`)

AwaML source files use the `.awaml` extension.

## Current Compiler Pipeline

```text
AwaML source (.awaml)
  -> parse_program()
  -> AstProgram
  -> type_check_program()      (currently stubbed)
  -> lower_core_program()
  -> CoreProgram { extern_sigs, functions }
  -> compile_to_awasm()
  -> Vec<Awatism>
  -> compile_to_binary() / compile_and_render_awasm()
```

Important details in current behavior:

- `Vec<Awatism>` is the codegen source of truth.
- `.awasm` text output can be rendered from the instruction stream via object encoding (`render_awasm_via_object`).
- Top-level codegen ends with `trm` and avoids unconditional raw `ret` insertion.
- Extern codegen emits symbol/arg frame structure and `lib` calls with typed tags.
- Literal parsing preserves `float`, `char`, `awachar` (`a'X'`), and `awastring` (`a"..."`) forms.

## Public API

### Frontend

- `parse_program(source) -> Result<AstProgram, ParseError>`
- `compile_to_core(source) -> Result<CoreProgram, CompileError>`
- `compile_and_render(source) -> Result<String, CompileError>` (Core-IR text)

### Backend

- `compile_to_awasm(source) -> Result<Vec<Awatism>, CompileError>`
- `render_awasm(instructions) -> String`
- `render_awasm_via_object(instructions) -> String`
- `compile_and_render_awasm(source) -> Result<String, CompileError>`
- `compile_to_binary(source) -> Result<Vec<u8>, CompileError>`

## CLI Usage (AwaML)

### File input mode

```bash
# Core-IR
cargo run -- examples/awaml/libfoo.awaml -o out.ir

# AWASM text
cargo run -- examples/awaml/libfoo.awaml -o out.awasm

# Object file
cargo run -- examples/awaml/libfoo.awaml -o out.o
```

### String/stdin mode

`--awaml` string mode now follows the same output extension dispatch as file mode:

- `.ir` -> `compile_and_render`
- `.awasm` -> `compile_and_render_awasm`
- `.o` -> `compile_to_binary`

```bash
cargo run -- --awaml -s "let x = 42;;" -o out.awasm
cargo run -- --awaml -s "let x = 42;;" -o out.o
```

## Testing

Run everything:

```bash
cargo test
```

Targeted suites:

```bash
cargo test --test lexer
cargo test --test parser
cargo test --test compiler_output
cargo test --test awasm_codegen
cargo test --test end_to_end
cargo test --test cli_awaml
```

What the tests cover now:

- Lexer/parser behavior and parse error shapes
- Core-IR structural output checks
- AWASM generation and render/object path parity checks
- Extern ABI codegen shape checks (symbol + typed arg framing + `lib`)
- End-to-end compile checks including `examples/awaml/raylib.awaml`
- CLI AwaML consistency between file and string modes
- Parser coverage for Awa literals and functor-apply include parsing

## Examples

Compiler examples:

- `examples/awaml/simple.awaml`
- `examples/awaml/libfoo.awaml`
- `examples/awaml/raylib.awaml`

Run raylib compile path checks:

```bash
cargo test --test end_to_end test_raylib_example_compiles_to_awasm_and_binary
```

## Limitations

Current known limitations:

- Type checking is still a stub (`type_check_program`).
- Lowering/codegen is still partial for full language semantics.
- Control-flow and expression lowering are scaffolded for coverage, not full optimization.
- Operator-call lowering (`+`/`-`/comparisons) and minimal `function`/`match` dispatch exist, but the full runtime-correct ABI/value model (stack-aware locals) is still evolving.
- Extern payload serialization is implemented for core constant cases and still evolving.
- `module type` signatures are still partially scaffolded even though module/type/functor shapes compile through Core-IR.

For ABI/reference docs, see:

- `docs/awaml-ir.md`
- `docs/awaml-extern.md`
- `docs/ffi-abi.md`

