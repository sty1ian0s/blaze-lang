# Blaze Phase 3b – Systems Library: Endianness (`std::endian`)

> **Goal:** Implement the `std::endian` module exactly as specified.  This module provides host‑endianness detection and byte‑swapping functions for all primitive integer and floating‑point types.  All functions are pure and have zero runtime overhead beyond the necessary bit manipulation.

---

## 1. Host Endianness

### 1.1 `Endianness` Enum

```
pub enum Endianness {
    Little,
    Big,
}
```

- Represents the byte order of the target platform.

### 1.2 `endianness` Function

```
pub fn endianness() -> Endianness;
```

- Returns `Endianness::Little` if the host is little‑endian, `Endianness::Big` if big‑endian.  
- The value is determined at compile time (the compiler supplies the constant `target_endian`), so the function returns a compile‑time constant.  The implementation uses `@cfg(target_endian = "little")` and `@cfg(target_endian = "big")` to select the branch, making it zero‑cost.

---

## 2. Byte‑Swap Functions

All byte‑swap functions take a value and return the value with its bytes reversed.  The functions are pure (empty effect set) and are implemented using the compiler’s built‑in bit‑rotation intrinsics (e.g., `__builtin_bswap32`) when available, or manually via shift and mask operations.

### 2.1 Integer Swaps

```
pub fn swap_u16(x: u16) -> u16;
pub fn swap_u32(x: u32) -> u32;
pub fn swap_u64(x: u64) -> u64;
pub fn swap_i16(x: i16) -> i16;
pub fn swap_i32(x: i32) -> i32;
pub fn swap_i64(x: i64) -> i64;
```

- The signed integer swaps reinterpret the value as an unsigned integer, swap bytes, and reinterpret back; overflow semantics are irrelevant because the bit pattern is preserved.

### 2.2 Floating‑Point Swaps

```
pub fn swap_f32(x: f32) -> f32;
pub fn swap_f64(x: f64) -> f64;
```

- These functions reinterpret the float as its equivalent unsigned integer (`u32` / `u64`), swap bytes, and reinterpret the result back to the float type.  This operation is sound for any IEEE‑754 bit pattern, including `NaN` and infinities.

---

## 3. Implementation Guidance

- The module will use conditional compilation attributes (`@cfg(target_endian = "little")`) to select the correct implementation of `endianness()` at compile time, emitting a `panic!` for an unrecognized target.
- Byte‑swap functions are implemented using the `{integer}::swap_bytes()` method which the compiler already provides for integers (since Phase 2a).  If those methods are not yet available, we implement them directly with shift and mask.
- The floating‑point swaps rely on `std::mem::transmute` (or `unsafe` pointer casts) to reinterpret the float as an unsigned integer.  This is safe because `f32` and `u32` have the same size and alignment, and no invalid values exist.

---

## 4. Testing

- **`endianness()`:** Verify that the returned value matches the expected target endianness as known from the build configuration.
- **Byte‑swap integers:** For each type, swap a value twice and check it equals the original.  For specific bit patterns (e.g., `0x12345678` on a 32‑bit type), verify the swapped value is the byte‑reversed value (by comparing to a precomputed constant).
- **Byte‑swap floats:** Swap twice to check identity; also verify that the byte pattern after swapping corresponds to the integer swap of the raw bits.
- Ensure all functions are pure (no effect annotations required, but test that they can be called from pure contexts).

All tests must pass before moving to the next module.
