# AwaML `extern` Declaration Draft

This document proposes a minimal `extern` syntax for AwaML, a human-friendly language that compiles to AWASM `lib` calls.

AwaML files use the `.awaml` extension.

The goal is a 1:1 mapping with the ABI rules in [ffi-abi.md](ffi-abi.md).

For full parser grammar, see [awaml-grammar.md](awaml-grammar.md).
For typed/lowered compiler node shapes, see [awaml-ir.md](awaml-ir.md).

## 1) Design goals

- Keep declarations small and explicit.
- Make ABI tag mapping deterministic.
- Enable compile-time checks for arity, width, charset mode, and return decoding.
- Avoid hidden coercions.

## 2) Core syntax (draft)

```text
extern fn <local_name>(<params...>) -> <ret_type> = "<symbol_name>";
```

Examples:

```text
extern fn init_window(width: i32, height: i32, title: cstr) -> unit = "initwindow";
extern fn is_key_down(key: i32) -> u8 = "iskeydown";
extern fn add_float(a: f32, b: f32) -> f32 = "addfloat";
```

## 3) FFI scalar and string types

Recommended source-level types:

- `i32` -> ABI tag `0x0` (4-byte LE)
- `f32` -> ABI tag `0x0` (4-byte LE)
- `achar` (AWA-SCII char) -> ABI tag `0x1`
- `char` (ASCII char) -> ABI tag `0x2`
- `acstr` (AWA-SCII string) -> ABI tag `0x3`
- `cstr` (ASCII string) -> ABI tag `0x4`
- `s32` (simple bubble i32) -> ABI tag `0x5`
- `unit` -> no expected return bytes
- `bytes` -> raw return bytes

## 4) Optional ABI override annotations

If you need explicit control:

```text
extern fn foo(x: i32 @[tag=0x0], name: cstr @[tag=0x4]) -> bytes = "foo";
```

Compiler rule: if annotation is present and disagrees with default mapping, emit error.

## 5) Call syntax (draft)

```text
let ok: u8 = is_key_down(256);
init_window(800, 450, "AWA5.0 Raylib");
```

Codegen must lower calls into canonical AWASM frame shape:

- no args: `[fn_name]`
- with args: `[fn_name, args]`

## 6) Lowering recipe

Given:

```text
extern fn draw_text(msg: cstr, x: i32, y: i32, size: i32, r: i32, g: i32, b: i32) -> unit = "drawtext";
draw_text("Hello", 190, 200, 20, 200, 200, 200);
```

Lowering steps:

1. Emit function symbol bubble from "drawtext".
2. Emit each typed arg using typed macro-equivalent lowering:
   - `cstr` -> tag `0x4`
   - each `i32` -> tag `0x0`
3. Pack args with `srn 7`.
4. Pack call frame with `srn 2`.
5. Emit `lib`.

## 7) Return decoding policy

Suggested explicit decode forms:

```text
let key: u8 = is_key_down(256) |> decode_u8;
let sum: f32 = add_float(1.0, 2.0) |> decode_f32;
```

Rules:

- `unit`: enforce zero-byte or ignored return.
- fixed-width decodes (for example `u8`, `i32`, `f32`): verify byte width.
- `bytes`: pass through raw runtime bytes.

## 8) Compile-time validation mapped to ABI checklist

- Validate symbol name non-empty.
- Validate argument count and declared order at each call.
- Reject implicit narrowing (for example `i64 -> i32`) without explicit cast.
- Validate AWA-SCII literals for `achar` / `acstr`.
- Reject interior NUL in `cstr`/`acstr` unless raw-byte mode is used.
- Require explicit return decoder for non-`unit`, non-`bytes` externs.

## 9) Suggested diagnostics

- `E-EXT-001 unknown extern function`
- `E-EXT-002 extern arity mismatch`
- `E-EXT-003 extern argument type mismatch`
- `E-EXT-004 symbol name is empty`
- `E-EXT-005 invalid AWA-SCII literal`
- `E-EXT-006 interior NUL in C-string`
- `E-EXT-007 missing explicit return decoder`
- `E-EXT-008 return decode width mismatch`

## 10) Minimal implementation plan

1. Parse `extern fn` declarations into an extern table.
2. Type-check call sites against extern signatures.
3. Lower typed args into ABI tags + payload bubbles.
4. Emit canonical frame and `lib` instruction.
5. Insert/validate return decode operations.

This is intentionally minimal so a first compiler can be built quickly and kept aligned with current `awa5_rs` behavior.
