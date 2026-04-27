# AwaML MVP Grammar (EBNF Draft)

This grammar is a minimal, implementation-oriented draft for AwaML, the source language that compiles to AWASM + `lib` calls.

AwaML files use the `.awaml` extension.

It is designed to pair with:

- [ffi-abi.md](ffi-abi.md)
- [awaml-extern.md](awaml-extern.md)
- [awaml-ir.md](awaml-ir.md)

## 1) Lexical conventions

```ebnf
letter        = "A".."Z" | "a".."z" | "_" ;
digit         = "0".."9" ;
hex_digit     = digit | "A".."F" | "a".."f" ;

ident         = letter , { letter | digit } ;
int_lit       = ["-"], digit, { digit } ;
float_lit     = ["-"], digit, { digit }, ".", digit, { digit } ;

char_lit      = "'", ? single ASCII char or escape ?, "'" ;
string_lit    = "\"", { ? char or escape ? }, "\"" ;

awachar_lit   = "a", char_lit ;
awastring_lit = "a", string_lit ;

comment       = "//", { ? any char except newline ? } ;
```

## 2) Program structure

```ebnf
program       = { item } ;

item          = extern_decl
              | fn_decl
              | let_decl
              | stmt ;
```

## 3) Extern declarations

```ebnf
extern_decl   = "extern", "fn", ident, "(", [param_list], ")",
                "->", type_ref, "=", string_lit, ";" ;

param_list    = param, { ",", param } ;
param         = ident, ":", type_ref, [abi_attr] ;

abi_attr      = "@[", "tag", "=", hex_byte, "]" ;
hex_byte      = "0x", hex_digit, hex_digit ;
```

## 4) Function declarations (pure-first core)

```ebnf
fn_decl       = "fn", ident, "(", [param_list], ")", "->", type_ref, block ;

block         = "{", { stmt }, [expr], "}" ;
```

## 5) Statements

```ebnf
stmt          = let_decl
              | assign_stmt
              | if_stmt
              | while_stmt
              | return_stmt
              | expr_stmt ;

let_decl      = "let", ident, [":", type_ref], "=", expr, ";" ;
assign_stmt   = ident, "=", expr, ";" ;

if_stmt       = "if", expr, block, ["else", block] ;
while_stmt    = "while", expr, block ;

return_stmt   = "return", [expr], ";" ;
expr_stmt     = expr, ";" ;
```

## 6) Expressions

```ebnf
expr          = pipe_expr ;

pipe_expr     = logic_or_expr, { "|>", ident } ;

logic_or_expr = logic_and_expr, { "||", logic_and_expr } ;
logic_and_expr= equality_expr, { "&&", equality_expr } ;
equality_expr = compare_expr, { ("==" | "!="), compare_expr } ;
compare_expr  = add_expr, { ("<" | "<=" | ">" | ">="), add_expr } ;
add_expr      = mul_expr, { ("+" | "-"), mul_expr } ;
mul_expr      = unary_expr, { ("*" | "/"), unary_expr } ;

unary_expr    = ["!" | "-"], postfix_expr ;

postfix_expr  = primary_expr, { call_suffix } ;
call_suffix   = "(", [arg_list], ")" ;

arg_list      = expr, { ",", expr } ;

primary_expr  = ident
              | int_lit
              | float_lit
              | char_lit
              | awachar_lit
              | string_lit
              | awastring_lit
              | "(", expr, ")" ;
```

## 7) Type system (MVP)

```ebnf
type_ref      = "i32"
              | "f32"
              | "bool"
              | "char"
              | "achar"
              | "cstr"
              | "acstr"
              | "s32"
              | "u8"
              | "bytes"
              | "unit" ;
```

## 8) Reserved keywords

```text
extern fn let if else while return true false
i32 f32 bool char achar cstr acstr s32 u8 bytes unit
```

## 9) Parsing + lowering notes

- `extern fn` signatures should populate an extern table before type-checking call sites.
- Pipeline form `expr |> decode_u8` is intentionally grammar-level so return decoding is explicit.
- `achar` / `acstr` literals require AWA-SCII validation during semantic analysis.
- Calls that target extern declarations lower to canonical `lib` frame shape described in [ffi-abi.md](ffi-abi.md).

## 10) Example accepted snippet

```text
extern fn is_key_down(key: i32) -> u8 = "iskeydown";
extern fn init_window(width: i32, height: i32, title: cstr) -> unit = "initwindow";

fn main() -> unit {
  init_window(800, 450, "AWA5.0 Raylib");
  let down: u8 = is_key_down(256) |> decode_u8;
  return;
}
```
