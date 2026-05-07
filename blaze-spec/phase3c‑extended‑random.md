# Blaze Phase 3c – Extended Library: Random (`std::random`)

> **Goal:** Implement the `std::random` module exactly as specified.  This module provides traits for random number generators, a thread‑local source of randomness, and functions for generating random values of primitive types.  All generators are deterministic given a seed and are suitable for both testing and production use.

---

## 1. `Rng` Trait

### 1.1 Trait Definition

```
pub trait Rng {
    fn next_u32(&mut self) -> u32;
    fn next_u64(&mut self) -> u64;
}
```

- `next_u32` returns a pseudorandom `u32` value.
- `next_u64` returns a pseudorandom `u64` value.

The trait provides default implementations for many convenience methods using the raw random outputs.

### 1.2 Provided Methods

```
trait Rng {
    fn gen_range(&mut self, low: Self, high: Self) -> Self;       // for integer types
    fn gen_range_f(&mut self, low: f64, high: f64) -> f64;        // for floats
    fn fill_bytes(&mut self, dest: &mut [u8]);
    fn gen<T: Random>(&mut self) -> T;
}
```

- `gen_range(low, high)`: returns a pseudorandom integer in the half‑open interval `[low, high)`.  Panics if `low >= high`.
- `gen_range_f(low, high)`: returns a pseudorandom float in `[low, high)`.
- `fill_bytes(dest)`: fills the slice with pseudorandom bytes.
- `gen::<T>()`: generates a random value of a type implementing the `Random` trait (primitive types, tuples of random types, etc.).

### 1.3 Trait `Random`

```
pub trait Random {
    fn random<R: Rng>(rng: &mut R) -> Self;
}
```

- Implemented for all primitive integer and floating‑point types, `bool`, `char`, and `()`.
- For integers: uniformly distributed over the full range.
- For floats: uniformly distributed in `[0, 1)`.
- For `bool`: `true` or `false` with equal probability.
- For `char`: a random Unicode scalar in the basic multilingual plane (`'\u{0000}'..='\u{D7FF}'` and `'\u{E000}'..='\u{FFFF}'`).
- For `()`: returns `()` (no randomness needed).

---

## 2. `SeedableRng` Trait

```
pub trait SeedableRng: Rng {
    fn from_seed(seed: u64) -> Self;
    fn seed(&self) -> u64;
}
```

- `from_seed(seed)`: creates a new generator from a 64‑bit seed.
- `seed()`: returns the seed that was used to initialise the generator.

---

## 3. Default Generator

The module exports a default generator `DefaultRng` that is cryptographically secure on platforms where hardware RNG is available, falling back to a fast, non‑cryptographic generator when unavailable.

### 3.1 `DefaultRng`

```
pub struct DefaultRng { /* private */ }

impl DefaultRng {
    pub fn new() -> DefaultRng;
}
```

- `new()` initialises from a combination of system entropy (e.g., `/dev/urandom`, `getrandom()`) and a hardware timestamp.

`DefaultRng` implements `Rng`, `SeedableRng` (though `from_seed` will produce a different sequence each time because it mixes in additional entropy).

---

## 4. `thread_rng` Function

```
pub fn thread_rng() -> impl Rng;
```

- Returns a handle to a thread‑local generator.  Multiple calls in the same thread return the same generator; this is safe because the generator’s mutability is tracked via ownership (the returned object is `&mut` or auto‑mut).  The thread‑local generator is initialised lazily with system entropy.

---

## 5. Additional PRNGs

The module also provides well‑known non‑cryptographic generators suitable for simulations and deterministic testing.

### 5.1 `Lcg128`

```
pub struct Lcg128 { state: u128 }
impl Lcg128 {
    pub fn new(seed: u128) -> Lcg128;
}
impl Rng for Lcg128 { ... }
impl SeedableRng for Lcg128 { ... }
```

- Linear congruential generator with modulus `2^128`, multiplier `6364136223846793005`, increment `1442695040888963407`.  (Well‑known constants from the PCG family.)

### 5.2 `Xoshiro256StarStar`

```
pub struct Xoshiro256StarStar { state: [u64; 4] }
impl Xoshiro256StarStar {
    pub fn new(seed: u64) -> Xoshiro256StarStar;
}
impl Rng for Xoshiro256StarStar { ... }
impl SeedableRng for Xoshiro256StarStar { ... }
```

- Fast, high‑quality PRNG using the xoshiro256** algorithm by David Blackman and Sebastiano Vigna.

---

## 6. Implementation Notes

- The `Rng` trait methods that build on `next_u32`/`next_u64` are provided as default implementations.
- `fill_bytes` repeatedly calls `next_u32` or `next_u64` and writes the bytes into the destination slice.
- `gen_range` uses rejection sampling when the range is not a power of two to avoid bias.
- Platform entropy is gathered via the operating system’s random facility (e.g., `/dev/urandom` on Unix, `BCryptGenRandom` on Windows) and is only used to seed the default generator and the thread‑local generator.

---

## 7. Testing

- **Determinism:** Create a `SeedableRng` from a known seed, generate a sequence of values, and verify it matches a precomputed sequence.
- **Thread‑local generator:** Call `thread_rng()` from multiple threads (if concurrency is available) and verify each thread generates random numbers independently without data races.
- **`gen_range`:** Generate many values, check that they lie within the specified bounds, and (optionally) perform a simple statistical test (e.g., chi‑square) to verify uniformity.
- **`fill_bytes`:** Fill a buffer, check that all bytes are not identical to the initial contents, and verify the buffer size is correct.
- **Trait `Random`:** Test that `gen::<i32>()`, `gen::<f64>()`, `gen::<bool>()` produce values within expected domains.

All tests must pass before moving to the next module.
