# Blaze Phase 3a – Core Library: Formatting (`std::fmt`)

> **Goal:** Implement the `std::fmt` module exactly as specified.  This module provides string formatting, the `Display` and `Debug` traits, and the `print` / `println` functions.

---

## 1. Public Functions

```
pub fn print(args: Arguments);
pub fn println(args: Arguments);
```

- `print` writes the formatted `Arguments` to standard output (via `stdout().write_fmt(args)`).  It does not add a newline.
- `println` calls `print(args)` followed by `stdout().write_str("\n")`.  Both functions panic if writing fails (via `unwrap()` on the result, but the specification says printing never fails; we implement by calling `unwrap()` after each write).

The `Arguments` type is produced exclusively by the `format!` macro (defined in `std::builtins`).  Users cannot construct `Arguments` directly; they can only use `format!` to create a `String` (i.e., `Text`) or pass the result to `print`/`println`.

---

## 2. `Arguments` Type

```
pub struct Arguments { /* opaque */ }
```

`Arguments` is an opaque structure; its layout is compiler‑defined.  For our implementation, it holds a reference to a static byte sequence (the formatted result of the macro expansion) plus any dynamic arguments that were interpolated.  The compiler replaces `format!("...", args...)` with a call to a built‑in compiler function that creates an `Arguments` value.  The exact internal representation is:

- For simple cases (no dynamic args), `Arguments` is just a pointer to a null‑terminated string literal.
- For dynamic arguments, the compiler generates a temporary stack‑allocated buffer and serializes each argument using its `Display` or `Debug` implementation.  The actual formatting logic is compiler‑intrinsic and not user‑definable.

Because this module is implemented after the compiler already supports `format!` (Phase 1 already accepted the syntax; the compiler intrinsic is added now), we only need to supply the runtime side: the `print` and `println` functions that write the already‑formatted string.

---

## 3. `Formatter` Type

```
pub struct Formatter { /* private */ }
```

`Formatter` is an opaque structure that represents a destination for formatted output.  It is used by `Display` and `Debug` implementations.

### Methods

```
impl Formatter {
    pub fn write_str(&mut self, s: &str) -> Result<(), Error>;
    pub fn write_fmt(&mut self, args: Arguments) -> Result<(), Error>;
}
```

- `write_str(s)`: Appends the given string to the formatter’s output.  Returns `Ok(())` or an error if the underlying writer fails.
- `write_fmt(args)`: Formats the `Arguments` into the formatter’s output (this is essentially a passthrough to the compiler’s formatting logic).  In our implementation, `write_fmt` will just write the already‑formatted byte sequence to the output buffer.

The `Formatter` struct is used internally by the `write!` macro (not yet implemented) and by user‑written `Display`/`Debug` implementations.  For now, we provide a minimal `Formatter` that can be constructed from a `Vec<u8>` or other target; this will be extended later.

---

## 4. `Error` Type

```
pub type Error = ();
```

Formatting operations are infallible; any “error” is impossible in our design because the compiler ensures the format string and arguments match.  Therefore, the `Error` type is simply the unit type `()`.  All operations return `Result<(), ()>` for compatibility with the trait signatures, but `Ok(())` is always returned.

---

## 5. Traits

### 5.1 `Display`

```
pub trait Display {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error>;
}
```

Types implementing `Display` provide a human‑readable representation.  The implementation must use the `Formatter` methods (preferably `write_str`) to output the representation.

Example (manual implementation):

```
impl Display for i32 {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        // convert self to string and write
        let s = ...;
        f.write_str(s)
    }
}
```

The `@derive(Display)` macro (from `@derive`) generates an implementation that prints the type name and field values.

### 5.2 `Debug`

```
pub trait Debug {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error>;
}
```

Similar to `Display`, but intended for debugging output.  The `@derive(Debug)` macro generates a representation that includes the type name and each field’s `Debug` output.

---

## 6. Internal Implementation Details

The `Formatter` struct contains a mutable reference to a buffer (`Vec<u8>`) that accumulates the output.  When `write_str` is called, it appends the bytes to that buffer.  The actual `print`/`println` functions will:

1. Obtain a `Formatter` backed by a new `Vec<u8>` (or directly write to stdout via a line‑buffered wrapper).
2. Call `args.write_fmt(&mut fmt)` (where `args` is the `Arguments` produced by the macro).  The compiler‑generated code for `Arguments::write_fmt` iterates over the format string pieces and argument values, calling their `Display::fmt` methods.
3. After formatting, write the buffer to stdout.

Because the compiler already handles the format string parsing and argument dispatch, the runtime side is relatively thin.

---

## 7. Integration with `printf`-style formatting

The `format!` macro is defined in `std::builtins` as a compiler intrinsic.  Its exact syntax and semantics are identical to C’s `printf` format specifiers (e.g., `%d`, `%s`, `%f`, etc.), with the following exceptions:

- `%?` uses the `Debug` trait instead of `Display`.
- `%%` writes a literal `%`.
- The macro returns a `Text` (i.e., `String`) containing the result.

For `print`/`println`, the formatting is done at the call site by the compiler, producing an `Arguments` value, which is then passed to the runtime functions.  The `Arguments` internally stores the result of formatting (or the instructions to format) and, when written to a `Formatter`, simply emits the already‑formatted string.

---

## 8. Testing

- **`print`/`println`:** Call these functions with simple string literals and verify the output on stdout (capture stdout in tests).  Ensure that `println` appends a newline.
- **`Display` and `Debug` for built‑ins:** Implement `Display` and `Debug` for `i32`, `bool`, `Text`, and verify they produce expected strings (e.g., `42`, `true`, `"hello"`).
- **Derived `Display`/`Debug`:** (These rely on `@derive`, which is not yet fully implemented in this phase; manual implementations for test structs are sufficient.)
- **Error handling:** All formatting operations must return `Ok(())` – test that `Result` is always `Ok`.
- All tests pass before moving to the next module.
