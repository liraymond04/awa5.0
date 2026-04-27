# AwaML IR Draft (Typed AST + Lowered ABI IR)

This document defines a practical IR shape for implementing a compiler from AwaML to AWASM.

Related docs:

- [ffi-abi.md](ffi-abi.md)
- [awaml-extern.md](awaml-extern.md)
- [awaml-grammar.md](awaml-grammar.md)

## 1) Pipeline overview

Recommended compiler phases:

1. Parse source -> `AstProgram`
2. Name resolution + extern table build
3. Type check -> `TypedProgram`
4. Effect/return-decode validation
5. Lower to `CoreIr`
6. Lower extern calls to `AbiIr`
7. Emit AWASM instruction stream

## 2) Semantic type model

```text
Type
  = I32
  | F32
  | Bool
  | CharAscii       // char
  | CharAwa         // achar
  | StrAscii        // cstr
  | StrAwa          // acstr
  | S32             // logical 32-bit bubble integer (not raw `blo` width)
  | U8
  | Bytes
  | Unit
```

## 3) Extern signature model

```text
ExternSig {
  local_name: String,
  symbol_name: String,
  params: Vec<ExternParam>,
  ret: Type,
}

ExternParam {
  name: String,
  ty: Type,
  explicit_tag: Option<u8>,
}
```

Tag mapping (default):

- `I32`, `F32` -> `0x0`
- `CharAwa` -> `0x1`
- `CharAscii` -> `0x2`
- `StrAwa` -> `0x3`
- `StrAscii` -> `0x4`
- `S32` -> `0x5` (logical simple-bubble integer value serialized as 4 little-endian bytes)

Note: `S32` is a language-level / IR-level integer form. It is not the same thing as the original AWA5.0 `blo` instruction, which only pushes a signed 8-bit integer. In this codebase, the consumable 32-bit integer form is represented via the double-bubble `!i32` / `!_i32` style lowering path, while raw `blo` remains an 8-bit signed push at the interpreter level.

## 4) Typed AST (high-level)

```text
TypedProgram {
  items: Vec<TypedItem>,
  externs: Map<String, ExternSig>,
}

TypedItem
  = TypedExternDecl(ExternSig)
  | TypedFnDecl(TypedFn)
  | TypedGlobalLet(TypedLet)

TypedFn {
  name: String,
  params: Vec<TypedParam>,
  ret: Type,
  body: TypedBlock,
}

TypedBlock {
  stmts: Vec<TypedStmt>,
  tail_expr: Option<TypedExpr>,
}
```

Expression nodes should carry resolved type:

```text
TypedExpr {
  kind: TypedExprKind,
  ty: Type,
  span: Span,
}
```

## 5) Core IR (control-flow friendly)

`CoreIr` should remove parser sugar and normalize expression order.

```text
CoreFunc {
  name: String,
  params: Vec<LocalId>,
  ret: Type,
  blocks: Vec<CoreBlock>,
}

CoreBlock {
  id: BlockId,
  stmts: Vec<CoreStmt>,
  term: CoreTerminator,
}

CoreStmt
  = Let { dst: LocalId, value: CoreValue }
  | Assign { dst: LocalId, value: CoreValue }
  | ExternCall { dst: Option<LocalId>, call: CoreExternCall }

CoreTerminator
  = Goto(BlockId)
  | If { cond: LocalId, then_bb: BlockId, else_bb: BlockId }
  | Return(Option<LocalId>)
```

Extern calls in `CoreIr` remain typed, not byte-packed:

```text
CoreExternCall {
  sig: ExternSigId,
  args: Vec<LocalId>,
  decode: ReturnDecode,
}

ReturnDecode
  = None            // for Unit / ignored
  | U8
  | I32
  | F32
  | Bytes
```

## 6) ABI IR (byte-oriented)

`AbiIr` is the final machine-oriented representation right before AWASM emission.

```text
AbiCall {
  symbol_name: String,
  args: Vec<AbiArg>,
  expected_return: AbiReturn,
}

AbiArg {
  tag: u8,                 // 0x0..0x5
  payload: AbiPayload,
}

AbiPayload
  = Le4([u8; 4])           // i32/f32
  | OneByte(u8)            // chars
  | CString(Vec<u8>)       // bytes without terminator in node; emitter appends 0x00
  | SimpleI32Le4([u8; 4])  // tag 0x5

AbiReturn
  = Unit
  | RawBytes
  | DecodeU8
  | DecodeI32Le
  | DecodeF32Le
```

## 7) Validation points by stage

### Typed stage

- Unknown extern name at call site -> `E-EXT-001`
- Arity mismatch -> `E-EXT-002`
- Argument type mismatch -> `E-EXT-003`
- Missing explicit return decode for required return type -> `E-EXT-007`

### Core->ABI lowering stage

- Unsupported tag after mapping -> `E-FFI-010`
- Invalid width for numeric decode/encode -> `E-FFI-011`
- Invalid AWA-SCII literal in `CharAwa`/`StrAwa` -> `E-FFI-030`
- Interior NUL in C-string mode -> `E-FFI-031`

### ABI emission stage

- Non-canonical frame shape generation attempt -> `E-FFI-001`
- Empty symbol name -> `E-FFI-002`

## 8) AWASM emission contract for `AbiCall`

Emitter should produce:

1. Function name bubble (`!str`-equivalent lowering)
2. One typed arg bubble per `AbiArg`
3. `srn <argc>` to wrap args (if `argc > 0`)
4. `srn 2` for full frame with args, else `srn 1`
5. `lib`
6. Optional decode sequence from return bubbles based on `AbiReturn`

## 9) Minimal decode helpers in IR

To avoid implicit behavior, represent return decode explicitly:

```text
DecodeOp
  = TakeBytes(n)
  | ToU8
  | ToI32Le
  | ToF32Le
```

Attach a decode plan to each extern call result.

## 10) Implementation sketch in Rust-style enums

```text
enum Type { I32, F32, Bool, CharAscii, CharAwa, StrAscii, StrAwa, S32, U8, Bytes, Unit }

enum AbiPayload {
    Le4([u8; 4]),
    OneByte(u8),
    CString(Vec<u8>),
    SimpleI32Le4([u8; 4]),
}

struct AbiArg { tag: u8, payload: AbiPayload }

enum AbiReturn { Unit, RawBytes, DecodeU8, DecodeI32Le, DecodeF32Le }

struct AbiCall {
    symbol_name: String,
    args: Vec<AbiArg>,
    expected_return: AbiReturn,
}
```

This split (Typed AST -> Core IR -> ABI IR) keeps parser concerns, type concerns, and byte-level ABI concerns isolated and easier to test.
