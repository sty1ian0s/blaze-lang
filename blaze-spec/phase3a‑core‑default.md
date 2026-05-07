# Blaze Phase 3a – Core Library: Default (`std::default`)

> **Goal:** Implement the `std::default` module exactly as specified.  This module provides the `Default` trait for creating default values of a type.

---

## 1. `Default` Trait

```
pub trait Default {
    fn default() -> Self;
}
```

- `Default` provides a standard way to create a “default” value for a type.
- It is used by `@derive(Default)` (which generates an implementation that calls `default()` on each field) and by generic code that needs to produce a value of a type without knowing a specific constructor.
- For structs with field defaults, the compiler automatically provides a `Default` implementation that uses those field defaults.  If any field does not have a default, the struct must implement `Default` manually or via `@derive(Default)`.
- Numeric types implement `Default` with `0` (or `0.0` for floats).  `bool` defaults to `false`.  `char` defaults to `'\0'` (null character).  `()` defaults to `()`.
- `Option<T>` defaults to `None`.  `Result<T,E>` does not implement `Default` because it’s not clear whether to produce `Ok` or `Err`.
- `Text` (string) defaults to an empty string.
- `Vec<T>` defaults to an empty vector.
- All of these implementations must be provided.

---

## 2. Standard Implementations

- All integer types: `default()` returns `0`.
- All floating‑point types: `default()` returns `0.0`.
- `bool`: `false`.
- `char`: `'\0'`.
- `Option<T>`: `None`.
- `Text`: `Text::new()`.
- `Vec<T>`: empty vector.

Additionally, user‑defined structs can derive `Default` via `@derive(Default)`, as handled in Phase 2f.

---

## 3. Testing

- For each primitive type, call `Default::default()` and assert it equals the expected zero/false value.
- Test that `Option::<i32>::default()` is `None`.
- Test that `Text::default()` is an empty string.
- Define a struct with fields that have defaults, implement `Default` via derive, and check that the struct’s default matches the field defaults.
- All tests must pass before moving to the next module.
