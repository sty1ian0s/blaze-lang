# Blaze Phase 3b – Systems Library: Futures (`std::future`)

> **Goal:** Implement the `std::future` module and the supporting `std::pin` module exactly as specified.  This module provides the core types for asynchronous programming: the `Future` trait, `Poll`, `Context`, `Wake`, and the `Pin` wrapper required by `Future::poll`.

Because `Future::poll` takes a `Pin<&mut Self>` argument, we first define the minimal `Pin` type in `std::pin`, then the future types in `std::future`.

---

## 1. `std::pin` – Pinning

### 1.1 `Pin<P>`

```
pub struct Pin<P> {
    pointer: P,
}
```

- `Pin` is a pointer wrapper that asserts the pointee will not be moved in memory.  It is used to guarantee that a value remains at a fixed address, as required by the `Future` state machine (which may contain self‑referential fields).

### 1.2 Constructor

```
impl<P> Pin<P> {
    pub unsafe fn new_unchecked(pointer: P) -> Pin<P>;
}
```

- `new_unchecked` creates a `Pin` from a pointer/reference.  The caller must guarantee that the pointee is stable and will not be moved until it is dropped.

- No safe constructor is provided; the runtime or async code generator uses `unsafe` to create `Pin` wrappers around future state machines.  The standard library does not offer a safe way to pin a value because that requires compiler‑generated support or unsafe code.

### 1.3 Deref and DerefMut

```
impl<P: Deref> Deref for Pin<P> {
    type Target = P::Target;
    fn deref(&self) -> &Self::Target;
}

impl<P: DerefMut> DerefMut for Pin<P> {
    fn deref_mut(&mut self) -> &mut Self::Target;
}
```

- `Deref` and `DerefMut` forward to the inner pointer’s target, allowing field access **without moving**.  The `DerefMut` implementation must not allow the user to swap or replace the pinned value; it only returns a mutable reference.  The `Pin` documentation warns that using `mem::replace` or similar on the pointee is unsound.  The standard library ensures that `Pin` is only used with types that are `Unpin` (for safe access) or with `unsafe` when implemented.  For now, we trust the `unsafe` boundary.

- `Unpin` trait: a marker trait that indicates a type can safely be moved out of a `Pin`.  All primitive types and most standard types implement `Unpin` automatically.  We'll define it as:

```
pub auto trait Unpin {}
```

- The auto trait `Unpin` means that any type that doesn't opt out (with `!Unpin`) is `Unpin`.  This will be automatically derived for all types by the compiler (Phase 2b already supports auto traits).  Types generated as future state machines are `!Unpin` to prevent accidental moves.

---

## 2. `std::future` Module

### 2.1 `Future` Trait

```
pub trait Future {
    type Output;
    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output>;
}
```

- A `Future` represents a value that may not be ready yet.  The `poll` method advances the state machine as far as possible; if complete, it returns `Poll::Ready(output)`, otherwise `Poll::Pending` and arranges to be woken when progress can be made.

- `poll` receives `Pin<&mut Self>` to guarantee the future is not moved between polls.  Implementors must not move out of `self`; the method is `unsafe` in spirit but safe code cannot violate the pinning invariant because `Pin` is only constructable via `unsafe`.

### 2.2 `Poll<T>`

```
pub enum Poll<T> {
    Ready(T),
    Pending,
}
```

- Either contains the final output (`Ready`) or signals that the future is still waiting.

### 2.3 `Context`

```
pub struct Context<'a> {
    waker: &'a dyn Wake,
}
```

- Holds a reference to a `Wake` implementation that can be used to notify the executor that the future is ready to be polled again.

### 2.4 `Wake` Trait

```
pub trait Wake {
    fn wake(self: &Self);
}
```

- The `wake` method is called by asynchronous operations when the future should be re‑polled.  Typically, an executor provides a `Wake` that schedules the task back onto the work queue.

---

## 3. Unsafe Future Marker

```
pub unsafe trait UnsafeFuture: Future {}
```

- Marker trait for low‑level futures that require the programmer to safely handle pinning and other invariants.

---

## 4. Implementation Guidance

- The compiler’s async‑to‑state‑machine transformation (Phase 2d) creates a struct implementing `Future`.  That struct is `!Unpin` and contains local variables moved from stack to fields.  `poll` advances the state machine and returns `Pending` until the final state.
- `Context` and `Wake` are used by the runtime (actor scheduler) to manage task waking.  The default runtime provides a `Wake` implementation that pushes the task back onto the appropriate worker queue.
- `Pin` is a lightweight wrapper; in the generated code, it simply holds a raw pointer or reference.  Its primary role is documentation and preventing accidental moves at the type level.

---

## 5. Testing

For `std::pin`:
- Create a `Pin<&mut i32>` and verify that deref works.
- Ensure that attempting to move out of a `Pin` (by using `replace`) is not allowed by the type system (e.g., the compiler should reject `std::mem::replace` on a `Pin<&mut T>`).

For `std::future`:
- Implement a simple `Future` manually (e.g., a future that immediately returns a value) and test `poll` returns `Ready`.
- Use the async‑to‑state‑machine compiler feature (already tested in Phase 2d) to create an async function and verify that its `.poll()` works through the `Future` trait.

All tests must pass before proceeding to the next module.
