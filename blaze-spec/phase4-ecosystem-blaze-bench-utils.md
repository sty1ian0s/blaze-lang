# Phase 4 – Ecosystem Crate: `blaze‑bench‑utils`

> **Goal:** Provide a lightweight, zero‑cost benchmarking framework for Blaze.  It offers macros and functions to measure execution time, isolate benchmarks from the runtime, and produce deterministic, reproducible results.  The crate is designed to complement `@bench` attributes and integrate with `blaze test` and `blaze bench`.  All measurement is pure and free of I/O; reporting may carry the `io` effect.

---

## 1. Core Concepts

- **`bench!` macro** – wraps a block of code and measures its execution time over multiple iterations.
- **`Bencher`** – an object that manages warm‑up, measurement, and statistical aggregation.
- **`black_box`** – prevents the compiler from optimising away benchmarked values.
- **`BenchConfig`** – controls the number of iterations, warm‑up, and output format.

---

## 2. `bench!` Macro

```
bench!(name: &str, iterations: u64, || {
    // code to benchmark
});
```

- Example:
```blaze
bench!("vector sum", 1_000_000, || {
    let v = vec![1, 2, 3];
    let s = v.iter().sum();
    black_box(s);
});
```

- The macro reports the time taken (in nanoseconds) per iteration via the global reporter.
- Inside the closure, `black_box` must be used on values that are computed to prevent dead‑code elimination.

---

## 3. `Bencher` (Manual API)

```
pub struct Bencher {
    config: BenchConfig,
}

impl Bencher {
    pub fn new(config: BenchConfig) -> Bencher;
    pub fn iter<F>(&mut self, f: F) -> Duration where F: FnMut();
    pub fn iter_with_setup<F, S>(&mut self, setup: S, f: F) -> Duration where S: FnMut(), F: FnMut();
}
```

- `iter` runs the closure many times and returns the total duration.
- `iter_with_setup` runs the setup once, then the benchmark closure repeatedly.

### 3.1 `BenchConfig`

```
pub struct BenchConfig {
    pub iterations: u64,
    pub warm_up: u64,
    pub min_time: Option<Duration>,
    pub max_time: Option<Duration>,
    pub report_format: ReportFormat,
}
```

- `min_time` / `max_time` override the iteration count to run for at least/most the given wall‑time.

---

## 4. `black_box`

```
pub fn black_box<T>(val: T) -> T;
```

- The compiler is not allowed to optimise the value away, ensuring the benchmarked code is actually measured.
- `black_box` is an identity function at the source level, but the compiler treats it as an opaque operation.

---

## 5. Reporters

```
pub trait BenchReporter: Send + Sync + 'static {
    fn report(&self, name: &str, duration: Duration, iterations: u64);
}
```

- Built‑in reporters:
  - `ConsoleReporter` – prints to `stdout`.
  - `JsonReporter` – writes NDJSON to a file.
  - `CsvReporter` – writes CSV.
  - `NullReporter` – discards results.

The global reporter is set via `set_global_reporter`.

---

## 6. Integration with `@bench`

The `@bench` attribute on functions generates code that uses this crate’s `Bencher` to run the function and report results.  The `blaze bench` command discovers `@bench` functions and executes them using the configured reporter.

---

## 7. Determinism

Under `--reproducible`, the random seed used for warm‑up jitter is fixed, ensuring identical benchmark output across runs.

---

## 8. Testing

- **Basic bench:** Write a simple benchmark, verify it runs without error and produces a duration >0.
- **black_box:** Benchmark something that would be optimised away; verify that it is not.
- **Configuration:** Set a limited number of iterations and check that the bench stops after that count.

All tests must pass on all platforms.
