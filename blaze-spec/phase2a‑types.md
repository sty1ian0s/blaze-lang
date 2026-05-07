# Blaze Phase 2a – Expanded Type System & Linear Ownership

> **Goal:** Extend the compiler to support all primitive types, compound types, structs, enums, unions, and the full linear type system (move semantics, partial moves, `Dispose`, `@copy`). The parser already accepts the complete syntax from Phase 1; this phase adds semantic analysis and code generation for the new types and ownership rules.

---

## 1. Primitive Types

### 1.1 Integer Types
- Signed: `i8`, `i16`, `i32`, `i64`, `i128`, `isize` (platform‑dependent width).
- Unsigned: `u8`, `u16`, `u32`, `u64`, `u128`, `usize`.
- Integer literals without a suffix default to `i32`. If the literal does not fit in `i32`, it is a compile‑time error.
- Suffixed literals (e.g., `42u64`) produce the corresponding type.

**Operations:**  
All arithmetic (`+`, `-`, `*`, `/`, `%`), bitwise (`&`, `|`, `^`, `<<`, `>>`), and comparison operators are defined for integers.  
- In debug mode, overflow panics. In release mode (to be added later), wrapping two’s complement.  
- Shift amount must be non‑negative; shift by more than the bit width panics.

### 1.2 Floating‑Point Types
- `f16`, `f32`, `f64`, `f128` (IEEE‑754 compliant).  
- Numeric literals with a decimal point or exponent default to `f64`.  
- Suffixed literals (e.g., `3.14f32`) select the type.

**Operations:**  
Arithmetic and comparison operators work.  
- Division by zero returns infinity (sign determined by numerator).  
- `=~` uses the `ApproxEq` trait (default epsilon = 4 × machine epsilon).

### 1.3 Boolean Type
- `bool` with values `true` and `false`.  
- Operators: `!`, `&&`, `||`. Logical operators short‑circuit.

### 1.4 Character and String Types
- `char`: a single Unicode scalar value. Literal `'a'`.  
- `str`: a UTF‑8 string slice, always behind a reference (`&str`).  
- String literals (`"abc"`) produce a reference to a static `str` (like `&'static str`).  
- `Text` (growable string) is defined in `std::string` but not yet available; for now only `&str` is usable.

### 1.5 Unit Type
- `()` is the unit type, with a single value `()`.  
- No operations except equality (`==`) and assignment.

---

## 2. Compound Types

All compound types are linear (move semantics) by default.

### 2.1 Tuples
- Syntax: `(T1, T2, ...)`. Unit `()` is the empty tuple.  
- Tuples can be destructured: `let (a, b) = (1, true);`.  
- Field access via positional indexing is **not** provided; use destructuring.

### 2.2 Arrays
- Fixed‑size: `[T; N]`, where `N` is a compile‑time constant expression.  
- Access: `arr[i]` produces a reference if `arr` is a reference, or moves the element if `arr` is owned.  
- Arrays are linear; moving the array moves all elements.

### 2.3 Slices
- Dynamically‑sized: `[T]`. Always used behind a reference: `&[T]` or `&mut [T]`.  
- Slices cannot be owned directly.

### 2.4 References
- Shared reference: `&T`. Can be freely duplicated (implicitly `@copy` for the reference itself).  
- Mutable reference: `&mut T`. Borrow rules: multiple shared references or one mutable reference may exist, but the compiler only issues a warning on potential aliasing, no hard error.  
- References are scoped to the region they point into (region lifetimes).

### 2.5 Raw Pointers
- `*T` (const) and `*mut T`. Unsafe; dereference requires `unsafe { }`.  
- No automatic disposal.

---

## 3. Algebraic Data Types

### 3.1 Structs
- Named product types: `struct Name { field: Type, ... }`.  
- Fields may have default values: `field: Type = expr`.  
- Construction: `Name { field1: val, field2 }` (if field2 has a default).  
- Access: `obj.field` (moves the field out of the struct, potentially partial move).  
- Structs are linear: assigning the whole struct moves it; moving a field leaves the rest of the struct partially valid until all moved fields are reinitialized.

### 3.2 Enums
- Tagged unions: `enum Name { Variant1, Variant2(T), Variant3 { x: i32, y: i32 } }`.  
- Pattern matching with `match` is not yet implemented (but will come in a later sub‑phase). For now, enums can be constructed and passed around, but their payloads can only be accessed via `unsafe` (or later via `match`).  
- Each variant implicitly defines a constructor.  
- Discriminant values (`= const`) are allowed on empty variants only.

### 3.3 Unions
- All fields share the same memory. Access is `unsafe`.  
- No automatic discriminant; the programmer must track which variant is active.

---

## 4. Linear Type System

All user‑defined types (struct, enum, union) are **linear** unless annotated with `@copy`.

### 4.1 Move Semantics
- Assignment `x = y`, function arguments (by non‑reference), and destructuring move the ownership.  
- After a move, the source binding is uninitialized and cannot be read.  
- Moves are shallow (but all owned sub‑objects are moved).

### 4.2 Partial Moves
- Moving a single field from a struct leaves the other fields still valid.  
- The struct as a whole cannot be used until all moved fields are reinitialized.  
- Reinitialization is done by assigning to the specific fields.

### 4.3 Dispose Trait
- Trait `Dispose` has a method `fn dispose(&mut self)`.  
- At the end of each scope, the compiler inserts calls to `dispose()` for all live variables that implement `Dispose`, in reverse declaration order.  
- If a linear variable does **not** implement `Dispose` and is not explicitly consumed (moved), the compiler emits a compile‑time error.  
- Primitive types and references implement `Dispose` as no‑ops.  
- Structs automatically implement `Dispose` if all fields do (impl generated by compiler).

### 4.4 `@copy` Attribute
- Marks a type as copyable (implicit duplication).  
- Only valid if:
  1. The type’s shallow size ≤ 16 bytes.
  2. All fields are `@copy`.
  3. The type contains no pointers, references, or slices.
- If any condition fails, the compiler rejects with a diagnostic.

---

## 5. Type Checking & Inference

- The compiler now performs full type checking for all supported types.  
- Function signatures must be fully annotated (parameters, return type).  
- Local variables may be inferred from initialiser.  
- Type mismatch errors include the expected and found types.

---

## 6. Code Generation

- Primitive types map to corresponding C types (`i32` → `int32_t`, etc.).  
- Structs become C structs; enums become a tag plus a union.  
- Linear moves are implemented by zeroing or poisoning the source memory.  
- Dispose calls are inserted at scope exits.  
- References are compiled as pointers in C (with the appropriate `const` qualifier).

---

## 7. Layout Attributes

- `@layout(soa)` – on a struct containing arrays: best‑effort transformation to structure‑of‑arrays.  The compiler will reorder memory so that each field’s elements are contiguous, enabling auto‑vectorisation and cache‑friendly iteration.
- `@layout(auto)` – automatic optimal layout (default).  The compiler freely reorders fields and inserts padding to minimise total size while respecting natural alignment.  **When the `std::hardware` module (Phase 3c) is available and a concrete target CPU is specified, the compiler uses the reported cache‑line size (`cache_line_size`), preferred alignment (`preferred_alignment`), and cache capacities (`cache_size`) to fine‑tune the layout for that hardware’s memory hierarchy.  It may, for instance, align hot fields to cache‑line boundaries or group frequently accessed fields together to reduce cache misses.  If no hardware information is available, a generic layout that balances alignment and size is used.**
- `@layout(packed)` – bit‑packed fields; compiler‑generated getters/setters.  Batch SIMD operations may be applied where beneficial.
- `@layout(C)` – C‑compatible layout, no reordering, natural alignment.  The compiler **shall not** apply hardware‑aware optimisations to `@layout(C)` structs, as their layout is fixed by the C ABI.

---

## 8. Testing

For each new feature:

- Write positive tests (programs that compile and run) and negative tests (must be rejected).  
- Test all primitive types with arithmetic, overflow, and casting.  
- Test partial moves and reinitialization.  
- Test `@copy` acceptance and rejection.  
- Test that variables not implementing `Dispose` cause an error when they fall out of scope.  
- All tests must pass before proceeding to Phase 2b.
