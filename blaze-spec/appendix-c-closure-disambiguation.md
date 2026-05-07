# Appendix C – Closure Disambiguation

> **Status:** Normative.  This appendix defines the exact lexical and syntactic disambiguation rules for closure expressions in Blaze.

---

## C.1 Lexical Introduction

A closure expression begins with the token `\` (U+005C, REVERSE SOLIDUS).  This character is not used for any other purpose in the Blaze grammar, making it unambiguous as a closure introducer.

## C.2 Syntax

```
closure_expr = "\" param_list "->" expr
```

- After `\`, the parser expects a `param_list` (identical to function parameter lists, but with optional type annotations).
- The token `->` separates the parameter list from the closure body.
- The body is an arbitrary expression, which may be a block expression `{ … }`.

## C.3 Parameter List

The parameter list may be empty, written as `||` (but in Blaze the syntax is `\` followed by parentheses):

```
\() -> expr
\(x) -> expr
\(x, y) -> expr
\(x: i32, y: i32) -> { x + y }
```

Types on parameters are optional.  If omitted, the compiler infers them from the closure body.

## C.4 Disambiguation from Other Constructs

- The `\` character is never used as a binary or unary operator in Blaze.
- The `||` token (logical OR) cannot be confused with an empty closure parameter list because Blaze does not use `||` for closures; the empty parameter list is `()`.
- A closure with a single parameter and no parentheses is allowed: `\x -> x + 1`.  This is distinct from a lambda in other languages; there is no ambiguity because `\` is not a valid expression start except for closures.

## C.5 Examples

```blaze
let add_one = \x -> x + 1;
let sum = \(a, b) -> a + b;
let run = \() -> { println("Hello"); };
let complex = \(x: i32, y: i32) -> { let z = x + y; z * z };
```
