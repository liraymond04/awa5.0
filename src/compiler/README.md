# AwaML compiler

This module contains the parser, AST, IR, and code generation for AwaML, the source language for this project.

AwaML source files use the `.awaml` extension.

## Compilation Pipeline

The compiler implements a complete pipeline from AwaML source to AWASM bytecode:

```
AwaML source (.awaml)
    ↓ parse_program()
AST (Abstract Syntax Tree)
    ↓ type_check_program() [stub]
    ↓ lower_core_program()
Core-IR (Control-flow IR)
    ↓ compile_to_awasm()
AWASM instructions (Awatism)
    ↓ assembler::make_object_vec()
Binary object code
    ↓ interpreter::interpet_object()
Execution
```

## Public API Functions

### Parsing
- **`parse_program(source: &str) -> Result<AstProgram, ParseError>`** - Parse AwaML source to AST
- **`parse_item(source: &str) -> Result<AstItem, ParseError>`** - Parse a single item

### Type Checking (Stub)
- **`type_check_program(program: &AstProgram) -> Result<(), CompileError>`** - Placeholder for semantic analysis

### Lowering to Core-IR
- **`compile_to_core(source: &str) -> Result<CoreProgram, CompileError>`** - Full parse + lower to Core-IR
- **`lower_core_program(program: &AstProgram) -> Result<CoreProgram, CompileError>`** - Convert AST to Core-IR

### Code Generation
- **`compile_to_awasm(source: &str) -> Result<Vec<Awatism>, CompileError>`** - Full AwaML → AWASM compilation
- **`compile_and_render(source: &str) -> Result<String, CompileError>`** - Compile and render IR as text (for debugging)

## Testing

### Run All Tests
```bash
cargo test -q
```

Test suite includes:
- **Lexer tests** (2) - Tokenization with OCaml-style comments
- **Parser tests** (15) - AST structure, expressions, patterns, error handling
- **Compiler output tests** (7) - Golden tests comparing Core-IR output
- **AWASM codegen tests** (6) - Bytecode generation validation
- **End-to-end tests** (3) - Full compilation pipeline verification

### Run Specific Test Suite
```bash
cargo test --test lexer -q
cargo test --test parser -q
cargo test --test compiler_output -q
cargo test --test awasm_codegen -q
cargo test --test end_to_end -q
```

## Usage Examples

### Compile AwaML File to Core-IR (Text)
```bash
cargo run --quiet -- examples/awaml/simple.awaml --awaml
```

Output:
```
CoreProgram
  Func extern_print_int() -> Unit
    Block 0
      return
  Func let_add() -> Unit
    Block 0
      let %0 = ... : ...
      return %0
  ...
```

### Compile and Save Core-IR
```bash
cargo run --quiet -- examples/awaml/libfoo.awaml -o output.ir --awaml
```

### Compile from String
```bash
cargo run --quiet -- -s "let x = 42;;" --awaml
```

### Programmatic Usage
```rust
use awa5_rs::compiler::{compile_to_awasm, compile_and_render};

// Generate AWASM bytecode
let bytecode = compile_to_awasm("let x = 42;;")?;

// Render Core-IR for debugging
let ir_text = compile_and_render("let x = 42;;")?;
```

## Supported Language Features

### Declarations
- **Modules** - `module Name = struct ... end;;`
- **Include** - `include ModuleName;;`
- **External** - `external name : type = "symbol";;`
- **Let bindings** - `let x = expr;;`
- **Functions** - `fn name args = body;;`

### Expressions
- **Literals** - integers, floats, strings, booleans
- **Identifiers** - variable references
- **Binary ops** - `+`, `-`, `*`, `/`, `==`, `!=`, `<`, `>`, `&&`, `||`
- **If/then/else** - conditional expressions
- **Match** - pattern matching
- **Function application**
- **Parentheses** for grouping

### Types
- Primitive: `int`, `float`, `bool`, `char`, `string`, `unit`
- Awa-specific: `awachar`, `awastring`, `bytes`
- Composite: `list`, `option`, `result`, tuples
- Function arrows: `int -> string -> unit`

### Comments
- OCaml-style: `(* ... *)`
- C-style: `// ...` and `/* ... */`

## Architecture

### Module Structure
- **`lexer.rs`** - Tokenization with keyword recognition, OCaml comment support
- **`parser.rs`** - Recursive descent parser with precedence climbing for expressions
- **`ast.rs`** - Abstract Syntax Tree type definitions
- **`ir.rs`** - Core-IR and type definitions for intermediate representation
- **`mod.rs`** - Compilation orchestration and code generation

### Key Types
- **`AstProgram`** - List of parsed items (modules, functions, etc.)
- **`CoreProgram`** - List of compiled functions with control flow
- **`Awatism`** - AWASM bytecode instruction enum

## Known Limitations

1. **Type checking is stubbed** - Uses placeholder types, no real type inference
2. **Constant loading** - Uses placeholder `Nop` for constant values
3. **Advanced modules** - Module signatures and functors not yet parsed
4. **Labeled parameters** - Labeled/optional parameters not yet supported
5. **Control flow graphs** - Minimal CFG construction, no optimizations

## Future Work

1. Real type inference and checking
2. Proper constant loading and stack management in codegen
3. Full module system with signatures
4. Labeled and optional parameters
5. Control flow optimization
6. Error messages with better diagnostics

