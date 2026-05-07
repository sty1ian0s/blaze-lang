# Blaze Phase 1 – Full Syntax Acceptance (LL(1) & Flat AST)

> **Goal:** Define the complete, deterministic LL(1) grammar for Blaze and the flat, array‑based concrete syntax tree (AST) that the parser produces.  This grammar is used by both the bootstrap interpreter (`blazei` in Phase 0) and the self‑hosting JIT compiler.  The AST is designed for zero‑copy semantic analysis and trivial parallel parsing at the item level.

---

## 1. Lexical Structure

Unchanged from Phase 0.  The full set of keywords, literals, operators, and comments is as defined in the main specification (Version 1.0).  The lexer produces a flat vector of tokens, each tagged with a token type and a source span.

---

## 2. Syntax (Fully LL(1) Grammar)

The following grammar has no backtracking.  Every production can be decided by examining the current token or, in a few cases, by looking at the **next** token (LL(1)).  The only production that previously required backtracking—`impl`—has been disambiguated with the mandatory `for` keyword for trait implementations.

### 2.1 Program

```
program        = { item }
item           = vis? attribute* ( fn_decl | struct_decl | enum_decl | union_decl
               | trait_decl | impl_decl | mod_decl | use_decl | const_decl
               | static_decl | extern_decl | actor_decl )
vis            = "pub" [ "(" "opaque" ")" ]
```

### 2.2 Attributes

```
attribute      = "@" ident [ "(" attr_args ")" ]
attr_args      = attr_arg { "," attr_arg } [ "," ]
attr_arg       = ident [ "=" expr ]
```

Attribute order is significant only where explicitly stated.  They are parsed left‑to‑right and expanded at parse time or compile time as needed.

### 2.3 Functions

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

### 2.4 Generic Parameters

```
generic_params = "[" generic_param { "," generic_param } [ "," ] "]"
generic_param  = ident [ ":" bound ] | "effect" ident
bound          = path { "+" path }
```

### 2.5 Types (LL(1))

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
const_expr     = expr   (* must be evaluable at compile time *)
```

### 2.6 Struct, Enum, Union

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

### 2.7 Trait, Impl (Disambiguated)

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

- After `impl` and optional generics, the parser reads a path.  
- If the next token after the path is `for`, the production is a trait impl: `impl Trait for Type { … }`.  
- Otherwise, it is an inherent impl: `impl Type { … }`.  
This is LL(1) because the decision is made on the single token following the path.

### 2.8 Modules, Use

```
mod_decl       = "mod" ident ( ";" | block )
use_decl       = "use" use_path ";"
use_path       = path [ "as" ident ]
               | "{" use_path { "," use_path } [ "," ] "}"
path           = path_segment { "::" path_segment }
path_segment   = ident [ generic_args ]
generic_args   = "<" type { "," type } [ "," ] ">"
```

### 2.9 Constants, Statics, Extern, Actors

```
const_decl     = "const" ident [ ":" type ] "=" expr ";"
static_decl    = "static" [ "mut" ] ident [ ":" type ] "=" expr ";"
extern_decl    = "extern" string_lit "{" { extern_item } "}"
extern_item    = attribute* "fn" ident "(" param_list ")" [ "->" type ] ";"
               | attribute* "static" [ "mut" ] ident ":" type ";"
actor_decl     = "actor" ident [ generic_params ] "{" { actor_item } "}"
actor_item     = field_decl | fn_decl
```

### 2.10 Statements (all brace‑delimited blocks)

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

### 2.11 Expressions (unchanged precedence, same grammar)

The expression grammar and precedence table remain identical to the previously approved version.  No changes are needed for LL(1) because expression parsing is naturally LL(1) with precedence climbing.

### 2.12 Patterns (unchanged)

```
pat            = ident | "_" | literal | path | tuple_pat | struct_pat | enum_pat
tuple_pat      = "(" [ pat { "," pat } [ "," ] ] ")"
struct_pat     = path "{" [ field_pat { "," field_pat } [ "," ] ] "}"
field_pat      = ident [ ":" pat ]
enum_pat       = path "::" ident [ "(" pat ")" | "{" field_pats "}" ]
```

Disambiguation: an unqualified identifier is a binding unless it names a known enum variant or constant in scope.

---

## 3. Flat Concrete Syntax Tree (AST)

The parser produces a **flat array of nodes** instead of a recursive tree.  This structure is cache‑friendly, trivial to traverse in parallel, and sufficient for all later compiler stages.

### 3.1 `Node` Structure

Every syntactic construct is represented by a fixed‑size `Node`:

```c
struct Node {
    u32 tag;           // NodeTag discriminant
    u32 span_start;    // byte offset of the first token
    u32 span_end;      // byte offset after the last token
    u32 payload[4];    // indices into the nodes array or the string table
}
```

The `payload` holds indices that point to child nodes, token indices, or string‑table entries.  The meaning of each payload slot depends on the `tag`.

### 3.2 Node Tags (Complete List)

Every grammar production is assigned a unique tag.  The following is a representative subset that covers all constructs.

| Tag | Production | payload[0] | payload[1] | payload[2] | payload[3] |
|-----|------------|------------|------------|------------|------------|
| `FN_DECL` | `fn` decl | name (string idx) | return type node | body block node | generics node |
| `STRUCT_DECL` | struct decl | name idx | fields block node | generics node | 0 |
| `ENUM_DECL` | enum decl | name idx | variants block node | generics node | 0 |
| `TRAIT_DECL` | trait decl | name idx | items block node | generics node | super‑trait node |
| `IMPL_INH` | inherent impl | type node | items block node | generics node | 0 |
| `IMPL_TRAIT` | trait impl | trait node | type node | items block node | generics node |
| `BLOCK` | `{ stmt* }` | first stmt node | 0 | 0 | 0 |
| `LET_STMT` | let stmt | pattern node | type node | initialiser node | 0 |
| `IF_STMT` | if stmt | condition node | then block node | else node | 0 |
| `WHILE_STMT` | while stmt | condition node | body block node | 0 | 0 |
| `FOR_STMT` | for stmt | pattern node | iterable node | body block node | flags (seq/unroll) |
| `MATCH_STMT` | match stmt | scrutinee node | arms block node | 0 | 0 |
| `BINARY_OP` | binary expr | left node | right node | op token idx | 0 |
| `UNARY_OP` | unary expr | operand node | op token idx | 0 | 0 |
| `CALL` | func call | func node | arguments block node | 0 | 0 |
| `IDENT` | variable/name | string table idx | 0 | 0 | 0 |
| `LITERAL` | literal | token idx | 0 | 0 | 0 |
| … | (all other constructs have analogous fixed‑size representations) | | | | |

### 3.3 Construction Rules

- The parser maintains a `Vec<Node>` and appends nodes in pre‑order (parent before children).  The root node is always `PROGRAM` with its payload pointing to the list of items.
- A “block node” (tag `BLOCK`) has its first child statement index in `payload[0]`; subsequent statements are linked implicitly (they are contiguous in the array).  The end of the block is marked by a sentinel node (`END_BLOCK`) or by the end of the parent’s range.
- String literals, identifiers, and user‑defined names are interned into a global `StringTable` and referenced by index.
- Token literals (numbers, characters) are stored in a separate `TokenBuffer` that records the raw byte representation; the `LITERAL` node points into this buffer.

### 3.4 Parallel Parsing

Because every top‑level item starts with a unique keyword and is self‑delimited by braces or semicolons, the token stream can be split at item boundaries.  Each chunk can be parsed independently by a separate thread, producing a slice of `Node` that is later merged into the global array.  Cross‑item references (e.g., imports) are resolved during semantic analysis, not parsing.

---

## 4. Testing

- **LL(1) conformance:** Use a grammar validator to verify that every production is decidable with at most one token of lookahead.
- **Parser correctness:** For every sample program, produce the flat AST and verify:
  - The depth‑first traversal of the node array reconstructs the expected structure.
  - The tag and payload fields match a known‑good AST (produced by a reference parser).
- **Parallel parsing:** Split a large file at item boundaries, parse each chunk in a separate thread, and verify the merged AST is identical to the single‑threaded parse.
- **No backtracking:** Instrument the parser to ensure it never resets its input position more than one token (i.e., it never backtracks).

All tests must pass before proceeding to Phase 2.
