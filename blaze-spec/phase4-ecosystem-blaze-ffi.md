# Phase 4 – Ecosystem Crate: `blaze‑ffi`

> **Goal:** Provide a safe, data‑oriented Foreign Function Interface (FFI) builder for Blaze.  It generates C‑compatible bindings for Blaze functions, structs, and enums, allowing them to be called from other languages (C, C++, Python, Java, etc.) without manual unsafe code.  The crate leverages `@comptime` macros and the reflection system to automatically produce C headers, shared library exports, and any necessary conversion logic.  All Blaze‑side code remains pure and safe; only the generated C shims contain the necessary `unsafe` blocks, which are validated at compile time.

---

## 1. Core Concepts

- **`#[ffi_export]`** – attribute placed on `fn` and `struct` to mark them for FFI export.
- **`#[ffi_import]`** – attribute on `extern` blocks to import C functions and types.
- **`blaze‑ffi‑tool`** – a command‑line tool that processes a Blaze crate and emits a `.h` header file and a `.so`/`.dll` shared library.
- **`CType`** – a trait mapping Blaze types to C types (`i32` → `int32_t`, `Text` → `const char*`, etc.).
- **`CStr`**, **`CArray`**, **`COwnedPtr`** – wrapper types for safe handling of C pointers and strings.

All export/import operations are pure (they are resolved at compile time); shared library loading carries the `io` effect.

---

## 2. `#[ffi_export]` Macro

### 2.1 Exported Functions

```
#[ffi_export]
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}
```

The macro generates:

- A C‑compatible wrapper function (e.g., `int32_t add(int32_t a, int32_t b)`) that follows the C ABI.
- Automatic conversion of parameters and return types using the `CType` trait.
- A corresponding declaration in the generated header.

### 2.2 Exported Structs

```
#[ffi_export]
#[repr(C)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}
```

- The struct must be `#[repr(C)]` to guarantee a stable memory layout.  The macro verifies this at compile time.
- A C struct with the same layout is emitted in the header, along with functions to allocate, free, and access fields if the struct is opaque.

### 2.3 Exported Enums

```
#[ffi_export]
pub enum Status { Ok, Error }
```

- Enums are exported as C `enum` types with the same discriminants.  If the enum carries payloads, the macro produces an opaque struct and accessor functions (or emits a compile error if the enum is not `#[repr(C)]` compatible).

---

## 3. `#[ffi_import]` Macro

```
#[ffi_import]
extern "C" {
    fn malloc(size: usize) -> *mut u8;
    fn free(ptr: *mut u8);
}
```

- The macro validates that the imported signatures match the C ABI and generates safe wrappers where possible.
- For `malloc`/`free`, the crate provides safe alternatives (`CBox<T>`) that wrap the raw pointer.

---

## 4. `CType` Trait

```
pub trait CType {
    type CEquivalent;
    fn to_c(self) -> Self::CEquivalent;
    unsafe fn from_c(c: Self::CEquivalent) -> Self;
}
```

- Implemented for all primitive types (`i32`, `f64`, `bool`), `Text` (→ `*const c_char`), `Vec<T>` (→ `*const T` + `length`), and `Option<T>` (→ nullable pointer).
- For complex types (structs, enums), the user can manually implement `CType` or rely on the generated conversion functions.

---

## 5. `blaze‑ffi‑tool`

Invoked as:

```
blaze ffi --crate mylib --out-dir ffi/
```

It:

1. Scans the crate for all `#[ffi_export]` items using the compiler’s reflection.
2. Generates a `mylib.h` header with C declarations.
3. Compiles the crate to a shared library (`libmylib.so` / `mylib.dll`).
4. Optionally generates a Python `ctypes` wrapper, a C# P/Invoke file, or a Java JNI wrapper (behind feature flags `pyo3`, `jni`).

---

## 6. Safety Guarantees

- The generated C code is free of undefined behavior under the Blaze memory model, provided the C caller respects the documented API.
- Linear types that are transferred across FFI must be explicitly consumed (e.g., via a `free` function) or wrapped in `COwnedPtr<T>`, which acts like `std::unique_ptr`.
- `blaze‑ffi` does not allow exporting functions that would violate linearity or that pass `&T` references without a clear lifetime contract; the macro emits a compile‑time error if the function signature is unsound.

---

## 7. Error Handling

```
pub enum FfiError {
    Io(std::io::Error),
    UnsupportedType(Text),
    LayoutMismatch(Text),
    LinkError(Text),
}
```

---

## 8. Testing

- **Export a function:** Export a simple add function, compile to a shared library, call it from a small C test program, verify the result.
- **Export a struct:** Export a `Point` struct, call a Blaze function that returns one, verify the fields in C.
- **Import a C function:** Import `abs` from libc, call it from Blaze, verify.
- **Safety:** Attempt to export a function that takes `&mut T` without a clear lifetime; verify the macro rejects it.

All tests require a C compiler and a platform capable of dynamic linking.
