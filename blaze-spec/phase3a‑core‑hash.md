# Blaze Phase 3a – Core Library: Hashing (`std::hash`)

> **Goal:** Implement the `std::hash` module exactly as specified.  This module provides the `Hash` and `Hasher` traits, and a default hasher (`DefaultHasher`).

---

## 1. Traits

### 1.1 `Hash`

```
pub trait Hash {
    fn hash<H: Hasher>(&self, state: &mut H);
}
```

- Types implementing `Hash` can be used in hash‑based collections (e.g., `Map`, `Set` – though `Set` is not yet in std).
- The `hash` method feeds the bytes (or structured data) of the value into the provided `Hasher` state.
- Implementing `Hash` manually requires calling `state.write(...)` with representations of the fields; `@derive(Hash)` (not yet fully specified but will be part of `@derive`) will generate the implementation automatically.

### 1.2 `Hasher`

```
pub trait Hasher {
    fn write(&mut self, bytes: &[u8]);
    fn finish(&self) -> u64;
}
```

- `write` feeds raw bytes into the hash computation.
- `finish` returns the final hash value as a `u64`.
- A `Hasher` may be called multiple times; the final `finish` consumes the state and produces the hash.

---

## 2. `DefaultHasher`

```
pub struct DefaultHasher { state: u64 }
```

- `DefaultHasher` provides a basic multiply‑rotate‑xor hashing algorithm.
- The internal algorithm is: for each byte `b`, `state = state.wrapping_mul(31).wrapping_add(b as u64)`.
- `finish` returns the current state.

Implementation:

```
impl DefaultHasher {
    pub fn new() -> Self {
        DefaultHasher { state: 0 }
    }
}

impl Hasher for DefaultHasher {
    fn write(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.state = self.state.wrapping_mul(31).wrapping_add(b as u64);
        }
    }
    fn finish(&self) -> u64 {
        self.state
    }
}
```

---

## 3. Standard Implementations

We provide `Hash` implementations for all primitive types:

- Integers: feed their byte representation.
- Floats: feed their byte representation (but note: `NaN` values are problematic; we treat `NaN` as a special pattern that must be consistent; for simplicity, we will implement by feeding the raw bytes of the float, with the caveat that `0.0` and `-0.0` have different byte patterns and will hash differently; this matches Rust’s behavior).  This will be documented.
- `bool`: feed a single byte (0 or 1).
- `char`: feed the 4‑byte scalar value.

For aggregate types, the `@derive(Hash)` macro will feed each field in order.

---

## 4. Testing

- Create a `DefaultHasher`, feed known byte sequences, and verify `finish` produces expected results (deterministic).
- Test `Hash` on integer types: two equal values produce the same hash; different values (usually) produce different hashes.
- Test `Hash` on strings (via `Text::as_str`): identical strings hash identically.
- Test that mutable hasher state can be built incrementally.
- All tests must pass before moving to the next module.
