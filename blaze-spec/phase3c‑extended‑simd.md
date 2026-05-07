# Blaze Phase 3c – Extended Library: SIMD (`std::simd`)

> **Goal:** Implement the `std::simd` module exactly as specified.  This module provides architecture‑specific SIMD vector types, operations, and compiler intrinsics for high‑performance parallel computation.  The module is only available on targets that support the required instructions, and falls back to scalar equivalents on unsupported platforms.

---

## 1. Vector Types

The module defines common SIMD vector types for 128‑bit, 256‑bit, and 512‑bit widths, parameterised by element type.  The element types supported are `i8`, `i16`, `i32`, `i64`, `u8`, `u16`, `u32`, `u64`, `f16`, `f32`, `f64`.

### 1.1 128‑bit vectors

```
pub struct i8x16(i8, i8, i8, i8, i8, i8, i8, i8, i8, i8, i8, i8, i8, i8, i8, i8);
pub struct i16x8(i16, i16, i16, i16, i16, i16, i16, i16);
pub struct i32x4(i32, i32, i32, i32);
pub struct i64x2(i64, i64);
pub struct u8x16(u8, u8, u8, u8, u8, u8, u8, u8, u8, u8, u8, u8, u8, u8, u8, u8);
pub struct u16x8(u16, u16, u16, u16, u16, u16, u16, u16);
pub struct u32x4(u32, u32, u32, u32);
pub struct u64x2(u64, u64);
pub struct f16x8(f16, f16, f16, f16, f16, f16, f16, f16);
pub struct f32x4(f32, f32, f32, f32);
pub struct f64x2(f64, f64);
```

### 1.2 256‑bit vectors

```
pub struct i8x32( /* 32 elements */ );
pub struct i16x16( /* 16 elements */ );
pub struct i32x8( /* 8 elements */ );
pub struct i64x4( /* 4 elements */ );
pub struct u8x32( /* 32 elements */ );
pub struct u16x16( /* 16 elements */ );
pub struct u32x8( /* 8 elements */ );
pub struct u64x4( /* 4 elements */ );
pub struct f16x16( /* 16 elements */ );
pub struct f32x8( /* 8 elements */ );
pub struct f64x4( /* 4 elements */ );
```

### 1.3 512‑bit vectors

```
pub struct i8x64( /* 64 elements */ );
pub struct i16x32( /* 32 elements */ );
pub struct i32x16( /* 16 elements */ );
pub struct i64x8( /* 8 elements */ );
pub struct u8x64( /* 64 elements */ );
pub struct u16x32( /* 32 elements */ );
pub struct u32x16( /* 16 elements */ );
pub struct u64x8( /* 8 elements */ );
pub struct f16x32( /* 32 elements */ );
pub struct f32x16( /* 16 elements */ );
pub struct f64x8( /* 8 elements */ );
```

Each vector type is annotated `@copy` (they are small and contain no pointers).  The internal representation uses the platform’s SIMD registers when available, or falls back to a tuple of scalars.

---

## 2. Operations

All vector types support element‑wise arithmetic, bitwise, and comparison operators via the operator overloading traits (`Add`, `Sub`, `Mul`, `Div`, `Rem`, `BitAnd`, `BitOr`, `BitXor`, `Shl`, `Shr`, `Not`).  The operations are implemented using the underlying SIMD instruction set.

Additionally, each vector type provides the following inherent methods:

```
impl i32x4 {    // example for one type; all types provide analogous methods
    pub fn splat(val: i32) -> Self;
    pub fn new(a: i32, b: i32, c: i32, d: i32) -> Self;
    pub fn extract(self, index: u32) -> i32;
    pub fn replace(self, index: u32, val: i32) -> Self;
    pub fn load_unaligned(ptr: *const i32) -> Self;
    pub fn store_unaligned(self, ptr: *mut i32);
}
```

- `splat`: creates a vector where all lanes have the same value.
- `new`: creates a vector from individual lane values (for 128‑bit types the constructor is a simple tuple; for wider types we rely on a compiler‑generated constructor).
- `extract`: returns the value of the lane at `index` (0‑based).  Panics if index out of bounds.
- `replace`: returns a new vector with the lane at `index` replaced by `val`.  Panics on out‑of‑bounds index.
- `load_unaligned`: reads a vector from memory; alignment is not required.  `unsafe` because the pointer must be valid and point to enough memory.
- `store_unaligned`: writes the vector to memory.  `unsafe` for same reasons.

For floating‑point vectors, additional methods are provided: `abs`, `min`, `max`, `sqrt`, `floor`, `ceil`, `round`, `trunc`.

---

## 3. Compiler Intrinsics

The module re‑exports compiler intrinsics for advanced operations (e.g., shuffles, permutations, reductions).  These intrinsics are `unsafe` and only available on specific target architectures.

```
pub unsafe fn shuffle_i32x4(a: i32x4, b: i32x4, mask: i8x4) -> i32x4;
```

Each intrinsic is guarded by `@cfg(target_feature = "...")`.  The exact set of intrinsics is documented in a separate compiler manual and not repeated here; the module must provide at least the above operations.

---

## 4. Feature Detection

### 4.1 Runtime Detection

```
pub fn is_x86_feature_detected(feature: &str) -> bool;
```

On x86 platforms, queries whether a CPU feature (e.g., `"sse4.2"`, `"avx2"`) is available.  Returns `false` on unsupported platforms.

```
pub fn is_arm_feature_detected(feature: &str) -> bool;
```

Similarly for ARM (e.g., `"neon"`).

### 4.2 Compile‑time Features

`@cfg(target_feature = "avx2")` allows conditional compilation based on enabled features.

---

## 5. Fallback on Unsupported Platforms

When a target does not support SIMD, the vector types are represented as tuples of scalars, and all operations are implemented with element‑wise loops.  The compiler must generate the fallback code automatically.  The `std::simd` module itself does not contain fallback implementations; it relies on the compiler’s monomorphisation and code generation to produce scalar code when no SIMD instructions are available.

---

## 6. Testing

- **Construction and splat:** Create vectors using `splat` and `new`, extract lanes and verify values.
- **Arithmetic:** Test `+`, `-`, `*`, `/` on integer and float vectors, comparing each lane with scalar operations.
- **Load/Store:** Write a vector to memory, read back, verify.
- **Feature detection:** Run `is_x86_feature_detected` and check that known features return expected values (e.g., on x86‑64, `"sse"` should almost always be true).  (This test only runs on the appropriate hardware.)
- **Fallback:** On a platform without SIMD, verify that vector types still work correctly (scalar fallback).

All tests must pass before moving to the next module.
