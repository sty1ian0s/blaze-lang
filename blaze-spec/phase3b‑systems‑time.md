# Blaze Phase 3b – Systems Library: Time (`std::time`)

> **Goal:** Implement the `std::time` module exactly as specified.  This module provides the `Instant` and `Duration` types for measuring and working with time intervals, along with a function to access the current time.

---

## 1. `Duration`

### 1.1 Struct

```
pub struct Duration {
    secs: u64,
    nanos: u32,
}
```

- Represents a span of time with nanosecond precision.  The internal representation stores seconds as a `u64` and fractional nanoseconds as a `u32`.  The nanoseconds field is always less than `1_000_000_000`.

### 1.2 Constructors

```
impl Duration {
    pub fn new(secs: u64, nanos: u32) -> Duration;
    pub fn from_secs(secs: u64) -> Duration;
    pub fn from_millis(millis: u64) -> Duration;
    pub fn from_micros(micros: u64) -> Duration;
    pub fn from_nanos(nanos: u64) -> Duration;
}
```

- `new(secs, nanos)`: creates a `Duration` from a whole seconds count and a nanosecond fraction.  Panics if `nanos >= 1_000_000_000`.
- `from_secs(secs)`: `Duration { secs, nanos: 0 }`.
- `from_millis(millis)`: converts milliseconds, e.g., `millis = 1250` gives `1` second and `250_000_000` nanoseconds.
- `from_micros`, `from_nanos` likewise convert and normalise into seconds and remaining nanoseconds.

### 1.3 Accessors

```
impl Duration {
    pub fn as_secs(&self) -> u64;
    pub fn subsec_millis(&self) -> u32;
    pub fn subsec_micros(&self) -> u32;
    pub fn subsec_nanos(&self) -> u32;
}
```

- `as_secs`: returns the whole seconds component.
- `subsec_millis`: returns the fractional part in milliseconds (`self.nanos / 1_000_000`).
- `subsec_micros`: fractional part in microseconds (`self.nanos / 1_000`).
- `subsec_nanos`: fractional part in nanoseconds (`self.nanos`).

### 1.4 Arithmetic

`Duration` implements `Add<Duration>`, `Sub<Duration>`, `Mul<u32>`, `Div<u32>`, and corresponding assignment operators.

```
impl Add<Duration> for Duration {
    type Output = Duration;
    fn add(self, rhs: Duration) -> Duration {
        let mut secs = self.secs + rhs.secs;
        let mut nanos = self.nanos + rhs.nanos;
        if nanos >= 1_000_000_000 {
            secs += 1;
            nanos -= 1_000_000_000;
        }
        Duration { secs, nanos }
    }
}
```

Sub, Mul, Div similarly; division by zero panics.

### 1.5 Comparisons

`Duration` implements `PartialEq`, `Eq`, `PartialOrd`, `Ord`.

### 1.6 Trait Implementations

`Duration` is `@copy` (size is 12 bytes, contains no pointers, all fields are `@copy`).  It also implements `Default` (returns zero duration).

---

## 2. `Instant`

### 2.1 Struct

```
pub struct Instant {
    t: u64,   // monotonic timestamp in nanoseconds from an arbitrary epoch
}
```

- `Instant` represents a point in time from a monotonic clock.  It cannot be directly converted to a wall‑clock time but is guaranteed to be non‑decreasing.

### 2.2 Constructors

```
impl Instant {
    pub fn now() -> Instant;
}
```

- `now()` returns the current instant using the highest‑resolution monotonic clock available on the platform (e.g., `clock_gettime(CLOCK_MONOTONIC)` on Unix, `QueryPerformanceCounter` on Windows).

### 2.3 Operations

`Instant` supports subtraction:

```
impl Sub<Instant> for Instant {
    type Output = Duration;
    fn sub(self, other: Instant) -> Duration {
        Duration::from_nanos(self.t - other.t)
    }
}
```

- Adding `Duration` to `Instant` is also supported:

```
impl Add<Duration> for Instant {
    type Output = Instant;
    fn add(self, dur: Duration) -> Instant {
        Instant { t: self.t + dur.as_nanos() }
    }
}
```

- `Instant` implements `PartialEq`, `Eq`, `PartialOrd`, `Ord` (by comparing the raw monotonic count).

### 2.4 `elapsed`

```
impl Instant {
    pub fn elapsed(&self) -> Duration;
}
```

- Returns `Instant::now() - *self`.

### 2.5 Trait Implementations

`Instant` is `@copy` (size 8 bytes).

---

## 3. `SystemTime` (Optional for Phase 3b)

If desired, we can add a `SystemTime` type for wall‑clock time, but the spec only requires `Instant` and `Duration`.  We'll implement `SystemTime` in a later phase or as an extension.  For now, `std::time` contains only `Duration` and `Instant`.

---

## 4. Implementation Notes

- The C backend wraps platform‑specific time functions.  On Unix, we use `clock_gettime` with `CLOCK_MONOTONIC`; on Windows, `QueryPerformanceFrequency` and `QueryPerformanceCounter`.
- The raw value `t` in `Instant` is a `u64` storing nanoseconds.  The actual resolution may be coarser; we convert from the system’s raw ticks to nanoseconds using a precomputed scaling factor.

---

## 5. Testing

- **Duration:** Create various durations, test arithmetic (add, sub, mul, div), check that sub‑second components are correct (e.g., `from_millis(1250)` gives `1s 250ms`).  Test comparisons.
- **Instant:** Take an instant, sleep for a short time, then check that the elapsed duration is within expected bounds (at least 0, not too large).
- **Monotonicity:** Verify that consecutive calls to `now()` do not decrease.
- Test all edge cases: zero duration, large durations, arithmetic overflow (when adding, the result wraps? Actually we should define overflow behavior: for `Duration` arithmetic, we will panic on overflow in debug mode; in release mode, we will wrap for now, but this is subject to future specification.  For Phase 3b, we will implement as panicking in debug and wrap in release, matching integer behavior.)

All tests must pass before moving to the next module.
