# AwaML `external` Declaration Draft

This document proposes a minimal OCaml-style `external` syntax for AwaML, a human-friendly language that compiles to AWASM `lib` calls.

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
external <name> : <type_expr> = "<symbol_name>";
```

Labeled and optional arguments may be written in OCaml style:

```text
external draw_text : ~msg:string -> ~x:int -> ~y:int -> unit = "drawtext";
external get_title : ?default:string -> string = "get_title";
```

Examples:

```text
external init_window : int -> int -> string -> unit = "initwindow";
external is_key_down : int -> bool = "iskeydown";
external add_float : float -> float -> float = "addfloat";
external print_text : ~msg:string -> unit = "print_text";
external lookup_title : string -> string option = "lookup_title";
external parse_value : string -> (int, string) result = "parse_value";
```

## 3) FFI scalar and string types

Recommended source-level types:

- `int` -> 32-bit signed integer, lowered via the `i32` double-bubble path
- `float` -> 32-bit float, lowered via the `f32` double-bubble path
- `bool` -> one-byte runtime result, typically decoded from `iskeydown`
- `awachar` (AWA-SCII char) -> ABI tag `0x1`
- `char` (ASCII char) -> ABI tag `0x2`
- `awastring` (AWA-SCII string) -> ABI tag `0x3`
- `string` (ASCII string) -> ABI tag `0x4`
- `bytes` -> raw return bytes
- `unit` -> no expected return bytes

## 4) Optional ABI override annotations

If you need explicit control:

```text
external foo : int -> string -> bytes = "foo";
```

Compiler rule: if annotation is present and disagrees with default mapping, emit error.

## 5) Call syntax (draft)

```text
let ok = is_key_down 256;
init_window 800 450 "AWA5.0 Raylib";
```

Codegen must lower calls into canonical AWASM frame shape:

- no args: `[fn_name]`
- with args: `[fn_name, args]`

## 6) Lowering recipe

Given:

```text
external draw_text : string -> int -> int -> int -> int -> int -> int -> unit = "drawtext";
draw_text "Hello" 190 200 20 200 200 200;
```

Lowering steps:

1. Emit function symbol bubble from "drawtext".
2. Emit each typed arg using typed macro-equivalent lowering:
   - `string` -> tag `0x4`
   - each `int` -> tag `0x0`
3. Pack args with `srn 7`.
4. Pack call frame with `srn 2`.
5. Emit `lib`.

## 7) Return decoding policy

Suggested explicit decode forms:

```text
let key = is_key_down 256;
let sum = add_float 1.0 2.0;
```

Rules:

- `unit`: enforce zero-byte or ignored return.
- fixed-width decodes (for example `u8`, `i32`, `f32`): verify byte width.
- `bytes`: pass through raw runtime bytes.

## 8) Compile-time validation mapped to ABI checklist

- Validate symbol name non-empty.
- Validate argument count and declared order at each call.
- Reject implicit narrowing (for example `i64 -> i32`) without explicit cast.
- Validate AWA-SCII literals for `awachar` / `awastring`.
- Reject interior NUL in `string`/`awastring` unless raw-byte mode is used.
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

1. Parse `external` declarations into an extern table.
2. Type-check call sites against extern signatures.
3. Lower typed args into ABI tags + payload bubbles.
4. Emit canonical frame and `lib` instruction.
5. Insert/validate return decode operations.

This is intentionally minimal so a first compiler can be built quickly and kept aligned with current `awa5_rs` behavior.
