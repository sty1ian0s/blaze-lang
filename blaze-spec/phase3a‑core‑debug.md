# Blaze Phase 3a – Core Library: Debug (`std::debug`)

> **Goal:** Implement the `std::debug` module exactly as specified.  This module re‑exports the `Debug` trait from `std::fmt` and provides no additional items.

---

## 1. Re‑export

```
pub use std::fmt::Debug;
```

The `std::debug` module exists solely to provide a canonical home for the `Debug` trait.  Code can import `std::debug::Debug` instead of `std::fmt::Debug`, though both are equivalent.

No other items are defined in this module.

---

## 2. Testing

- Verify that `use std::debug::Debug;` brings the `Debug` trait into scope.
- Ensure that types implementing `Debug` (either manually or via `@derive(Debug)`) can be formatted with the same behavior as if they used `std::fmt::Debug`.
- Write a test that uses the fully qualified path `std::debug::Debug` and see that it works identically to `std::fmt::Debug`.

All tests must pass before moving to the next module.
