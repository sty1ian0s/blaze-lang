# Blaze Phase 3a – Core Library: Operator Traits (`std::ops`)

> **Goal:** Implement the `std::ops` module exactly as specified.  This module provides traits for operator overloading.  Each trait maps to a language operator, enabling custom types to support standard arithmetic, bitwise, and assignment operators with zero runtime overhead.

---

## 1. Binary Operator Traits

All binary operator traits define an associated `Output` type and a single method.  The default `Rhs` generic parameter is `Self`, meaning the operator works on two values of the same type unless explicitly specified otherwise.

### 1.1 `Add`

```
pub trait Add<Rhs = Self> {
    type Output;
    fn add(self, rhs: Rhs) -> Self::Output;
}
```
- `+` operator desugars to `Add::add(left, right)`.
- Typically returns the same type (`Output = Self`), but can differ (e.g., adding a `Duration` to an `Instant`).

### 1.2 `Sub`

```
pub trait Sub<Rhs = Self> {
    type Output;
    fn sub(self, rhs: Rhs) -> Self::Output;
}
```

### 1.3 `Mul`

```
pub trait Mul<Rhs = Self> {
    type Output;
    fn mul(self, rhs: Rhs) -> Self::Output;
}
```

### 1.4 `Div`

```
pub trait Div<Rhs = Self> {
    type Output;
    fn div(self, rhs: Rhs) -> Self::Output;
}
```
- Division by zero for integers panics; for floats returns infinity.

### 1.5 `Rem`

```
pub trait Rem<Rhs = Self> {
    type Output;
    fn rem(self, rhs: Rhs) -> Self::Output;
}
```
- Remainder operator `%`.  For integers, follows C semantics; for floats, returns the floating‑point remainder.

### 1.6 `BitAnd`

```
pub trait BitAnd<Rhs = Self> {
    type Output;
    fn bitand(self, rhs: Rhs) -> Self::Output;
}
```
- `&` (bitwise AND).  Not to be confused with reference operator; the parser disambiguates by context.

### 1.7 `BitOr`

```
pub trait BitOr<Rhs = Self> {
    type Output;
    fn bitor(self, rhs: Rhs) -> Self::Output;
}
```

### 1.8 `BitXor`

```
pub trait BitXor<Rhs = Self> {
    type Output;
    fn bitxor(self, rhs: Rhs) -> Self::Output;
}
```

### 1.9 `Shl`

```
pub trait Shl<Rhs = Self> {
    type Output;
    fn shl(self, rhs: Rhs) -> Self::Output;
}
```
- `<<` shift left.

### 1.10 `Shr`

```
pub trait Shr<Rhs = Self> {
    type Output;
    fn shr(self, rhs: Rhs) -> Self::Output;
}
```
- `>>` shift right.  For signed types, arithmetic shift.

---

## 2. Unary Operator Traits

### 2.1 `Neg`

```
pub trait Neg {
    type Output;
    fn neg(self) -> Self::Output;
}
```
- Unary `-`.

### 2.2 `Not`

```
pub trait Not {
    type Output;
    fn not(self) -> Self::Output;
}
```
- Unary `!` (logical or bitwise complement).

---

## 3. Compound Assignment Traits

Each compound assignment trait requires a mutable borrow of `self` and the right‑hand side.  The compiler can synthesize `a += b` as `a = a + b` if only `Add` is implemented; however, providing an explicit implementation may be more efficient.

### 3.1 `AddAssign`

```
pub trait AddAssign<Rhs = Self> {
    fn add_assign(&mut self, rhs: Rhs);
}
```

### 3.2 `SubAssign`

```
pub trait SubAssign<Rhs = Self> {
    fn sub_assign(&mut self, rhs: Rhs);
}
```

### 3.3 `MulAssign`

```
pub trait MulAssign<Rhs = Self> {
    fn mul_assign(&mut self, rhs: Rhs);
}
```

### 3.4 `DivAssign`

```
pub trait DivAssign<Rhs = Self> {
    fn div_assign(&mut self, rhs: Rhs);
}
```

### 3.5 `RemAssign`

```
pub trait RemAssign<Rhs = Self> {
    fn rem_assign(&mut self, rhs: Rhs);
}
```

### 3.6 `BitAndAssign`

```
pub trait BitAndAssign<Rhs = Self> {
    fn bitand_assign(&mut self, rhs: Rhs);
}
```

### 3.7 `BitOrAssign`

```
pub trait BitOrAssign<Rhs = Self> {
    fn bitor_assign(&mut self, rhs: Rhs);
}
```

### 3.8 `BitXorAssign`

```
pub trait BitXorAssign<Rhs = Self> {
    fn bitxor_assign(&mut self, rhs: Rhs);
}
```

### 3.9 `ShlAssign`

```
pub trait ShlAssign<Rhs = Self> {
    fn shl_assign(&mut self, rhs: Rhs);
}
```

### 3.10 `ShrAssign`

```
pub trait ShrAssign<Rhs = Self> {
    fn shr_assign(&mut self, rhs: Rhs);
}
```

---

## 4. Standard Implementations

The compiler provides implementations of these traits for all primitive types where the operators are defined (e.g., `i32` implements `Add`, `Sub`, etc.).  For user‑defined types, the developer must implement the desired traits.

---

## 5. Testing

- For each primitive type, verify that the operators work as expected (e.g., `3 + 4`, `3 > 4` are already tested elsewhere, but we should ensure that the traits are actually implemented).
- Define a custom struct (e.g., `Vec2`) and implement `Add`; verify that `+` works.
- Test that `a += b` works if `AddAssign` is implemented, and that the compiler synthesises it from `Add` if only `Add` is present (this is a compiler feature already in place from earlier phases, but a test confirms it).
- Verify that custom `Output` types (e.g., `Add<Duration> for Instant`) compile and run correctly.

All tests must pass before moving to the next module.
