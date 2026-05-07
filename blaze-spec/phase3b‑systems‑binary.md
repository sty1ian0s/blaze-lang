# Blaze Phase 3b – Systems Library: Binary Data (`std::binary`)

> **Goal:** Implement the `std::binary` module exactly as specified.  This module provides utilities for encoding and decoding binary data, including reading and writing primitive types from byte buffers in a zero‑cost, unsafe‑free manner using references and slices.

---

## 1. Module Overview

The `std::binary` module is designed for low‑level binary I/O when working with network protocols, file formats, and hardware registers.  It operates on `&[u8]` slices for reading and `&mut [u8]` slices for writing, enforcing all bounds checks in debug mode and eliding them when proven safe by the compiler.

---

## 2. Functions

All functions in this module are pure (empty effect set) and work directly on byte slices.

### 2.1 Reading Functions

```
pub fn read_u8(buf: &[u8]) -> Option<(u8, &[u8])>;
pub fn read_u16_be(buf: &[u8]) -> Option<(u16, &[u8])>;
pub fn read_u16_le(buf: &[u8]) -> Option<(u16, &[u8])>;
pub fn read_u32_be(buf: &[u8]) -> Option<(u32, &[u8])>;
pub fn read_u32_le(buf: &[u8]) -> Option<(u32, &[u8])>;
pub fn read_u64_be(buf: &[u8]) -> Option<(u64, &[u8])>;
pub fn read_u64_le(buf: &[u8]) -> Option<(u64, &[u8])>;
pub fn read_i8(buf: &[u8]) -> Option<(i8, &[u8])>;
pub fn read_i16_be(buf: &[u8]) -> Option<(i16, &[u8])>;
pub fn read_i16_le(buf: &[u8]) -> Option<(i16, &[u8])>;
pub fn read_i32_be(buf: &[u8]) -> Option<(i32, &[u8])>;
pub fn read_i32_le(buf: &[u8]) -> Option<(i32, &[u8])>;
pub fn read_i64_be(buf: &[u8]) -> Option<(i64, &[u8])>;
pub fn read_i64_le(buf: &[u8]) -> Option<(i64, &[u8])>;
pub fn read_f32_be(buf: &[u8]) -> Option<(f32, &[u8])>;
pub fn read_f32_le(buf: &[u8]) -> Option<(f32, &[u8])>;
pub fn read_f64_be(buf: &[u8]) -> Option<(f64, &[u8])>;
pub fn read_f64_le(buf: &[u8]) -> Option<(f64, &[u8])>;
```

- Each function returns `None` if the buffer does not contain enough bytes for the type.
- On success, it returns the decoded value and a reference to the remaining bytes.
- Little‑endian (`_le`) variants interpret the bytes in the host‑native byte order (Blaze assumes little‑endian for its internal representation, but the `_le` functions explicitly assume little‑endian input regardless of host, while `_be` assumes big‑endian).  The implementation swaps bytes if the host’s endianness differs (see `std::endian` for helpers).  For now, we assume the host matches the function suffix; later we will use the `std::endian` module to be platform‑agnostic.

### 2.2 Writing Functions

```
pub fn write_u8(buf: &mut [u8], val: u8) -> Option<&mut [u8]>;
pub fn write_u16_be(buf: &mut [u8], val: u16) -> Option<&mut [u8]>;
pub fn write_u16_le(buf: &mut [u8], val: u16) -> Option<&mut [u8]>;
pub fn write_u32_be(buf: &mut [u8], val: u32) -> Option<&mut [u8]>;
pub fn write_u32_le(buf: &mut [u8], val: u32) -> Option<&mut [u8]>;
pub fn write_u64_be(buf: &mut [u8], val: u64) -> Option<&mut [u8]>;
pub fn write_u64_le(buf: &mut [u8], val: u64) -> Option<&mut [u8]>;
pub fn write_i8(buf: &mut [u8], val: i8) -> Option<&mut [u8]>;
pub fn write_i16_be(buf: &mut [u8], val: i16) -> Option<&mut [u8]>;
pub fn write_i16_le(buf: &mut [u8], val: i16) -> Option<&mut [u8]>;
pub fn write_i32_be(buf: &mut [u8], val: i32) -> Option<&mut [u8]>;
pub fn write_i32_le(buf: &mut [u8], val: i32) -> Option<&mut [u8]>;
pub fn write_i64_be(buf: &mut [u8], val: i64) -> Option<&mut [u8]>;
pub fn write_i64_le(buf: &mut [u8], val: i64) -> Option<&mut [u8]>;
pub fn write_f32_be(buf: &mut [u8], val: f32) -> Option<&mut [u8]>;
pub fn write_f32_le(buf: &mut [u8], val: f32) -> Option<&mut [u8]>;
pub fn write_f64_be(buf: &mut [u8], val: f64) -> Option<&mut [u8]>;
pub fn write_f64_le(buf: &mut [u8], val: f64) -> Option<&mut [u8]>;
```

- Each function returns `None` if the buffer does not have enough space.
- Otherwise, it writes the value and returns a mutable reference to the remaining bytes.

---

## 3. Implementation Notes

- The functions use unsafe pointer casting to reinterpret byte slices as typed values.  They must respect alignment and aliasing rules: all reads/writes are aligned to the natural alignment of the type (which is guaranteed by using `#[repr(C)]` on intermediate on‑stack variables).  The remaining slice is created by advancing the pointer by `size_of::<T>()`.
- `read_*` and `write_*` are marked as pure (they only touch memory, no I/O).  This allows them to be used in auto‑parallelised loops.

---

## 4. Testing

For each type and endianness:

- **Reading:** Create a buffer with a known byte pattern, read a value, check it, and verify the remaining slice.
- **Writing:** Write a value into a buffer and verify the byte contents.
- **Edge cases:** Empty buffer, buffer exactly matching the size, buffer one byte short → `None`.

All tests must pass before moving to the next module.
