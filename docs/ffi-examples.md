# AWA5.RS FFI Worked Examples

This document complements the ABI reference with concrete examples based on existing programs.

- ABI reference: [ffi-abi.md](ffi-abi.md)

## 1) `libfoo.awasm` call walkthrough

Source: [examples/awasm/libfoo.awasm](../examples/awasm/libfoo.awasm)

The program builds:

1. Function name bubble: `!str "foo"`
2. Six typed args:
   - `!_i32 4`
   - `!_str a"Hello World!"`
   - `!_str "foo"`
   - `!_f32 4.2`
   - `!_chr a'W'`
   - `!_chr 'a'`
3. Args list with `srn 6`
4. Full call frame with `srn 2`
5. Dispatch with `lib`

Logical frame shape at `lib`:

```text
[
  fn_name("foo"),
  args([
    [0x0, i32(4)],
    [0x3, awascii_string("Hello World!")],
    [0x4, ascii_string("foo")],
    [0x0, f32(4.2)],
    [0x1, awascii_char('W')],
    [0x2, ascii_char('a')]
  ])
]
```

### Marshaled byte stream seen by C (argument order)

The runtime flattens typed args into one contiguous `uint8_t*` payload:

- `i32 4` -> `04 00 00 00`
- `awascii "Hello World!"` -> converted to ASCII bytes, then `00`
- `ascii "foo"` -> `66 6F 6F 00`
- `f32 4.2` -> `66 66 86 40`
- `awascii 'W'` -> converted to one ASCII byte
- `ascii 'a'` -> `61`

This matches the unpacking pattern in [examples/lib/foo.c](../examples/lib/foo.c), which reads `int32`, C-string, C-string, `float`, `char`, `char` in that order.

## 2) `raylib.awasm` call walkthrough

Source: [examples/awasm/raylib.awasm](../examples/awasm/raylib.awasm)

### `initwindow` setup

Program fragment:

```awasm
!str "initwindow"
!_i32 800
!_i32 450
!_str "AWA5.0 Raylib"
srn 3
srn 2
lib
```

Logical frame:

```text
[
  fn_name("initwindow"),
  args([
    [0x0, i32(800)],
    [0x0, i32(450)],
    [0x4, ascii_string("AWA5.0 Raylib")]
  ])
]
```

Marshaled payload bytes begin as:

- `i32 800` -> `20 03 00 00`
- `i32 450` -> `C2 01 00 00`
- `"AWA5.0 Raylib"` -> ASCII bytes + `00`

This matches `initwindow` decoding in [examples/lib/awa5_raylib.c](../examples/lib/awa5_raylib.c).

### No-arg draw calls

Program fragment:

```awasm
!str "BeginDrawing"
srn 1
lib
```

No args bubble is provided; runtime calls symbol with empty arg payload.

## 3) Return bytes back into bubble abyss

When a C function writes `*out`/`*out_len`, runtime pushes each returned byte as `Simple(i32)`.

Example from [examples/lib/foo.c](../examples/lib/foo.c):

- returns one byte `0x01`
- AWASM side receives one simple bubble with value `1`

`pr1` then prints numeric output.

## 4) Quick authoring template

Use this shape for predictable FFI calls:

```awasm
!str "function_name"
!_i32 123
!_str "hello"
srn 2
srn 2
lib
```

For no-arg functions:

```awasm
!str "function_name"
srn 1
lib
```
