# Blaze Phase 0 – Minimal Language Specification  
*(for the bootstrapping interpreter `blazei`)*

> **Goal:** implement a Rust‑based interpreter (Rust 1.60.0) that can execute a tiny subset of Blaze.  Later phases will add generics, effects, actors, etc.  The interpreter is a pure Rust project with no external dependencies beyond the standard library.

---

## 1. Lexical Structure

### 1.1 Source Text
Unicode text, UTF‑8.  Line breaks: LF or CRLF.

### 1.2 Keywords
```
fn      if      else    while   let     return  struct  true    false   loop    continue    break   pub     const   static  for     match   enum    union   unsafe  use     mod     extern  actor   trait   impl
```
In Phase 0, the interpreter only needs to recognise the words `fn`, `if`, `else`, `while`, `let`, `return`, `struct`, `true`, `false`, `loop`, `continue`, `break`.  All other keywords may be parsed but cause a “not yet supported” error.

### 1.3 Identifiers
Sequence of letters, digits, underscores (start with letter or `_`).  Raw identifiers `r#...` are **not** required in Phase 0.

### 1.4 Literals
- **Integer literal:** decimal digits only (`123`).  No suffixes; type is `i32`.
- **Boolean literal:** `true`, `false`.
- **String literal:** not supported (skip).
- **Character literal:** not supported.
- No arrays, tuples, floats, etc.

### 1.5 Operators
```
+  -  *  /  %  =  ==  !=  <  >  <=  >=  &&  ||  !  ( )  {  }  ;  ,  ->  .
```
The interpreter must handle these operators in precedence order (see below).

### 1.6 Comments
Single‑line `// ...` only.  Block comments `/* */` are optional but recommended.

---

## 2. Syntax (EBNF – Phase 0)

### 2.1 Program
```
program   = { item } ;
item      = fn_decl | struct_decl ;
```

### 2.2 Function Declaration
```
fn_decl   = "fn" ident "(" param_list ")" block ;
param_list= [ ident { "," ident } ] ;       (* type‑less! all types are i32 *)
block     = "{" { stmt } "}" ;
```
- Parameters are always `i32`.
- Return type is implicit `i32` from the last expression in the body, or `void` if no expression is returned.  (We’ll treat `void` as “no return value” – the function can be called but its return value cannot be used.)
- No `-> type` syntax in Phase 0.

### 2.3 Struct Declaration
```
struct_decl = "struct" ident "{" { ident ":" primitive_type ";" } "}" ;
primitive_type = "i32" | "bool" ;
```
- Only `i32` and `bool` fields.
- No methods, no traits, no constructors – just a grouping of fields.

### 2.4 Statements
```
stmt      = let_stmt | expr_stmt | if_stmt | while_stmt | loop_stmt
           | return_stmt | break_stmt | continue_stmt | block ;
let_stmt  = "let" ident "=" expr ";" ;          (* variables are affine: can be read only once after binding *)
expr_stmt = expr ";" ;
if_stmt   = "if" expr block [ "else" ( if_stmt | block ) ] ;
while_stmt= "while" expr block ;
loop_stmt = "loop" block ;
break_stmt= "break" ";" ;
continue_stmt= "continue" ";" ;
return_stmt= "return" [ expr ] ";" ;
block     = "{" { stmt } "}" ;
```

### 2.5 Expressions (precedence and grammar)
Order of precedence (high to low):
1. Unary `-`, `!`
2. `*` `/` `%`
3. `+` `-`
4. `<` `>` `<=` `>=`
5. `==` `!=`
6. `&&`
7. `||`
8. Assignment `=`

```
expr       = assignment ;
assignment = logical_or { "=" assignment } ;   (* only simple variable names on left *)
logical_or = logical_and { "||" logical_and } ;
logical_and= equality { "&&" equality } ;
equality   = relational { ("==" | "!=") relational } ;
relational = additive { ("<" | ">" | "<=" | ">=") additive } ;
additive   = multiplicative { ("+" | "-") multiplicative } ;
multiplicative = unary { ("*" | "/" | "%") unary } ;
unary      = "-" unary | "!" unary | primary ;
primary    = literal | ident [ "(" args ")" ] | "(" expr ")" | struct_init | field_access ;
args       = [ expr { "," expr } ] ;
struct_init= ident "{" field_init { "," field_init } "}" ;
field_init = ident ":" expr ;
field_access= expr "." ident ;
```

- **Variables** are affine: after a variable is used as an rvalue (read), it cannot be used again.  Example: `let x = 5; let y = x + x;` is **illegal**.  Assignment `x = ...` re‑initialises a dead variable.
- **Function call:** `ident( args )` – calls a previously defined function.
- **Struct construction:** `Point{ x: 10, y: 20 }`.
- **Field access:** `p.x`.

---

## 3. Semantics (Phase 0)

### 3.1 Types
Only `i32` and `bool`.  There is no implicit conversion between them.

### 3.2 Variables (Affine)
- When a local variable is read in an expression, its binding is consumed.
- After consumption, using the variable again is a compile‑time error.
- Variables can be re‑initialised with `=` (must be dead before assignment).
- Parameters of functions are consumed on call; they cannot be used after the call.

### 3.3 Arithmetic
- Integer overflow (e.g., `2147483647 + 1`) panics.  No wrapping.
- Division by zero panics.
- All operators follow standard C semantics otherwise.

### 3.4 Booleans
- `&&`, `||`, `!` work as usual (short‑circuiting not required for Phase 0, but recommended).

### 3.5 Control Flow
- `if` / `else`: condition must be `bool`.
- `while`: loops while condition is true.
- `loop`: infinite loop; use `break` to exit.
- `return` exits the current function; if it has an expression, that value is returned.
- `void` functions are those that end without a return value; they cannot be used in expressions.

### 3.6 Function Calls
- Arguments are passed by value (move).
- Recursion is allowed.  Limit depth to avoid stack overflow.

---

## 4. Testing Requirements for Phase 0

When building `blazei`, you must write tests (in Rust, using the `#[test]` attribute and the standard library’s test harness) for:
- Lexer: tokenising all keywords, operators, integers, booleans.
- Parser: accepting valid programs and rejecting invalid ones.
- Semantic checks: affine usage, type errors, undefined variables.
- Interpreter: evaluating expressions, control flow, function calls, struct allocation, overflow panics.
