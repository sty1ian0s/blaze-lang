# Blaze Phase 3c – Extended Library: Fixed‑Point Arithmetic (`std::fixed`)

> **Goal:** Implement the `std::fixed` module exactly as specified.  This module provides fixed‑point numeric types with compile‑time‑defined integer and fractional widths.  All operations are deterministic, overflow behavior is configurable, and performance is equivalent to integer arithmetic with zero runtime overhead beyond the underlying integer operations.

---

## 1. `Fixed` Type

### 1.1 Type Definition

```
pub struct Fixed<const N: usize, const M: usize> {
    bits: iN,   // N is the total number of bits, M is the number of fractional bits
}
```

- `N` is the total bit width of the underlying signed integer; `M` is the number of fractional bits (0 ≤ M < N).
- The type internally stores the raw integer representation with `M` implied fractional bits.
- All arithmetic operations automatically handle scaling so that the programming model is in fixed‑point with the given radix point.

The underlying integer type `iN` is chosen as the smallest signed integer type capable of holding `N` bits, with `N` being one of 8, 16, 32, 64, 128.  The generic parameters `N` and `M` are compile‑time constants.

### 1.2 Constructors

```
impl<const N: usize, const M: usize> Fixed<N, M> {
    pub fn from_raw(raw: iN) -> Self;
    pub fn from_float(val: f64) -> Self;
    pub fn from_int(val: i32) -> Self;
    pub fn to_float(self) -> f64;
    pub fn to_int(self) -> i32;
    pub fn raw(self) -> iN;
}
```

- `from_raw(raw)`: constructs a `Fixed` directly from the raw integer representation.  No scaling is performed.
- `from_float(val)`: converts a floating‑point value to fixed‑point with round‑to‑nearest ties‑to‑even.  Saturates if the value cannot be represented (overflow to min/max).
- `from_int(val)`: converts an integer to fixed‑point by shifting left by `M` bits.  Overflow panics in debug mode, wraps in release mode (unless configured otherwise).
- `to_float(self)`: converts the fixed‑point value to `f64` by dividing by `2^M`.
- `to_int(self)`: converts to integer by truncating the fractional part (shift right by `M`).
- `raw(self)`: returns the raw integer representation.

---

## 2. Arithmetic Operations

`Fixed<N, M>` implements all common operator traits from `std::ops`:

- `Add`, `Sub`, `Mul`, `Div`, `Rem`, `Neg`, `AddAssign`, `SubAssign`, `MulAssign`, `DivAssign`, `RemAssign`.

### 2.1 Behavior

- Addition and subtraction are straightforward integer addition/subtraction on the raw `bits` (no extra scaling needed because both operands have the same radix point).
- Multiplication: `(a * b) >> M` (with appropriate intermediate widening to avoid overflow in the product).  The implementation uses a wider intermediate type (e.g., for `N=32`, multiply as `i64` then shift right `M` and clamp to `i32` range).  Overflow behavior: debug panics, release wraps.
- Division: `(a << M) / b` (using widening to prevent quotient overflow).  Division by zero panics.
- Remainder: `a % b` (applied to the raw integers).  Result has same radix point.

All arithmetic respects the fixed‑point scaling automatically.

---

## 3. Saturation and Wrapping Control

By default, `Fixed` follows the same overflow policy as the underlying integer type: panic in debug, wrap in release.  However, the module provides additional methods for explicit saturation:

```
impl<const N: usize, const M: usize> Fixed<N, M> {
    pub fn saturating_add(self, rhs: Self) -> Self;
    pub fn saturating_sub(self, rhs: Self) -> Self;
    pub fn saturating_mul(self, rhs: Self) -> Self;
    pub fn saturating_neg(self) -> Self;
}
```

These methods clamp the result to the representable range instead of wrapping.

---

## 4. Comparisons

`Fixed<N, M>` implements `PartialEq`, `Eq`, `PartialOrd`, `Ord`.  Comparisons compare the raw integer representations (which is equivalent to comparing the fixed‑point values).

---

## 5. Bitwise Operations (not typically used, but available)

Bitwise operations (`&`, `|`, `^`, `<<`, `>>`) operate on the raw bits.  Right shift does **not** preserve the radix point; the user must manually adjust if needed.  These are provided for advanced use cases.

---

## 6. Traits Implemented

- `@copy` (if the underlying integer is `@copy`, which it always is).
- `Clone` (trivial).
- `Default` (returns zero).
- `Debug`, `Display` (formats the value using the fixed‑point representation, e.g., `12.34`).
- `Hash` (hashes the raw bits).

---

## 7. Testing

- **Construction:** Convert from float and integer, verify that the raw bits produce the expected scaled integer.
- **Arithmetic:** Add, subtract, multiply, divide fixed‑point numbers and compare with expected floating‑point equivalents (allowing for epsilon due to rounding).
- **Overflow behavior:** Create a fixed‑point value near the representable limit and test panic (debug) vs wrap (release) for add/sub/mul.  Test saturation methods.
- **Comparisons:** Check equality and ordering among fixed‑point values.
- **Precision:** For a given `N` and `M`, verify that the range and resolution are correct (e.g., `Fixed<8, 4>` should have step size 1/16).
- All tests must pass before moving to the next module.
