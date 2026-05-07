# Phase 4 – Ecosystem Crate: `blaze‑async‑std`

> **Goal:** Provide an ergonomic async runtime and utilities for Blaze applications that do not need the full actor model or that want a lighter, futures‑based concurrency framework.  It offers a multi‑threaded work‑stealing executor, timers, I/O helpers, and synchronisation primitives built on top of Blaze’s core async traits (`Future`, `Poll`, `Context`, `Wake`).  All I/O carries the `io` effect; pure operations are free of any effect.  The crate is designed to be compatible with `blaze‑smol` (a smaller single‑threaded executor) and the full actor runtime.

---

## 1. Core Concepts

The crate provides:

- **`Executor`** – a multi‑threaded, work‑stealing async executor, suitable for most applications.
- **`block_on`** – convenience function to run a future to completion, blocking the current thread.
- **`spawn`** – spawns a task onto the global executor.
- **`Timer`** – a future that resolves after a given duration (using `std::time`).
- **`Interval`** – a stream that yields every interval.
- **`io` module** – async versions of `Read` and `Write` traits (`AsyncRead`, `AsyncWrite`), compatible with standard I/O types.

All types are linear where they own resources (e.g., `Timer`).

---

## 2. Executor and Spawning

### 2.1 `Executor`

```
pub struct Executor {
    // work‑stealing thread pool
}
impl Executor {
    pub fn new() -> Result<Executor, IoError>;
    pub fn block_on<F: Future>(&self, future: F) -> F::Output;
    pub fn spawn<F: Future + Send + 'static>(&self, future: F) -> JoinHandle<F::Output>;
}
```

- `new` creates a thread pool with as many threads as logical cores.
- `block_on` runs the executor until the given future completes, blocking the calling thread.
- `spawn` submits a future to the executor and returns a `JoinHandle` that can be awaited for the result.

### 2.2 `JoinHandle<T>`

```
pub struct JoinHandle<T> { … }
impl<T> Future for JoinHandle<T> {
    type Output = T;
    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<T>;
}
```

- A handle to a spawned task; its `Future` implementation polls the task and returns the result when ready.

### 2.3 Global Executor

A global default executor is initialised lazily.  Functions `spawn` and `block_on` without an explicit executor use this global instance.

```
pub fn spawn<F: Future + Send + 'static>(future: F) -> JoinHandle<F::Output>;
pub fn block_on<F: Future>(future: F) -> F::Output;
```

---

## 3. Async I/O

### 3.1 `AsyncRead` and `AsyncWrite`

```
pub trait AsyncRead {
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, io::Error>;
    async fn read_to_end(&mut self, buf: &mut Vec<u8>) -> Result<usize, io::Error>;
}

pub trait AsyncWrite {
    async fn write(&mut self, buf: &[u8]) -> Result<usize, io::Error>;
    async fn flush(&mut self) -> Result<(), io::Error>;
}
```

- These traits mirror `std::io::Read` and `Write` but return futures.
- Blanket implementations are provided for types that implement the synchronous traits, using the executor’s blocking pool.

### 3.2 `AsyncFile`, `AsyncTcpStream`, `AsyncTcpListener`

Thin wrappers around the standard library’s `File`, `TcpStream`, and `TcpListener`, implementing `AsyncRead` and/or `AsyncWrite` by delegating I/O to the thread pool to avoid blocking the executor.

---

## 4. Timers and Intervals

### 4.1 `sleep`

```
pub fn sleep(duration: Duration) -> Sleep;
pub struct Sleep { … }
impl Future for Sleep { type Output = (); … }
```

- Resolves after the given duration.  Implemented via the global timer wheel.

### 4.2 `interval`

```
pub fn interval(period: Duration) -> Interval;
pub struct Interval { … }
impl Stream for Interval { type Item = (); … }
```

- Yields `()` every `period`.  Suitable for periodic tasks.

---

## 5. Synchronisation Primitives

The crate provides async‑aware versions of common synchronisation types, usable inside async contexts without blocking the entire thread.

- **`Mutex<T>`** – async mutex with a `lock` method that returns a future.
- **`RwLock<T>`** – async read‑write lock.
- **`Semaphore`** – counting semaphore.
- **`Channel<T>`** – a multi‑producer, single‑consumer channel with async `send` and `recv`.

All synchronisation types are linear and implement `Dispose` to wake waiting tasks.

---

## 6. Integration with Actor Runtime

The `blaze‑async‑std` executor is compatible with Blaze’s actor runtime.  Actors can be spawned on the same thread pool; the actor scheduler uses the global executor’s pool for work‑stealing.  Applications can mix `spawn` (for bare futures) and actor `spawn` (for actors) seamlessly.

---

## 7. Error Handling

```
pub enum IoError {
    Io(std::io::Error),
    Timeout,
    Canceled,
}
```

---

## 8. Testing

- **Executor:** Spawn several futures, collect results, verify they complete correctly.
- **Timer:** Use `sleep` and verify it wakes up after at least the specified duration.
- **Async I/O:** Write to an async file, read back, compare data.
- **Mutex:** Lock a mutex from multiple concurrent tasks, verify only one holds it at a time.
- **Channel:** Send and receive messages, verify ordering.

All tests must pass on all platforms.
