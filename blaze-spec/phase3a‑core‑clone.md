# Blaze Phase 3a – Core Library: Clone (`std::clone`)

> **Goal:** Implement the `std::clone` module exactly as specified.  This module provides the `Clone` trait for explicit duplication of values.

---

## 1. `Clone` Trait

```
pub trait Clone {
    fn clone(&self) -> Self;
}
```

- `Clone` provides a way to explicitly duplicate a value.  Unlike `@copy` (which is implicit), `Clone` requires an explicit call to `.clone()` and can be implemented for types that manage resources (e.g., heap‑allocated strings, file handles that need duplication via OS calls, etc.).
- Types annotated with `@copy` automatically implement `Clone` where `clone()` simply copies the value.
- Types that implement `Dispose` may also implement `Clone` if a deep copy is possible; the `clone` method must produce a new instance that is independently disposed.

---

## 2. Standard Implementations

We provide `Clone` implementations for all primitive types (they simply return the value) and for common standard library types (once they are implemented):

- All integer and floating‑point types: `Clone` via trivial copy.
- `bool`, `char`: trivial copy.
- `()`: trivial copy.
- `&T`: cloning a reference just copies the reference (not the underlying data).
- `Text` (string): already implements `Clone` (deep copy) as defined in `std::string`.
- `Option<T>`, `Result<T,E>`: implement `Clone` if their inner types implement `Clone`.
- `Vec<T>`: will implement `Clone` when `T: Clone`.
- Other collections (`List`, `Map`, etc.) will implement `Clone` where appropriate.

The `@derive(Clone)` macro (in `@derive`) generates an implementation that clones each field.

---

## 3. Testing

- Verify that primitive types can be cloned and that the cloned value equals the original (using `==`).
- Test `Text::clone()` produces an independent string (modifying one does not affect the other).
- Define a custom struct with a `Clone` implementation that logs or counts calls, and ensure it’s only called when `.clone()` is invoked.
- For types that do not implement `Clone`, ensure the compiler rejects `.clone()` calls (this test is more about type checking, so it can be deferred if not yet possible).
- All tests must pass before moving to the next module.
