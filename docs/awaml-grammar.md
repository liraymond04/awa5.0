# AwaML MVP Grammar (OCaml-style EBNF Draft)

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

comment       = "(*", { ? any char ? }, "*)"
              | "//", { ? any char except newline ? } ;
```

## 2) Program structure

```ebnf
program       = { item } ;

item          = type_decl
              | external_decl
              | module_decl
              | module_type_decl
              | include_stmt
              | let_item
              | stmt ;

top_sep       = ";;" ;
```

## 3) Type declarations

```ebnf
type_decl     = "type", ident, [type_params], "=", type_def, ";" ;

type_params   = { type_param } ;
type_param    = "'", ident ;

type_def      = variant_type
              | record_type
              | poly_variant_type
              | alias_type ;

alias_type    = type_expr ;

variant_type  = constructor, { "|", constructor } ;
constructor   = ident, ["of", type_expr] ;

poly_variant_type = "[", "<", poly_variant_row, { "|", poly_variant_row }, ["|", ".."], ">", "]" ;
poly_variant_row  = "`", ident, ["of", type_expr] ;

module_decl   = "module", ident, "=", module_expr, ";" ;
module_expr   = ident
              | "struct", { item }, "end"
              | "functor", "(", ident, ":", module_sig, ")", "->", module_expr
              | module_expr, "(", module_expr, ")"
              | "(", "module", ident, ")" ;

module_type_decl = "module", "type", ident, "=", module_sig, ";" ;

module_sig    = ident
              | "sig", { sig_item }, "end" ;

sig_item      = "val", ident, ":", type_expr, ";"
              | "type", ident, [type_params], ["=", type_def], ";" ;

include_stmt  = "include", module_expr, ";" ;

record_type   = "{", record_field, { ";", record_field }, [";"], "}" ;
record_field  = ident, ":", type_expr ;
```

## 4) External declarations

```ebnf
external_decl = "external", ident, ":", type_expr, "=", string_lit, ";" ;

label         = "~", ident ;
optional_label = "?", ident ;

arg           = [label | optional_label], expr ;

param         = [label | optional_label], pattern ;
```

## 5) Let-bindings and functions

```ebnf
let_item      = "let", ["rec"], binding, { "and", binding }, [top_sep] ;

let_in_expr   = "let", ["rec"], binding, { "and", binding }, "in", expr ;

local_open    = "let", "open", ident, "in", expr ;

binding       = param, { param }, "=", expr ;

pattern       = "_"
              | ident
              | int_lit
              | char_lit
              | string_lit
              | constructor_pattern
              | list_pattern
              | tuple_pattern
              | as_pattern
              | "(", [pattern, { ",", pattern }], ")" ;

constructor_pattern = ident, [pattern] ;
list_pattern       = "[", [pattern, { ";", pattern }], "]" ;
tuple_pattern      = "(", pattern, ",", pattern, { ",", pattern }, ")" ;
as_pattern         = pattern, "as", ident ;

block         = expr ;
```

## 6) Core statements

```ebnf
stmt          = expr ;

expr_stmt     = expr ;
```

## 7) Expressions

```ebnf
expr          = let_in_expr
              | local_open
              | if_expr
              | match_expr
              | fun_expr
              | function_expr
              | cons_expr ;

if_expr       = "if", expr, "then", expr, "else", expr ;

match_expr    = "match", expr, "with", match_case, { "|", match_case } ;
match_case    = pattern, ["when", expr], "->", expr ;

fun_expr      = "fun", param, { param }, "->", expr ;
function_expr = "function", match_case, { "|", match_case } ;

logic_or_expr = logic_and_expr, { "||", logic_and_expr } ;
logic_and_expr= equality_expr, { "&&", equality_expr } ;
equality_expr = compare_expr, { ("==" | "!="), compare_expr } ;
compare_expr  = add_expr, { ("<" | "<=" | ">" | ">="), add_expr } ;
add_expr      = mul_expr, { ("+" | "-"), mul_expr } ;
mul_expr      = prefix_expr, { ("*" | "/"), prefix_expr } ;

prefix_expr   = ["-" | "!"], postfix_expr ;

cons_expr     = logic_or_expr, { "::", logic_or_expr } ;

postfix_expr  = primary_expr, { application } ;
application   = [arg_list] ;

arg_list      = expr, { ",", expr } ;

primary_expr  = ident
              | int_lit
              | float_lit
              | char_lit
              | awachar_lit
              | string_lit
              | awastring_lit
              | unit_lit
              | list_expr
              | tuple_expr
              | "(", expr, ")"
              | labeled_record_expr
              | record_update_expr
              | poly_variant_expr ;

unit_lit      = "()" ;
list_expr     = "[", [expr, { ";", expr }], "]" ;
tuple_expr    = "(", expr, ",", expr, { ",", expr }, ")" ;
labeled_record_expr = "{", [field_value, { ";", field_value }], [";"], "}" ;
field_value   = ident, "=", expr ;
record_update_expr = "{", expr, "with", field_value, { ";", field_value }, "}" ;
poly_variant_expr = "`", ident, [expr] ;
```

## 8) Type system (MVP)

```ebnf
type_expr     = type_arrow ;

type_arrow    = type_cons, { "->", type_cons } ;

type_cons     = type_simple, { type_postfix } ;

type_postfix  = "list"
              | "option"
              | "result" ;

type_simple   = type_atom
              | "(", type_expr, ")"
              | "(", type_expr, ",", type_expr, { ",", type_expr }, ")" ;

type_atom     = "int"
              | "float"
              | "bool"
              | "char"
              | "string"
              | "awachar"
              | "awastring"
              | "bytes"
              | "unit"
              | ident
              | poly_type ;

poly_type     = "[", "<", poly_row, { "|", poly_row }, ["|", ".."], ">", "]" ;
poly_row      = "`", ident, ["of", type_expr] ;
```

## 9) Reserved keywords

```text
type external let rec and in fun match with if then else
true false int float bool char string awachar awastring bytes unit
list
function as when
module struct end open with include sig val functor
;;
```

## 10) Parsing + lowering notes

- `external` declarations should populate an extern table before type-checking call sites.
- Function application is whitespace-based, OCaml-style.
- Top-level items may optionally be terminated with `;;` for familiarity.
- `awachar` / `awastring` values require AWA-SCII validation during semantic analysis.
- Calls that target extern declarations lower to canonical `lib` frame shape described in [ffi-abi.md](ffi-abi.md).

## 11) Example accepted snippet

```text
type maybe_int =
  | None
  | Some of int;

module type TEXT_SIG = sig
  val print_text : string -> unit;
end;

module Text = struct
  external print_text : string -> unit = "print_text";
end;

module MakeGreeter = functor (T : TEXT_SIG) -> struct
  let greet ~msg = T.print_text msg;
end;

include MakeGreeter(Text);

let main () =
  greet ~msg:"Hello, AwaML!"
```
