# Blaze Phase 3a – Core Library: Comparison (`std::cmp`)

> **Goal:** Implement the `std::cmp` module exactly as specified.  This module provides traits for equality, ordering, and approximate equality, along with the `Ordering` enum.

---

## 1. Traits

### 1.1 `PartialEq`

```
pub trait PartialEq {
    fn eq(&self, other: &Self) -> bool;
}
```

- `PartialEq` allows partial equivalence relations.  Implementations must be symmetric and transitive.
- The `==` operator desugars to `PartialEq::eq`.
- There is a blanket implementation for all types that can be compared with `==` using the compiler‑derived implementation when `@derive(PartialEq)` is used; manual impls are also allowed.

### 1.2 `Eq`

```
pub trait Eq: PartialEq {}
```

- `Eq` is a marker trait that indicates an equivalence relation (reflexive, symmetric, transitive).  It adds no methods; it just serves as a guarantee that the implementation of `PartialEq` is a true equivalence.
- All primitive types (`i32`, `bool`, `f32`, etc.) implement `Eq` (for floats, `Eq` is not implemented because `NaN` breaks reflexivity; however, for floating‑point, we only provide `PartialEq`).  Actually, `f64` does **not** implement `Eq` because `NaN` is not equal to itself.  So only integer and boolean types implement `Eq`.

### 1.3 `Ordering`

```
pub enum Ordering {
    Less,
    Equal,
    Greater,
}
```

This enum represents the result of a comparison between two values.

### 1.4 `PartialOrd`

```
pub trait PartialOrd: PartialEq {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering>;
}
```

- Returns `Some(Less)` if `self < other`, `Some(Greater)` if `self > other`, `Some(Equal)` if they are equal, and `None` if they are incomparable (like `NaN` for floats).
- The `<`, `>`, `<=`, `>=` operators desugar to `partial_cmp` with appropriate `Ordering` checks.

### 1.5 `Ord`

```
pub trait Ord: Eq + PartialOrd {
    fn cmp(&self, other: &Self) -> Ordering;
}
```

- `Ord` guarantees a total order.  `cmp` must return `Less`, `Equal`, or `Greater`.
- All integer types and `bool` implement `Ord`.

### 1.6 `ApproxEq`

```
pub trait ApproxEq {
    fn approx_eq(&self, other: &Self, epsilon: Self) -> bool;
}
```

- Used by the `=~` operator for approximate floating‑point equality.
- The default epsilon used by `=~` is `4 * EPSILON` of the involved floating‑point type (where `EPSILON` is the machine epsilon).  Users may override per module with `@float_epsilon(val)` (the compiler handles that substitution).
- Implementations for `f32`, `f64` check if the absolute difference is less than `epsilon`.

---

## 2. Standard Implementations

We provide implementations of these traits for all primitive types:

- `i8`, `i16`, `i32`, `i64`, `i128`, `isize` → `PartialEq`, `Eq`, `PartialOrd`, `Ord`.
- `u8`, `u16`, `u32`, `u64`, `u128`, `usize` → same.
- `f16`, `f32`, `f64`, `f128` → `PartialEq`, `PartialOrd` (no `Eq`/`Ord`), and `ApproxEq`.
- `bool` → `PartialEq`, `Eq`, `PartialOrd`, `Ord`.
- `char` → `PartialEq`, `Eq`, `PartialOrd`, `Ord`.
- `()` → `PartialEq`, `Eq`, `PartialOrd`, `Ord` (only one value).
- `&T` → `PartialEq` if `T: PartialEq` (pointer equality? Actually reference equality is not what we want. In Blaze, references are compared by value, so `&T` delegates to `T::eq`.)

For aggregate types (structs, enums, tuples), the `@derive` attribute will generate the appropriate implementations.  The compiler does that in Phase 2f.  For now, we rely on manual implementations for test structs.

---

## 3. Derived Implementations (Compiler)

The compiler's `@derive` knows how to generate:

- `PartialEq` for a struct: chain `eq` on each field with `&&`.  For an enum, first compare discriminants, then delegating to the variant’s field(s).
- `Eq` is added automatically if all fields are `Eq` and the struct is not a union/enum with floating‑point fields (but actually enums can still be `Eq` if all payload fields are `Eq`).
- `PartialOrd` and `Ord`: lexicographic ordering based on field order.  For enums, discriminants are ordered by declaration order.

---

## 4. Testing

- For each primitive type, test equality and ordering operators against expected results.  Cover edge cases like `i32::MIN`, `i32::MAX`, floats `NaN`, `infinity`.
- Test `ApproxEq`: compare equal and unequal floats with various epsilons.  Ensure `=~` operator works as expected in expressions.
- Write a struct with multiple fields, manually implement `PartialEq` and `Ord`, then test comparisons.
- Verify that types without `Eq` (e.g., `f64`) cannot be used where `Eq` is required (check that the compiler correctly rejects them, though this belongs to earlier phases; just confirm with a simple test).
- All tests must pass before moving to the next module.
