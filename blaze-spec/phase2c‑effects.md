# Blaze Phase 2c – Effect System & Purity

> **Goal:** Implement the effect system: tracking effect sets, inferring minimal effect sets for each function, effect polymorphism, purity detection, and auto‑parallelisation of pure loops.  The parser already accepts effect annotations (`/ io`, `/ pure`, etc.) and the `seq for` syntax.  This phase adds semantic analysis for effects and the corresponding code generation / runtime integration.

---

## 1. Effect Sets

### 1.1 Built‑in Effects
- `alloc` – performs dynamic memory allocation.
- `io` – performs I/O (implies `alloc`).
- `parallel` – may execute in parallel (requires thread‑safe data).
- `gpu` – uses GPU resources (implies `parallel`).
- `hal` – uses hardware abstraction layer (implies `io`).

### 1.2 Effect Implications
```
io  ⇒ alloc
gpu ⇒ parallel
hal ⇒ io
```
The lattice of effect sets is the power set of `{alloc, io, parallel, gpu, hal}` with the following order: a set `A` is smaller than `B` if `B` contains `A` and the implications are respected (e.g., `{alloc}` is smaller than `{alloc, io}` because `io` implies `alloc`).

### 1.3 Effect Annotations
- Functions may declare their effect set: `fn foo() -> i32 / io`.
- If no annotation is present, the compiler infers the minimal set.
- The declared set must be a superset of the inferred set (checked by the compiler).

---

## 2. Effect Inference Algorithm

### 2.1 Bottom‑Up Fixed‑Point Analysis
The compiler computes a minimal effect set for each function using a fixed‑point iteration over the call graph.

**Initialization:**
- For each function, set its effect set to `∅` (empty).

**Iteration (repeat until no changes):**
For each function `f`:
1. **Local effects:**
   - If the body contains an explicit allocation site (e.g., call to an `Allocator::allocate`), add `alloc`.
   - If the body calls a function with a declared effect set `E`, add `E` to `f`'s set.
   - If the body contains `?` or `catch` that constructs an error value (i.e., a heap‑allocating error variant), add `alloc`.
   - If the body contains a `for` loop whose body is pure (see below), add `parallel`.
   - If the body contains `unsafe` blocks, no additional effects are inferred beyond the operations inside them.
2. **Panics, unreachable, todo:**
   - `panic!`, `todo!`, `unreachable!` contribute **no effect** to the set.  (They are terminal and do not imply any side effect.)
3. **Closures:**
   - A closure is treated like an anonymous function.  Its effect set is inferred from its body.
4. **Recursive functions:**
   - If `f` calls itself, the fixed‑point iteration handles it; the effect set stabilizes when all call sites have been accounted for.

**Termination:**  
The lattice has finite height (max 5 elements), so the algorithm terminates in at most 5 iterations over the whole program.

### 2.2 Purity
A function is **pure** if its inferred effect set is empty (`∅`).
- The compiler marks pure functions with a special flag for later optimizations.
- Pure functions cannot call any function that has a non‑empty effect set.

---

## 3. Effect Polymorphism

### 3.1 Effect Variables
A generic parameter declared as `effect E` allows a function to be parameterised by the effect set of a closure argument.
```
fn map<T, effect E>(v: Vec<T>, f: fn(T) -> T / E) -> Vec<T> / E;
```
- The effect variable `E` binds to the actual effect set of the closure passed as `f`.
- The return type’s effect set and the function’s overall effect are unified with `E` (i.e., the function inherits the effect of the closure).

### 3.2 Inference with Effect Variables
When a function with an effect parameter is called, the compiler infers the concrete effect set from the actual argument.  
- For example, `map(vec, \x -> x + 1)` gives `E = ∅`.
- `map(vec, \x -> { println(x); x })` gives `E = {io}`.
- If the same effect variable appears in multiple positions, the concrete set is the union of all inferred sets.

---

## 4. Auto‑Parallelisation of Pure Loops

### 4.1 Criteria
A `for` loop is automatically parallelised if:
- The loop body is pure (effect set `∅`).
- There is no explicit `seq for` modifier.
- The loop is not inside a `@comptime` function (which runs at compile time, single‑threaded).

### 4.2 Implementation
- The compiler emits a parallel version using the runtime’s work‑stealing thread pool.
- The loop iterations are split into chunks and distributed across worker threads.
- The effect `parallel` is added to the enclosing function’s effect set (even if the developer did not annotate it).  This ensures that any function that performs auto‑parallelisation is marked as potentially parallel, which in turn requires that its data be thread‑safe.

### 4.3 Thread‑Safety Requirements
- Data accessed in the loop body must satisfy `Send` and `Sync` (see later phase).  
- For now, since we haven’t fully implemented `Send`/`Sync` auto‑derivation, the compiler will **not** auto‑parallelise any loop that accesses non‑primitive types.  We will add the full thread‑safety analysis in a subsequent phase.  For Phase 2c, auto‑parallelisation is only enabled for loops over primitive types and simple arrays.

### 4.4 `seq for`
- Explicit `seq for` forces sequential execution even if the loop body is pure.
- It does **not** add `parallel` to the effect set.

---

## 5. Code Generation & Runtime Integration

### 5.1 Effect Checks
- The compiler inserts no runtime checks for effects; they are purely compile‑time.
- When generating code, the compiler uses the effect information to decide whether to emit parallelised loops (see above).

### 5.2 Runtime Thread Pool
- The runtime (to be developed in Phase 2d) provides a global work‑stealing thread pool.  For now, we emit calls to a simple library that provides `parallel_for` (a placeholder that may later be replaced by the actor runtime).
- The C backend translates a parallelised loop into a call to a runtime function `blaze_parallel_for` that takes a chunked iteration count and a callback.

### 5.3 Determinism
- The parallelisation must be deterministic: identical inputs produce identical outputs regardless of thread scheduling.  This is guaranteed because the loop body is pure (no side effects).

---

## 6. Testing

Tests for this phase:

- **Inference:** Write functions with various combinations of allocations, I/O, and calls; verify that the compiler infers the expected effect set (the test harness can query the compiler’s output, e.g., via a compiler flag that dumps inferred sets).
- **Explicit annotations:** Declare a function with an effect set that is too small; the compiler must reject it with an error.
- **Effect polymorphism:** Write a generic `map` and call it with pure and impure closures; verify that the instantiated effect sets are correct.
- **Purity:** Mark a pure function; verify that it cannot call an impure function.
- **Auto‑parallelisation:** Write a pure loop over an array of integers; verify that the generated code includes a parallel call.  Then force `seq for` and verify sequential code.
- **No parallelisation of impure loops:** Write a loop that calls `println` (which has an effect) and verify it is not parallelised.
- All tests must pass before moving to Phase 2d.
