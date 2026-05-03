# AWA5.RS

An [AWA5.0](https://github.com/TempTempai/AWA5.0) CLI tool written in Rust (btw)

Runs as an AWA5.0 interpreter for Awatisms with file extension `.awasm` and Awatalk with file extension `.awa`

Can also run as an assembler for Awatisms and Awatalk to object files, and assembled object files with extension `.o` can be run by the interpreter

The planned metalanguage for this project is named AwaML, and source files use the `.awaml` extension.

## AwaML docs

- [docs/awaml-grammar.md](docs/awaml-grammar.md)
- [docs/awaml-extern.md](docs/awaml-extern.md)
- [docs/awaml-ir.md](docs/awaml-ir.md)

## AwaML examples

- [examples/awaml/hello_world.awaml](examples/awaml/hello_world.awaml)
- [examples/awaml/libfoo.awaml](examples/awaml/libfoo.awaml)
- [examples/awaml/raylib.awaml](examples/awaml/raylib.awaml)

## FFI ABI Reference

For `lib` call frame layout, typed argument tags, byte ordering, and C binding expectations, see:

- [docs/ffi-abi.md](docs/ffi-abi.md)
- [docs/ffi-examples.md](docs/ffi-examples.md)

## Installation

To install or build from source, you will need to have `rust` or `rustup` installed

```bash
# using Arch Linux's package manager
$ sudo pacman -S rust # or rustup
```

If you installed `rustup`, you need to install a toolchain

```bash
$ rustup toolchain install latest
```

### With Cargo

You can install from crates.io using cargo

```bash
$ cargo install awa5_rs
```

And then run from the command line

```bash
$ awa5_rs --help
```

### From source

Or clone this repository and build it from source using cargo

```bash
$ git clone https://github.com/liraymond04/awa5_rs.git
$ cd awa5_rs
$ cargo build
$ ./target/debug/awa5_rs # you can also build and run with `cargo run`, and you can pass flags with `cargo run -- --help` for example
```

### Web builds

You need Emscripten or [emsdk](https://github.com/emscripten-core/emsdk) installed

```bash
$ cd examples/wasm/raylib
$ cargo build --target wasm32-unknown-emscripten
$ cd target/wasm32-unknown-emscripten/debug
$ emrun index.html # opens the web build on localhost:6931, which can be opened in a browser
```

## Usage

AwaML source files use the `.awaml` extension. The CLI includes an `--awaml` string mode for the compiler scaffold.

```
Usage: awa5_rs [OPTIONS] [input]

Arguments:
  [input]  File to interpret or convert

Options:
  -o, --output <output>  Output to file with new format .awasm .awa .o
  -s, --string <string>  String to interpret or convert
      --awasm            Parse string as awasm
      --awa              Parse string as awatalk
      --awaml            Parse string as AwaML (compiler scaffold)
  -p, --path <path>      Search paths separated by ';' for shared libraries
  -i, --include <include>  Include paths separated by ';' for source files
  -h, --help             Print help
  -V, --version          Print version
```

### AwaML status

AwaML now has an active parse -> Core-IR -> AWASM pipeline with CLI output routing for:

- `.ir` (Core-IR text)
- `.awasm` (rendered AWASM text)
- `.o` (assembled object bytes)

The compiler is still evolving (type checking is stubbed and some lowering/codegen paths remain partial), but end-to-end compilation tests now cover examples including `examples/awaml/raylib.awaml`.

Current supported subset includes:
- float/char/awachar/awastring literal parsing
- extern call lowering with typed ABI framing
- operator-call lowering for arithmetic/comparisons (`+`/`-`/`*`/`/`/`eq`/`lt`/`gt`) into VM opcodes
- module/type/functor shape lowering sufficient for current `raylib.awaml` scaffold compile path
- minimal `if`/CFG lowering (both branches emitted) with conditional-skip idiom
- tuple destructuring in `let` bindings
- minimal `function`/`match` dispatch for constructor literals (needed for raylib’s `keycode_of_key`)

For full compiler details, usage, tests, and limitations, see:

- [src/compiler/README.md](src/compiler/README.md)
