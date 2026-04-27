# AWA5.RS

An [AWA5.0](https://github.com/TempTempai/AWA5.0) CLI tool written in Rust (btw)

Runs as an AWA5.0 interpreter for Awatisms with file extension `.awasm` and Awatalk with file extension `.awa`

Can also run as an assembler for Awatisms and Awatalk to object files, and assembled object files with extension `.o` can be run by the interpreter

The planned metalanguage for this project is named AwaML, and source files use the `.awaml` extension.

## AwaML docs

- [docs/awaml-grammar.md](docs/awaml-grammar.md)
- [docs/awaml-extern.md](docs/awaml-extern.md)
- [docs/awaml-ir.md](docs/awaml-ir.md)

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

AwaML is scaffolded in the codebase and can be parsed at a stub level, but lowering and code generation are not implemented yet.
