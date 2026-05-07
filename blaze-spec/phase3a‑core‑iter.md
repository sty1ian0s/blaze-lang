# Blaze Phase 3a – Core Library: Iterators (`std::iter`)

> **Goal:** Implement the `std::iter` module exactly as specified. This module provides the `Iterator`, `IntoIterator`, `FromIterator`, `Sum`, `Product` traits, and the common iterator adaptor structs.

---

## 1. Core Traits

### 1.1 `Iterator`

```
pub trait Iterator {
    type Item;
    fn next(&mut self) -> Option<Self::Item>;
    fn size_hint(&self) -> (usize, Option<usize>) { (0, None) }
    fn map<B, F>(self, f: F) -> Map<Self, F> where F: Fn(Self::Item) -> B;
    fn filter<P>(self, predicate: P) -> Filter<Self, P> where P: Fn(&Self::Item) -> bool;
    fn take(self, n: usize) -> Take<Self>;
    fn skip(self, n: usize) -> Skip<Self>;
    fn chain<U>(self, other: U) -> Chain<Self, U::IntoIter> where U: IntoIterator<Item = Self::Item>;
    fn zip<U>(self, other: U) -> Zip<Self, U::IntoIter> where U: IntoIterator;
    fn enumerate(self) -> Enumerate<Self>;
    fn fold<B, F>(self, init: B, f: F) -> B where F: Fn(B, Self::Item) -> B;
    fn sum<S>(self) -> S where S: Sum<Self::Item>;
    fn product<P>(self) -> P where P: Product<Self::Item>;
    fn collect<B>(self) -> B where B: FromIterator<Self::Item>;
    fn any<F>(&mut self, f: F) -> bool where F: Fn(Self::Item) -> bool;
    fn all<F>(&mut self, f: F) -> bool where F: Fn(Self::Item) -> bool;
    fn find<F>(&mut self, f: F) -> Option<Self::Item> where F: Fn(&Self::Item) -> bool;
}
```

All methods except `next` have default implementations provided by the compiler‑generated standard library code.  Every iterator must implement `next`.

### 1.2 `IntoIterator`

```
pub trait IntoIterator {
    type Item;
    type IntoIter: Iterator<Item = Self::Item>;
    fn into_iter(self) -> Self::IntoIter;
}
```

Types that can be converted into an iterator implement `IntoIterator`.  There are blanket implementations for all `Iterator` types (returning themselves) and for reference/slice types.

### 1.3 `FromIterator`

```
pub trait FromIterator<A> {
    fn from_iter<T: IntoIterator<Item = A>>(iter: T) -> Self;
}
```

Allows constructing a collection from an iterator (e.g., `Vec::from_iter`, `Map::from_iter`).

### 1.4 `Sum` and `Product`

```
pub trait Sum<A = Self> {
    fn sum<I: Iterator<Item = A>>(iter: I) -> Self;
}
pub trait Product<A = Self> {
    fn product<I: Iterator<Item = A>>(iter: I) -> Self;
}
```

Implemented by numeric types to support the `sum()` and `product()` iterator methods.

---

## 2. Adaptor Structs

Each adaptor is a struct that wraps an underlying iterator and implements `Iterator`.  They are returned by the corresponding methods on `Iterator`.

### 2.1 `Map<I, F>`

```
pub struct Map<I, F> { iter: I, f: F }
impl<B, I: Iterator, F: Fn(I::Item) -> B> Iterator for Map<I, F> {
    type Item = B;
    fn next(&mut self) -> Option<B> { self.iter.next().map(&self.f) }
}
```

### 2.2 `Filter<I, P>`

```
pub struct Filter<I, P> { iter: I, pred: P }
impl<I: Iterator, P: Fn(&I::Item) -> bool> Iterator for Filter<I, P> {
    type Item = I::Item;
    fn next(&mut self) -> Option<I::Item> {
        while let Some(x) = self.iter.next() {
            if (self.pred)(&x) { return Some(x); }
        }
        None
    }
}
```

### 2.3 `Take<I>`

```
pub struct Take<I> { iter: I, n: usize }
impl<I: Iterator> Iterator for Take<I> {
    type Item = I::Item;
    fn next(&mut self) -> Option<I::Item> {
        if self.n == 0 { None }
        else { self.n -= 1; self.iter.next() }
    }
}
```

### 2.4 `Skip<I>`

```
pub struct Skip<I> { iter: I, n: usize }
impl<I: Iterator> Iterator for Skip<I> {
    type Item = I::Item;
    fn next(&mut self) -> Option<I::Item> {
        while self.n > 0 { self.iter.next(); self.n -= 1; }
        self.iter.next()
    }
}
```

### 2.5 `Chain<A, B>`

```
pub struct Chain<A, B> { a: A, b: B }
impl<A: Iterator, B: Iterator<Item = A::Item>> Iterator for Chain<A, B> {
    type Item = A::Item;
    fn next(&mut self) -> Option<A::Item> {
        self.a.next().or_else(|| self.b.next())
    }
}
```

### 2.6 `Zip<A, B>`

```
pub struct Zip<A, B> { a: A, b: B }
impl<A: Iterator, B: Iterator> Iterator for Zip<A, B> {
    type Item = (A::Item, B::Item);
    fn next(&mut self) -> Option<(A::Item, B::Item)> {
        let x = self.a.next()?;
        let y = self.b.next()?;
        Some((x, y))
    }
}
```

### 2.7 `Enumerate<I>`

```
pub struct Enumerate<I> { iter: I, count: usize }
impl<I: Iterator> Iterator for Enumerate<I> {
    type Item = (usize, I::Item);
    fn next(&mut self) -> Option<(usize, I::Item)> {
        let x = self.iter.next()?;
        let i = self.count;
        self.count += 1;
        Some((i, x))
    }
}
```

---

## 3. Blanket Implementations

- Every `Iterator` automatically implements `IntoIterator` (returning itself).
- References to slices (`&[T]`, `&mut [T]`) implement `IntoIterator` yielding references to elements.
- Ranges (`Range<Idx>`) implement `IntoIterator` (provided from `std::builtins` and linked to `Step`).
- Tuples and other types may get implementations later; for now only the above are required.

---

## 4. Testing

Write tests for:

- Each adaptor: create a vector, iterate via `map`, `filter`, `take`, `skip`, `chain`, `zip`, `enumerate`, and verify the elements produced.
- `fold`, `sum`, `product`: use with integer iterators to compute aggregations.
- `collect`: collect an iterator into a `Vec`, a `Map`, etc.
- `any` / `all` / `find`: verify correct boolean behavior.
- Edge cases: empty iterators, `size_hint`, zipping uneven lengths.

All tests must pass before moving to the next module.
