# Blaze Phase 2d – Concurrency: Actors, Async, Channels, Supervision

> **Goal:** Implement the actor model, asynchronous functions (`async fn`), channels (`Sender`/`Receiver`), and the supervision tree. The parser already accepts `actor`, `async`, `await`, `spawn`, and the channel syntax. This phase adds semantic analysis, code generation for state machines, and the runtime scheduler.

---

## 1. Actor Model

### 1.1 Actor Declaration
```
actor_decl = "actor" ident [ generic_params ] "{" { actor_item } "}"
actor_item = field_decl | fn_decl
```
- An actor is a struct with isolated state and sequential message handling.
- Fields may be of any type.
- Methods annotated with `@message` become message handlers.

### 1.2 Message Handlers (`@message`)
- A method decorated with `@message` is not callable synchronously; it can only be invoked by sending a message to the actor.
- The compiler generates:
  - A **message struct** containing the method arguments (if any).
  - A **handle type** `Handle<M>` for the response channel.
  - A **capability method** on `Capability<A>` that sends the message.
- Example:
  ```blaze
  actor Counter {
      value: i32,
      @message fn inc(&mut self, n: i32) -> i32 { self.value += n; self.value }
  }
  ```
  generates:
  - Message struct `Counter__inc { n: i32 }`.
  - `fn inc(cap: &Capability<Counter>, n: i32) -> Request<i32>` on `Capability<Counter>`.

### 1.3 Spawning Actors
- `spawn(actor_expression, supervisor?)` creates a new actor and returns a `Capability<A>`.
- The actor starts executing immediately in its own logical thread.
- `Capability<A>` is a lightweight handle; cloning it is allowed (shallow, reference‑counted).

### 1.4 Actor Semantics
- Actors are cooperative: each actor runs until it returns `await` or completes a message handler.
- No pre‑emption; an infinite loop in a handler blocks the actor permanently.
- Actors are linear: when an actor terminates (all handlers return, or it panics), its resources are disposed.

---

## 2. Asynchronous Functions (`async fn`)

### 2.1 Syntax and Desugaring
- `async fn foo() -> T` returns a `Future<T>`.
- Inside an `async fn`, `await` is used to suspend until a future completes.
- The compiler transforms the body into a state machine that implements `Future`.

### 2.2 State Machine Generation
- Each `await` point becomes a state. The state machine stores local variables that live across suspensions.
- All linear variables captured across an `await` must implement `Dispose` so they can be cleaned up on cancellation.
- The generated `Future` trait implementation provides `poll` that advances the state machine.

### 2.3 Cancellation
- When a `Future` is dropped (cancelled), the compiler inserts a call to `Dispose::dispose` on all live linear variables.
- This ensures no resource leaks.

---

## 3. Channels

### 3.1 Channel Types
- `Sender<T>`: cloneable, reference‑counted sender end.
- `Receiver<T>`: linear receiver end; implements `Iterator<Item = T>`.
- `channel<T>() -> (Sender<T>, Receiver<T>)` creates a new channel pair.

### 3.2 Semantics
- Sends are non‑blocking (if the buffer is full, the sender yields via the runtime).
- Receives are non‑blocking via `recv()` (returns `Option<T>`), or can be used in a `for` loop (which polls).
- Channels are used for communication between actors.

---

## 4. Supervisor Tree

### 4.1 Supervision
- Each actor has a parent supervisor (the actor that spawned it, or the runtime for root actors).
- If an actor panics more than 3 times within a 10‑second window, the supervisor escalates the failure to its own parent.
- Root actor failure terminates the process with exit code 1.

### 4.2 Supervisor Handling
- The supervisor can optionally handle failure by restarting the child actor.
- The runtime provides a default supervisor that implements the escalation logic.

---

## 5. Runtime Scheduler

### 5.1 Thread Pool
- The runtime creates a work‑stealing thread pool with one worker thread per physical core.
- Each worker thread maintains a local queue of actor mailboxes.

### 5.2 Mailbox
- Each actor has a lock‑free SPSC mailbox for incoming messages, upgraded to MPSC when `Capability` is cloned (via a small internal lock).
- The actor polls its mailbox when scheduled; on message, it runs the handler to completion.

### 5.3 Scheduling
- Actors are scheduled cooperatively: they run until they return `await` or finish a message handler, then the next actor is picked.
- When a worker queue is empty, it steals work from other workers.

---

## 6. Code Generation

- The C backend emits state‑machine structures for `async` functions.
- Actor mailboxes are implemented as ring buffers.
- The runtime library provides the thread pool, work‑stealing, and supervision logic.
- All I/O operations are integrated with the event loop (to be done in Phase 2e).

---

## 7. Testing

- **Actor messaging:** spawn an actor, send a message, verify response.
- **Async/await:** create an async function, poll it, verify state machine progress and cancellation cleanup.
- **Channels:** test send/receive, bounded overflow, iterator usage.
- **Supervision:** trigger actor panics, verify escalation and process termination.
- **Determinism:** ensure that actor execution order does not affect final state (since actors are isolated).

All tests must pass before Phase 2e begins.
