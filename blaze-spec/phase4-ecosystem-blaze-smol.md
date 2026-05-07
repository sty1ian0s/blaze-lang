# Phase 4 – Ecosystem Crate: `blaze‑smol`

> **Goal:** Provide a lightweight, single‑threaded async executor and minimal runtime for Blaze, suitable for embedded environments, WebAssembly, and scenarios where a full multi‑threaded runtime is unnecessary.  It implements the core `Future` and `Stream` traits, a local‑only executor, timers, and basic I/O primitives without any global state or thread pool.  All operations carry the `io` effect only when actual I/O is performed; pure futures are effect‑free.  The crate is `#![no_std]`‑friendly when the `alloc` feature is enabled, and can be used as a drop‑in replacement for `blaze‑async‑std` in single‑threaded contexts.

---

## 1. Core Concepts

The crate provides:

- **`LocalExecutor`** – a single‑threaded, non‑work‑stealing executor that runs futures on the current thread.
- **`block_on`** – runs a future to completion, blocking the calling thread.
- **`spawn_local`** – spawns a `!Send` future onto the local executor.
- **`Timer`** – an async timer future that resolves after a given duration, using a platform‑specific monotonic clock.
- **`I/O`** – async versions of `Read` and `Write` for the standard I/O types, implemented via polling and `WouldBlock` detection.
- **`io_uring`** support (optional, feature `io_uring`) for Linux systems that support it, enabling zero‑copy, truly asynchronous file I/O without blocking threads.

All executors are linear; the timer and I/O primitives are plain structs.

---

## 2. `LocalExecutor`

### 2.1 Struct

```
pub struct LocalExecutor {
    // task queue, timer wheel
}

impl LocalExecutor {
    pub fn new() -> LocalExecutor;
    pub fn spawn_local<F: Future + 'static>(&self, future: F) -> JoinHandle<F::Output>;
    pub fn block_on<F: Future>(&self, future: F) -> F::Output;
    pub fn run_until_stalled(&self);
}
```

- `spawn_local` adds a future to the executor’s queue.  The future does not need to be `Send` because it runs on the same thread.
- `block_on` runs the executor until the given future completes, blocking the current thread.
- `run_until_stalled` processes all ready tasks once without blocking.

### 2.2 `JoinHandle<T>`

```
pub struct JoinHandle<T> { … }
impl<T> Future for JoinHandle<T> {
    type Output = T;
    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<T>;
}
```

- A handle to a spawned local task, yielding its result when ready.

---

## 3. Timers

### 3.1 `sleep`

```
pub fn sleep(duration: Duration) -> Sleep;
pub struct Sleep { … }
impl Future for Sleep { type Output = (); … }
```

- Resolves after the given duration using the platform’s monotonic clock (`Instant`).

### 3.2 `interval`

```
pub fn interval(period: Duration) -> Interval;
pub struct Interval { … }
impl Stream for Interval { type Item = (); … }
```

- Yields `()` every `period`.

Timers are implemented by registering wake‑ups in the executor’s timer wheel.  No separate thread is needed.

---

## 4. I/O

### 4.1 `Async` wrapper

```
pub struct Async<T> {
    inner: T,
}
```

- Wraps any type implementing `Read` and/or `Write` and provides `AsyncRead`/`AsyncWrite` implementations using non‑blocking mode (`WouldBlock`).
- Works with `TcpStream`, `TcpListener`, `File`, etc.

### 4.2 `AsyncRead` and `AsyncWrite`

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

- The `Async` wrapper implements these traits.

### 4.3 `io_uring` (Optional)

When the feature `io_uring` is enabled, the executor uses an `io_uring` instance for truly asynchronous file I/O, bypassing the thread pool entirely.  This provides zero‑copy, low‑latency disk access on Linux.  The API remains the same; only the implementation changes.

---

## 5. WebAssembly Support

On `wasm32` targets, `smol` uses `wasm‑bindgen` bindings to schedule timers and I/O.  The `LocalExecutor::block_on` function is replaced by an event loop that integrates with the browser’s microtask queue.  The `Timer` uses `setTimeout` or `requestAnimationFrame` depending on precision requirements.

---

## 6. Error Handling

```
pub enum SmolError {
    Io(std::io::Error),
    Timeout,
    Canceled,
    NotSupported,
}
```

---

## 7. Testing

- **Local executor:** Spawn a local future, verify it runs to completion.
- **Timer:** Use `sleep` and verify it wakes after the specified duration.
- **I/O:** Create a TCP listener, accept a connection, send data, verify receipt.
- **WASM:** Run a simple async task in a browser environment, verify completion.

All tests must pass on all supported platforms, including WASM (via `wasm‑pack test`).
