# Blaze Phase 3a – Core Library (Builtins, Memory, Strings)

> **Goal:** Implement the modules `std::builtins`, `std::mem`, and `std::string` exactly as specified.  Every function, struct, and trait must match the signatures and behaviour described below.  Write tests for all items before implementation.

---

## 1. `std::builtins`

### 1.1 `Option<T>`

```
pub enum Option<T> {
    Some(T),
    None,
}
```

Methods (implemented on `Option<T>`):

- `pub fn is_some(&self) -> bool` – `true` if `Some`.
- `pub fn is_none(&self) -> bool` – `true` if `None`.
- `pub fn unwrap(self) -> T` – returns the inner value if `Some`; panics with `"called Option::unwrap() on a None value"` if `None`.  All live linear variables are disposed before the panic.
- `pub fn expect(self, msg: &str) -> T` – like `unwrap` but panic message includes `msg`.
- `pub fn map<U>(self, f: fn(T) -> U) -> Option<U>` – `Some(x)` → `Some(f(x))`, `None` → `None`.
- `pub fn and_then<U>(self, f: fn(T) -> Option<U>) -> Option<U>` – `Some(x)` → `f(x)`, `None` → `None`.
- `pub fn or(self, optb: Option<T>) -> Option<T>` – `Some(x)` → `Some(x)`, `None` → `optb`.
- `pub fn or_else(self, f: fn() -> Option<T>) -> Option<T>` – `Some(x)` → `Some(x)`, `None` → `f()`.
- `pub fn ok_or<E>(self, err: E) -> Result<T, E>` – `Some(x)` → `Ok(x)`, `None` → `Err(err)`.
- `pub fn ok_or_else<E>(self, f: fn() -> E) -> Result<T, E>` – `Some(x)` → `Ok(x)`, `None` → `Err(f())`.
- `pub fn iter(&self) -> OptionIter<T>` – returns an iterator that yields at most one `T`.

`OptionIter<T>`:
```
pub struct OptionIter<T> { ptr: *const Option<T> }
impl<T> Iterator for OptionIter<T> {
    type Item = T;
    fn next(&mut self) -> Option<T>;
}
```
`next` returns the inner value if `Some`, otherwise `None`, and advances the internal pointer to null.

### 1.2 `Result<T, E>`

```
pub enum Result<T, E> {
    Ok(T),
    Err(E),
}
```

Methods:

- `pub fn is_ok(&self) -> bool`
- `pub fn is_err(&self) -> bool`
- `pub fn unwrap(self) -> T` – panics with `"called Result::unwrap() on an Err value"` if `Err`, showing debug of error if possible.
- `pub fn expect(self, msg: &str) -> T`
- `pub fn unwrap_err(self) -> E` – panics if `Ok`.
- `pub fn map<U>(self, f: fn(T) -> U) -> Result<U, E>` – `Ok(x)` → `Ok(f(x))`.
- `pub fn map_err<F>(self, f: fn(E) -> F) -> Result<T, F>` – `Err(e)` → `Err(f(e))`.
- `pub fn and_then<U>(self, f: fn(T) -> Result<U, E>) -> Result<U, E>` – `Ok(x)` → `f(x)`.
- `pub fn or_else<F>(self, f: fn(E) -> Result<T, F>) -> Result<T, F>` – `Err(e)` → `f(e)`.
- `pub fn ok(self) -> Option<T>` – `Ok(x)` → `Some(x)`, `Err` → `None`.
- `pub fn err(self) -> Option<E>` – `Err(e)` → `Some(e)`, `Ok` → `None`.
- `pub fn iter(&self) -> ResultIter<T, E>` – iterator over `Ok` values.

`ResultIter<T, E>`: analog of `OptionIter`.

### 1.3 `Range<Idx>`

```
pub struct Range<Idx> {
    pub start: Idx,
    pub end: Idx,
}
impl<Idx: PartialOrd + Step> IntoIterator for Range<Idx> {
    type Item = Idx;
    type IntoIter = RangeIter<Idx>;
    fn into_iter(self) -> RangeIter<Idx>;
}
pub struct RangeIter<Idx> { current: Idx, end: Idx }
impl<Idx: PartialOrd + Step> Iterator for RangeIter<Idx> {
    type Item = Idx;
    fn next(&mut self) -> Option<Idx>;
}
```

The `Step` trait (in `std::iter`) provides:
```
pub trait Step {
    fn succ(&self) -> Self;
    fn pred(&self) -> Self;
    fn steps(&self, to: &Self) -> Option<usize>;
}
```
Integers implement `Step` with arithmetic.

### 1.4 Macros

- `panic!(format_args...)` – compiler intrinsic, prints to stderr and aborts.
- `todo!()` – `panic!("not yet implemented")`.
- `unreachable!()` – `panic!("unreachable code reached")`.
- `format!(...)` – returns a `Text` (from `std::string`) containing formatted output.

---

## 2. `std::mem`

### 2.1 `Allocator` Trait

```
pub trait Allocator {
    fn allocate<T>(&self) -> Owned<T>;
    fn deallocate<T>(&self, ptr: Owned<T>);
}
```

### 2.2 `Arena`

```
pub struct Arena {
    bump_ptr: *mut u8,
    end_ptr: *mut u8,
}
impl Arena {
    pub fn new() -> Arena;           // default capacity 4096
    pub fn with_capacity(cap: usize) -> Arena;
    pub fn allocate<T>(&self) -> Owned<T>;
    pub fn deallocate<T>(&self, ptr: Owned<T>);   // no‑op
}
impl Allocator for Arena { }
```

Memory grows by allocating new blocks if bump pointer exceeds end; freeing is a no‑op, all memory released when arena is dropped.

### 2.3 `Pool<T>`

```
pub struct Pool<T> {
    slab: *mut T,
    free_list: *mut usize,
    capacity: usize,
}
impl<T> Pool<T> {
    pub fn new(capacity: usize) -> Pool<T>;
    pub fn allocate(&self) -> Owned<T>;
    pub fn deallocate(&self, ptr: Owned<T>);
}
impl<T> Allocator for Pool<T> { }
```

Slab allocation with a free list; capacity is fixed.

### 2.4 `Owned<T>`

```
pub struct Owned<T> {
    ptr: *mut T,
    alloc: *const dyn Allocator,
}
impl<T> Owned<T> {
    pub unsafe fn as_ptr(&self) -> *const T;
    pub unsafe fn as_mut_ptr(&mut self) -> *mut T;
}
impl<T> Deref for Owned<T> { type Target = T; ... }
impl<T> DerefMut for Owned<T> { ... }
impl<T> Dispose for Owned<T> { ... }
```
Dispose calls `alloc.deallocate(self)`.

### 2.5 `NonNull<T>`

```
pub struct NonNull<T> { ptr: *const T }
impl<T> NonNull<T> {
    pub unsafe fn new(ptr: *const T) -> Option<NonNull<T>>;
    pub unsafe fn new_unchecked(ptr: *const T) -> NonNull<T>;
    pub fn as_ptr(self) -> *const T;
}
```

---

## 3. `std::string`

### 3.1 `Text` (alias `String`)

```
pub struct Text { data: [u8; 24] }
```

Internal layout: if `data[23] == 0` → inline string, max 23 bytes, terminated by a zero in the unused space.  If `data[23] == 1` → heap allocated: bytes 0‑7 pointer, 8‑15 length, 16‑23 capacity.

Constructors and methods:

- `pub fn new() -> Text` – empty inline string.
- `pub fn from_str(s: &str) -> Text` – copies inline if `s.len() <= 23`, else heap allocates.
- `pub fn as_str(&self) -> &str` – returns the slice.
- `pub fn len(&self) -> usize`
- `pub fn is_empty(&self) -> bool`
- `pub fn push_str(&mut self, s: &str)` – may convert to heap or reallocate if needed.
- `pub fn push(&mut self, ch: char)` – encodes as UTF‑8 and pushes.
- `pub fn to_lowercase(&self) -> Text`, `to_uppercase(&self) -> Text`
- `pub fn trim(&self) -> &str` – whitespace trim.
- `pub fn split_whitespace(&self) -> SplitWhitespace`
- `pub fn as_bytes(&self) -> &[u8]`
- `pub fn into_bytes(self) -> Vec<u8>`
- `impl Clone for Text` – deep copy.
- `impl Dispose for Text` – frees heap if owned.

Iterator implementations:

- `impl IntoIterator for Text` (and for `&Text`) yields `Chars`.
- `Chars` implements `Iterator<Item = char>`.

### 3.2 Other string iterators

- `Lines<'a>` – splits by `\n`, yields `&'a str`.
- `SplitWhitespace<'a>` – splits by Unicode whitespace, yields `&'a str`.

---

## 4. Testing

For each module:

- `std::builtins`: unit tests for every method on `Option` and `Result`, including panic messages.  Test `Range` iterator, macro output.
- `std::mem`: allocation, alignment, disposal, out‑of‑memory behaviour (when implemented), `NonNull` creation.
- `std::string`: inline vs heap strings, push across boundary, iteration over chars/lines/whitespace, cloning, disposal.

All tests must pass before proceeding to the next standard library modules.
