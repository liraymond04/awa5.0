# AWA5.RS FFI ABI Specification

This document specifies the current Foreign Function Interface (FFI) ABI used by `awa5_rs` when executing `lib` calls from AWASM.

It is intended as a stable target for tools that compile to AWASM.

Worked examples are available in [ffi-examples.md](ffi-examples.md).
Draft `extern` declaration syntax is available in [metalanguage-extern.md](metalanguage-extern.md).

## 1) Call Frame Shape for `lib`

At runtime, `lib` expects the top bubble to be a `Double` with this logical shape:

- `[fn_name]` for no-arg calls
- `[fn_name, args]` for calls with arguments

Where:

- `fn_name` is a bubble representation of a string.
- `args` is a `Double` where each item is one typed argument (also a `Double`).

### Typical AWASM construction pattern

```awasm
!str "foo"      ; function name
!_i32 4         ; typed arg 1
!_str "bar"    ; typed arg 2
srn 2           ; surround args list (if you pushed 2 args)
srn 2           ; surround [fn_name, args]
lib
```

No-arg call pattern:

```awasm
!str "BeginDrawing"
srn 1
lib
```

## 2) Endianness and Numeric Encoding

- Numeric payloads are little-endian.
- `!i32` emits 4 bytes and wraps with `srn 4`.
- `!f32` emits IEEE-754 `f32` bytes and wraps with `srn 4`.

## 3) Typed Argument Encoding (for `!_...` macros)

Each typed argument is encoded as a `Double([type_tag, value])`.

Type tags currently recognized by runtime argument marshaling:

- `0x0`: i32/f32 raw 4-byte payload from a value double bubble
- `0x1`: AWA-SCII character index (converted to one ASCII byte)
- `0x2`: ASCII character byte
- `0x3`: AWA-SCII string (converted to bytes, then null-terminated)
- `0x4`: ASCII string bytes (null-terminated)
- `0x5`: simple bubble value encoded as little-endian i32

### Macro mappings

- `!_i32 N` => `[0x0, <i32-le-4-bytes>]`
- `!_f32 X` => `[0x0, <f32-le-4-bytes>]`
- `!_chr a'C'` => `[0x1, <awascii-index>]`
- `!_chr 'C'` => `[0x2, <ascii-byte>]`
- `!_str a"..."` => `[0x3, <awascii-string-bubble>]`
- `!_str "..."` => `[0x4, <ascii-string-bubble>]`

## 4) String Ordering Rules

Strings are assembled through stack operations and often exist in reversed bubble order internally.

During marshaling:

- function names are reversed before UTF-8 decode,
- string argument bubbles are iterated in reverse to produce byte order,
- string arguments are null-terminated (`\0`).

For C libraries, treat string args as C strings.

## 5) Shared Library Discovery

- CLI `--path` accepts a semicolon-separated list of directories.
- Runtime scans each directory for platform extension:
  - Linux: `.so`
  - macOS: `.dylib`
  - Windows: `.dll`

The first loaded symbol matching function name is used.

## 6) Native Symbol ABI

Native (non-wasm) dynamic symbols are called with this signature:

```c
void fn(const uint8_t *data, uint8_t **out, size_t *out_len)
```

Contract:

- `data` points to contiguous packed argument bytes in AWASM declaration order.
- If returning bytes:
  - allocate `*out` with heap allocation compatible with Rust side ownership transfer,
  - set `*out_len` to the number of bytes.
- If no return value:
  - leave `*out` null or return zero-length.

The runtime currently takes ownership of returned memory by reconstructing a `Vec<u8>` from `(*out, *out_len)`.

## 7) Return Value Semantics in AWASM

- Returned bytes are pushed back onto the bubble abyss as individual `Simple(i32)` bubbles (`byte -> i32`).
- There is no built-in typed return decoder; callers interpret stack values explicitly.

## 8) WASM Target Notes

On wasm target builds, `load_libs` is replaced by a static function table that maps known names to imported Rust externs.

- Some entries are `WithArgs(data, out, out_len)`.
- Some are `NoArgs()` convenience entries (for example `BeginDrawing`, `EndDrawing`).

## 9) Compatibility Guidance for Metalanguage Compilers

When compiling a higher-level language to AWASM `lib` calls:

1. Always build canonical frame shape: `[fn_name]` or `[fn_name, args]`.
2. Use `!_...` typed macros (or equivalent lowered instructions) to avoid manual tag mistakes.
3. Preserve little-endian numeric encoding.
4. Keep string payloads null-terminated for C interop.
5. Define and version your own typed-return convention at language level (since runtime is byte-oriented).

## 10) Current Status

This document reflects current runtime behavior and is not yet an explicitly versioned ABI in the CLI.
If you plan to build a metalanguage compiler, treat this as `ABI v0` and pin against a specific `awa5_rs` release.

## 11) Metalanguage Backend Checklist

Use this checklist in your compiler backend to enforce ABI correctness before emitting AWASM.

### A. Frame construction rules

- Emit `lib` call frames only in canonical shape:
  - no args: `[fn_name]`
  - with args: `[fn_name, args]`
- Ensure `fn_name` resolves to a non-empty UTF-8 string after runtime reversal.
- Ensure `args` is a `Double` of typed-argument doubles.

Suggested compile errors:

- `E-FFI-001 invalid lib frame shape`
- `E-FFI-002 empty function name`
- `E-FFI-003 function name not encodable`

### B. Type-tag emission rules

- Emit one of supported tags only: `0x0..0x5`.
- For tag `0x0`, emit exactly 4 payload bytes.
- For tag `0x1`/`0x2`, emit exactly one character payload.
- For tag `0x3`/`0x4`, emit string payload and expect null-terminated marshaling.
- For tag `0x5`, ensure source is representable as `i32`.

Suggested compile errors:

- `E-FFI-010 unsupported type tag`
- `E-FFI-011 invalid numeric payload width`
- `E-FFI-012 invalid char payload width`
- `E-FFI-013 i32 overflow in simple-bubble argument`

### C. Endianness and scalar layout

- Always lower numeric scalars (`i32`, `f32`) as little-endian.
- Reject host-dependent or target-dependent byte ordering.

Suggested compile errors:

- `E-FFI-020 non-little-endian scalar encoding`

### D. String and charset rules

- Distinguish AWA-SCII vs ASCII at source type level.
- For AWA-SCII literals, validate every character is in `AWA_SCII`.
- Preserve literal order at language level; let runtime reversal/marshaling normalize order.
- For C interop assumptions, disallow interior `\0` in source strings unless explicitly escaped as raw bytes.

Suggested compile errors:

- `E-FFI-030 invalid AWA-SCII character`
- `E-FFI-031 interior NUL not allowed in C-string mode`

### E. Arity and signature checks (recommended)

If your compiler has optional extern declarations, validate at compile time:

- argument count matches declaration
- each argument’s ABI tag class matches declaration
- no implicit narrowing (for example `i64 -> i32`) without explicit cast

Suggested compile errors:

- `E-FFI-040 extern arity mismatch`
- `E-FFI-041 extern argument type mismatch`
- `E-FFI-042 lossy numeric conversion without cast`

### F. Return handling policy

- Treat returns as raw bytes at ABI level.
- Define a language-level decode convention (for example: `u8`, `bool`, `i32`, `f32`, c-string pointer model).
- Require explicit decode operator in source or insert typed decoder stubs.

Suggested compile errors:

- `E-FFI-050 missing return decode`
- `E-FFI-051 decode width mismatch`

### G. Platform/library resolution checks

- Validate configured library search paths are non-empty in deployment profile.
- Optionally require declared symbol names to be non-empty and ASCII-safe.
- Consider lints for case-sensitive symbol mismatches (`BeginDrawing` vs `begindrawing`).

Suggested diagnostics:

- `W-FFI-060 empty library search path set`
- `W-FFI-061 symbol contains suspicious casing`

### H. Codegen patterns to prefer

- Prefer `!_i32`, `!_f32`, `!_chr`, `!_str` style lowering helpers over manual `blo/srn` emission.
- Auto-generate `srn N` for arg packing and final `srn 2` frame wrapping.
- Emit deterministic argument order matching extern declaration order.

### I. Minimal pre-emit validation pass

Before final AWASM output, run a backend pass that checks each planned `lib` call for:

1. canonical frame shape
2. supported tags only
3. payload width per tag
4. scalar endianness
5. charset validity (AWA-SCII mode)
6. explicit return decode policy

If any check fails, abort codegen with structured diagnostics.