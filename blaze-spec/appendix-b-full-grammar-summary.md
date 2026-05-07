# Appendix B – Full Grammar Summary

> **Status:** Normative.  This appendix contains the complete, unambiguous EBNF grammar for the Blaze language (Version 1.0).  Every production that appears in the main specification is reproduced here without abbreviations.  Implementors may use this appendix as the definitive parsing reference.

---

## B.1 Program

```
program        = { item }
item           = vis? attribute* ( fn_decl | struct_decl | enum_decl | union_decl
               | trait_decl | impl_decl | mod_decl | use_decl | const_decl
               | static_decl | extern_decl | actor_decl )
vis            = "pub" [ "(" "opaque" ")" ]
```

## B.2 Attributes

```
attribute      = "@" ident [ "(" attr_args ")" ]
attr_args      = attr_arg { "," attr_arg } [ "," ]
attr_arg       = ident [ "=" expr ]
```

## B.3 Functions

```
fn_decl        = attribute* [ "async" ] "fn" ident [ generic_params ]
                 "(" param_list ")" [ "->" type [ effect_spec ] ]
                 [ where_clause ] [ contract_clause ] block
fn_decl_no_body = attribute* [ "async" ] "fn" ident [ generic_params ]
                 "(" param_list ")" [ "->" type [ effect_spec ] ] ";"
param_list     = [ param { "," param } [ "," ] ]
param          = pat ":" type [ "=" expr ]
where_clause   = "where" where_bound { "," where_bound } [ "," ]
where_bound    = ( ident | "effect" ident ) ":" bound
contract_clause= { "requires" expr } { "ensures" expr }
```

## B.4 Generic Parameters

```
generic_params = "[" generic_param { "," generic_param } [ "," ] "]"
generic_param  = ident [ ":" bound ] | "effect" ident
bound          = path { "+" path }
```

## B.5 Types

```
type           = path
               | primitive_type
               | "(" ")"
               | "(" type "," [ type { "," type } [ "," ] ] ")"
               | "[" type ";" const_expr "]"
               | "[" type "]"
               | "*" type | "*mut" type
               | "&" type | "&mut" type
               | "fn" "(" [ param_types ] ")" [ "->" type ] [ effect_spec ]
               | "impl" bound
               | "dyn" bound
primitive_type = "u8" | "u16" | "u32" | "u64" | "u128" | "usize"
               | "i8" | "i16" | "i32" | "i64" | "i128" | "isize"
               | "f16" | "f32" | "f64" | "f128"
               | "bool" | "char" | "str"
param_types    = type { "," type } [ "," ]
effect_spec    = "/" effect_set
effect_set     = effect { "+" effect }
effect         = path
const_expr     = expr
```

## B.6 Struct, Enum, Union

```
struct_decl    = "struct" ident [ generic_params ]
                 ( "{" field_list "}" | "(" type_list ")" | ";" )
                 [ struct_invariant ]
field_list     = [ field_decl { "," field_decl } [ "," ] ]
field_decl     = vis? ident ":" type [ "=" expr ]
type_list      = type { "," type } [ "," ]
struct_invariant = "contract_invariant" expr ";"
enum_decl      = "enum" ident [ generic_params ] "{" variant_list "}"
variant_list   = [ variant_decl { "," variant_decl } [ "," ] ]
variant_decl   = ident [ "=" expr ]
                 ( "(" type_list ")" | "{" field_list "}" | ε )
union_decl     = "union" ident [ generic_params ] "{" field_list "}"
```

## B.7 Trait, Impl

```
trait_decl     = "trait" ident [ generic_params ] [ ":" bound ]
                 "{" { trait_item } "}"
trait_item     = fn_decl_no_body | const_decl | type_decl
type_decl      = "type" ident [ generic_params ] [ "=" type ] ";"
impl_decl      = "impl" [ generic_params ]
                 ( type "{" { impl_item } "}"
                 | bound "for" type "{" { impl_item } "}" )
impl_item      = fn_decl | const_decl | type_decl
```

## B.8 Modules, Use

```
mod_decl       = "mod" ident ( ";" | block )
use_decl       = "use" use_path ";"
use_path       = path [ "as" ident ]
               | "{" use_path { "," use_path } [ "," ] "}"
path           = path_segment { "::" path_segment }
path_segment   = ident [ generic_args ]
generic_args   = "<" type { "," type } [ "," ] ">"
```

## B.9 Constants, Statics, Extern, Actors

```
const_decl     = "const" ident [ ":" type ] "=" expr ";"
static_decl    = "static" [ "mut" ] ident [ ":" type ] "=" expr ";"
extern_decl    = "extern" string_lit "{" { extern_item } "}"
extern_item    = attribute* "fn" ident "(" param_list ")" [ "->" type ] ";"
               | attribute* "static" [ "mut" ] ident ":" type ";"
actor_decl     = "actor" ident [ generic_params ] "{" { actor_item } "}"
actor_item     = field_decl | fn_decl
```

## B.10 Statements

```
stmt           = let_stmt | expr_stmt | if_stmt | while_stmt | for_stmt
               | loop_stmt | match_stmt | guard_stmt | defer_stmt
               | return_stmt | try_stmt | with_stmt
let_stmt       = "let" [ "mut" ] pat [ ":" type ] [ "=" expr ] ";"
expr_stmt      = expr ";"
if_stmt        = "if" expr block [ "else" ( if_stmt | block ) ]
while_stmt     = "while" expr block [ "else" block ]
for_stmt       = [ "unroll" ] ( "for" | "seq" "for" ) pat "in" expr block
loop_stmt      = "loop" block
match_stmt     = "match" expr "{" match_arms "}"
match_arms     = match_arm { "," match_arm } [ "," ]
match_arm      = pat [ "when" expr ] "=>" expr
guard_stmt     = "guard" cond = expr "else" block
defer_stmt     = "defer" block
return_stmt    = "return" [ expr ] ";"
try_stmt       = "try" block "catch" ( pat "=>" block | block | expr ) ";"
with_stmt      = "with" expr [ "as" pat ] block
```

## B.11 Expressions

```
expr           = assign_expr
assign_expr    = logical_or_expr [ assign_op assign_expr ]
assign_op      = "=" | "+=" | "-=" | "*=" | "/=" | "%=" | "&=" | "|=" | "^=" | "<<=" | ">>="
logical_or_expr = logical_and_expr { "||" logical_and_expr }
logical_and_expr = bit_or_expr { "&&" bit_or_expr }
bit_or_expr    = bit_xor_expr { "|" bit_xor_expr }
bit_xor_expr   = bit_and_expr { "^" bit_and_expr }
bit_and_expr   = equality_expr { "&" equality_expr }
equality_expr  = relational_expr { ("=="|"!="|"=~") relational_expr }
relational_expr = shift_expr { ("<"|">"|"<="|">=") shift_expr }
shift_expr     = additive_expr { ("<<"|">>") additive_expr }
additive_expr  = multiplicative_expr { ("+"|"-") multiplicative_expr }
multiplicative_expr = unary_expr { ("*"|"/"|"%") unary_expr }
unary_expr     = ( "!" | "-" | "*" | "&" | "&mut" | "own" ) unary_expr | postfix_expr
postfix_expr   = primary_expr
                 { "." ident | "[" expr "]" | "(" args ")" | "?"
                 | "catch" ( pat "=>" expr | expr ) | "as" type | "is" pat
                 | "@" string_lit | "@retry" "(" attr_args ")" | "@log" [ "(" string_lit ")" ] }
primary_expr   = literal | ident | "(" expr ")"
               | block_expr | "if" expr block [ "else" block ] | "match" expr "{" match_arms "}"
               | "loop" block | "while" expr block | "for" pat "in" expr block
               | "try" block "catch" ( pat "=>" block | block | expr )
               | "unsafe" block | "region" block | "async" block | "await" expr
               | closure_expr | macro_invocation
block_expr     = "{" { stmt } "}"
closure_expr   = "\" param_list "->" expr
macro_invocation = path "!" "(" args ")"
args           = [ expr { "," expr } [ "," ] ]
```

## B.12 Patterns

```
pat            = ident | "_" | literal | path | tuple_pat | struct_pat | enum_pat
tuple_pat      = "(" [ pat { "," pat } [ "," ] ] ")"
struct_pat     = path "{" [ field_pat { "," field_pat } [ "," ] ] "}"
field_pat      = ident [ ":" pat ]
enum_pat       = path "::" ident [ "(" pat ")" | "{" field_pats "}" ]
```

## B.13 Blocks

```
block          = "{" { stmt } "}"
```
