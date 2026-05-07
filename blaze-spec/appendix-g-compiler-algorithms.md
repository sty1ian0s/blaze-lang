# Appendix G – Compiler Algorithm Details

> **Status:** Normative.  This appendix contains the exact algorithms and data structures required to implement the semantic analysis and optimisation passes of the Blaze compiler.  Every rule is specified to the level of pseudocode or clear English that can be directly translated into code.

---

## G.1 Linearity Checking

The linearity checker operates on each function body independently.  It maintains:

- `Live` – a set of variable names that are currently initialised (live).
- `Partial` – a map from variable name to a set of field indices that have been moved out, if only some fields were moved.

The checker processes the function body statement by statement in source order.  The following rules are applied.

### G.1.1 Scalar Variables

Let `v` be a variable of a type that is not a struct (or is a struct but has no fields).

**Read of `v`:**
- If `v` is in `Live` and `Partial` is empty, the read is valid.  Remove `v` from `Live` (affine consumption).  The value is moved.
- If `v` is not in `Live` or is partially moved, emit an error “use of moved value”.

**Assignment to `v`:**
- If `v` is not in `Live`, add it to `Live`.  The right‑hand side is consumed normally.
- If `v` is in `Live` and `Partial` is empty, emit an error “cannot assign to live variable without explicit reinitialisation” (unless it’s a re‑initialisation after partial move, handled below).

**Binding `let v = expr;`:**
- `expr` is evaluated and consumed.  `v` is added to `Live`.

**Passing as argument:**
- Same as read: consume `v`.

**End of scope:**
- For every variable in `Live` that implements `Dispose`, insert a call to `dispose(&mut self)` in reverse declaration order.
- For any variable not implementing `Dispose`, emit an error “linear variable not consumed”.

### G.1.2 Struct Fields

Let `s` be a variable of struct type with fields `f1, …, fn`.

**Read of `s.f_i`:**
- If `s` is in `Live` and `Partial` does not contain index `i`, the read is valid.  Add `i` to `Partial`.  If `Partial` now contains all fields (0..n), remove `s` from `Live` and clear `Partial`.  The field is moved; the struct is now partially valid.

**Assignment to `s.f_i`:**
- If `s` is in `Live` and `Partial` contains `i`, remove `i` from `Partial`.  If `Partial` becomes empty, the struct is fully live again.

**Whole‑struct read:**
- Same as scalar read; `s` is removed from `Live`.

**Whole‑struct assignment:**
- Same as scalar assignment; `s` becomes (or remains) live.

### G.1.3 Branches (if/else, match)

Let `L_in` be the live/partial state before the branch.  Compute `L_out` as:

- If the branch is an `if` without `else`, `L_out = L_in` (no change, because the `if` may not execute).  However, any variables moved inside the `if` are considered **potentially moved** and must be treated as dead after the merge unless reinitialised in the branch.  In practice, the compiler conservatively marks them as dead.
- For `if/else`, compute the live/partial state after each branch independently.  Then, for each variable:
  - If it is live in **both** branches (with identical partial sets), it is live after the merge.
  - If it is dead in either branch, it is dead after the merge (unless reinitialised in all branches).
  - If partial sets differ, the struct is dead after the merge (cannot be used safely).

The compiler must insert `dispose` calls for any fully‑live variable that becomes dead at the end of a branch, but this is handled by the scope‑exit rule (end of block).

### G.1.4 Loops (`while`, `loop`)

Treat the loop body as a function.  The live/partial state at the start of the body is the same as before the loop (for the first iteration), and the state at the end of the body becomes the state at the start of the next iteration.  Run a fixed‑point iteration over the loop:

- Start with `live_start = L_in`.
- Compute `live_end` by processing the body.
- If `live_end` differs from `live_start`, set `live_start = live_end` and recompute.
- The final `live_start` is the live state at loop entry.

After the loop, all variables that were live at entry remain live (since the loop may not execute).  Variables that were moved inside the loop are conservatively considered dead.

### G.1.5 Function Calls and Returns

- **Function call:** each argument is consumed (as a read).  The return value is a new binding.
- **Return statement:** the returned expression is consumed.  All live variables in the current scope are implicitly disposed (unless moved into the return value).  The compiler inserts `dispose` calls for unconsumed live variables.

---

## G.2 Effect Inference

### G.2.1 Effect Lattice

Define the effect lattice as the power set of `{alloc, io, parallel, gpu, hal}` with the following partial order and join operation:

- Implications: `io` ⊇ `alloc`; `gpu` ⊇ `parallel`; `hal` ⊇ `io`.
- Join (⊔): union of sets, then close under implications.
- Bottom: ∅; Top: all five effects.

### G.2.2 Intra‑procedural Analysis

For each function body:

1. Initialize the function’s effect set to ∅.
2. Traverse the body:
   - For an expression that allocates (e.g., `Arena::allocate`, `Box::new`, `vec![…]`), add `alloc`.
   - For a call to another function `f`, add the declared effect set of `f` (or its inferred set if already computed).  If `f` is generic and not yet instantiated, delay analysis until monomorphisation; for non‑generic functions, use the current fixed‑point set.
   - For `?` or `catch`: if the error value carries a boxed/heap‑allocating payload (e.g., `Box<dyn Error>`), add `alloc`.  If the error value is a simple enum variant (plain `Err(e)` with a non‑heap `E`), no effect is added.
   - For `panic!`, `todo!`, `unreachable!`: add nothing.
   - For a `for` loop whose body is pure (effect set ∅), add `parallel`.  A `seq for` loop does not add `parallel`.
3. Store the final effect set.

### G.2.3 Fixed‑Point Iteration (Inter‑procedural)

The call graph may be recursive.  Use a standard worklist algorithm:

- Initialize all functions’ effect sets to ∅.
- Repeat until no set changes:
  - For each function, recompute its effect set using the current sets of its callees.
  - If a set changes, re‑add all callers to the worklist.

Termination is guaranteed because the lattice has finite height.

---

## G.3 @comptime Interpreter

The interpreter is a stack machine that executes a byte‑code representation of the UIR of a `@comptime` function.  It is deterministic.

### G.3.1 Instruction Set

| Mnemonic | Operands | Description |
|----------|----------|-------------|
| `PUSH_LIT` | `value` (i64 or f64) | Push a literal onto the stack. |
| `PUSH_VAR` | `idx` (variable index) | Push the value of a local variable. |
| `POP` | | Discard top of stack. |
| `STORE` | `idx` | Pop the top of stack and store in local variable `idx`. |
| `ADD` | | Pop two values, push their sum. |
| `SUB` | | Pop two values, push `b - a` (where `a` was top). |
| `MUL` | | Pop two values, push their product. |
| `DIV` | | Pop two values, push `b / a`. |
| `REM` | | Pop two values, push remainder. |
| `NEG` | | Pop one value, push its negation. |
| `EQ` / `NE` / `LT` / `LE` / `GT` / `GE` | | Pop two values, push boolean result. |
| `NOT` | | Pop one value, push logical not. |
| `AND` / `OR` | | Pop two, push logical and/or (short‑circuit handled by compiler, but here we evaluate both). |
| `JUMP` | `offset` (i32) | Unconditional jump. |
| `JUMP_IF_FALSE` | `offset` | Pop value; jump if it is false. |
| `CALL` | `fn_ptr` | Call another @comptime function. |
| `RETURN` | | Return from function (top of stack is return value). |
| `MAKE_TUPLE` | `n` | Pop `n` values, push a tuple. |
| `MAKE_STRUCT` | `name_idx` | Pop field values according to struct definition, push struct. |
| `FIELD_ACCESS` | `idx` | Pop a struct/tuple, push field `idx`. |
| `ARRAY_INDEX` | | Pop array and index, push element. |
| `REFLECT` | `op` | Various reflection operations (type_of, fields_of, etc.). |

All arithmetic follows the same overflow rules as debug mode (panic).  Floating‑point uses roundTiesToEven.  The interpreter maintains a stack of `Value` (an enum of integer, float, bool, char, tuple, struct, array, etc.).

### G.3.2 Interpreter Loop

The interpreter reads instructions sequentially.  The call stack stores return addresses.  When `CALL` is executed, the current instruction pointer is pushed onto the call stack and the target function’s first instruction is executed.  `RETURN` pops the call stack and continues.

The interpreter is embedded in the compiler; it is not a separate process.  The `@comptime` environment is fully deterministic: no I/O, no randomness, no system calls.

---

## G.4 Optimisation Passes

The optimiser consists of three groups of passes, executed in the order given.  Each group uses a worklist algorithm: passes are applied repeatedly until no more changes occur.

### G.4.1 Canonicalisation (enabled at all opt‑levels)

1. **Dead Code Elimination (DCE)** – remove instructions whose result is never used.
2. **Constant Folding** – evaluate constant expressions at compile time, replace with literal.
3. **Global Value Numbering (GVN)** – identify equivalent computations and replace redundant ones.
4. **Partial Redundancy Elimination (PRE)** – move computations to optimal points to avoid duplication.
5. **Loop‑Invariant Code Motion (LICM)** – move loop‑invariant computations out of loops.
6. **Strength Reduction** – replace expensive operations with cheaper ones (e.g., `x * 8` → `x << 3`).
7. **Induction Variable Optimisation** – simplify loop induction variables (e.g., replace `i*stride + base` with a pointer increment).

### G.4.2 Specialisation (opt‑level ≥ 1)

1. **Inlining** – substitute the body of a callee directly at the call site when the callee is small (heuristic: ≤ 50 instructions) or marked `#[inline]`.
2. **Auto‑vectorisation** – detect loops that can be mapped to SIMD instructions; emit SIMD intrinsics or LLVM metadata.
3. **SoA Transformation** – for structs annotated with `@layout(soa)`, transform arrays of structs into structure‑of‑arrays by rewriting all access patterns.
4. **Auto‑parallelisation** – for pure `for` loops, emit code that splits iterations across the runtime’s thread pool.
5. **Allocation Elision** – replace heap allocations with stack or register storage when the lifetime is local and the size is known.
6. **Region Fusion** – merge adjacent `region { … }` blocks that use the same allocator, reducing overhead.
7. **Tail‑Call Optimisation** – convert a call in tail position into a jump.
8. **Devirtualisation** – when the concrete type of a `dyn Trait` is statically known, replace dynamic dispatch with a direct call.

### G.4.3 Code‑Gen Preparation (applied at all opt‑levels before code emission)

1. **Bounds Check Elision** – remove array/slice bounds checks that can be proven safe via range analysis.
2. **Peephole Optimisation** – local instruction pattern replacement (e.g., `mov x, 0; add x, 1` → `mov x, 1`).
3. **Prefetch Insertion** – insert cache‑prefetch instructions for predictable memory access patterns.
4. **Alignment** – align memory accesses to preferred boundaries (e.g., 16‑byte for SIMD).
5. **Non‑temporal Stores** – use streaming store instructions for write‑only data that is not reused soon (detected via dead‑store analysis).
6. **Loop Versioning** – split a loop into a fast path (with assumptions like alignment, non‑aliasing) and a slow path.
7. **Redundant Store Elimination** – remove stores to memory locations that are overwritten before being read.
8. **Idiom Recognition** – detect known patterns (e.g., `memcpy`, `memset`, `memcmp`) and replace with highly optimised built‑ins.

---

## G.5 Region Lifetime Inference (NLL)

The compiler infers non‑lexical lifetimes for references as follows:

- For each borrow expression (creation of `&` or `&mut`), record the **borrow point** (instruction index) and the **borrowed place** (variable + optional path).
- For every use of the borrowed place (read or write), record a **use point**.
- A borrow is **live** from its creation until its **last use**.
- At a **region end** (end of a `region { }` block or function return), check: is any borrow into that region still live?  If so, emit an error “borrow may outlive its region”.
- `&mut` borrows are treated as exclusive: two mutable borrows to the same place must not overlap in their live ranges; otherwise emit a warning (not an error, since Blaze does not enforce strict aliasing, but the language encourages clean usage).

The analysis is flow‑sensitive and intra‑procedural.  It uses the same live‑variable framework as linearity checking.

---

## G.6 Array Bounds Check Elision

For each array access `arr[i]`, the compiler tries to prove that `0 ≤ i < length(arr)`.

- Track the possible range of `i` using a simple value‑range lattice.  Initially, `i ∈ [min_int, max_int]`.
- Refine the range based on comparisons (`if i >= 0 { … }`, loops with known bounds, etc.).
- If at the access point the proven range is within `[0, len-1]`, the bounds check is removed.
- This analysis is intra‑procedural and based on an SSA representation of the function.  It runs before codegen.

---

## G.7 Actor Runtime Scheduling

The runtime uses a work‑stealing thread pool with one worker thread per physical core.

- Each worker maintains a local deque of actor mailboxes.
- When an actor is spawned, its mailbox is placed on the spawning worker’s deque.
- A worker processes its deque in FIFO order (its own actors) and steals from the back of other workers’ deques when idle (LIFO for work‑stealing).
- An actor’s mailbox is a lock‑free SPSC queue.  When a `Capability` is cloned, the mailbox is upgraded to MPSC via a small lock (or an atomic queue).  Sending a message pushes an item; if the queue is full, the sender either yields or the queue grows.
- The actor runs when scheduled.  It processes one message (or a batch) and then yields.  If an actor panics, the supervisor is notified.  The supervisor escalation logic is a separate actor that receives panic notifications and tracks the count per actor.  If >3 panics in 10 seconds, it escalates to its own parent.

For distributed actors (`spawn_on`), the actor’s state is serialised (via `blaze‑serde`) and sent to the remote node’s runtime, which deserialises and spawns it locally.  A network proxy capability forwards sends over the network.

---

## G.8 @derive Expansion

The compiler supports a set of built‑in derive macros.  The expansion is **syntactic** and produces an `impl` block.

### G.8.1 `Debug`

For a struct `S` with fields `f1`, …, `fn`:
- Generate:
```
impl Debug for S {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_struct(stringify!(S))
            .field("f1", &self.f1)
            // …
            .field("fn", &self.fn)
            .finish()
    }
}
```
For an enum, generate a match over variants, each branch writing the variant name and its payload fields similarly.

### G.8.2 `PartialEq`

For a struct: generate `eq(self, other)` that calls `eq` on each field and combines with `&&`.  For an enum, first compare discriminants, then decompose.

Requires all fields to implement `PartialEq`.

### G.8.3 `Default`

For a struct: generate `fn default() -> Self` that calls `Default::default()` on each field.  If the struct has field defaults, those are used; otherwise, the field type’s `Default` is required.

### G.8.4 Other Traits

- `Clone`, `Binary`, `Display`, `From<T>`, `Into<T>`, `AsRef<T>`, `AsMut<T>`, `Error`, `FromRow` are generated following similar structural rules (see the specific trait documentation in the standard library).

The expansion is triggered by the `@derive` attribute and the compiler inserts the generated code before semantic analysis.

---

## G.9 `.blzlib` Format

A `.blzlib` file is a pre‑compiled standard library archive.  Its physical format is a `.tar` archive containing exactly three files:

### G.9.1 `metadata.json`

A JSON object with the following schema:
```json
{
    "version": "1.0",
    "target": "x86_64-unknown-linux-gnu",
    "opt_level": "1",
    "modules": [
        {
            "name": "std::builtins",
            "symbols": [
                "Option",
                "Result",
                "Range",
                "PanicInfo",
                "…"
            ]
        }
    ]
}
```
The `modules` array lists every public symbol (type, function, trait) exported by that module.

### G.9.2 `object.o`

An ELF (or COFF on Windows) object file containing the compiled code for all modules listed in the metadata.  The symbols are in the same order and mangled according to the Blaze ABI.

### G.9.3 `debug.blzdbg`

A binary file containing debug information in the Blaze Debug Format (BDF), which is a superset of DWARF.  It includes type information, source locations, and variable names.  The compiler uses this to emit debug info in the final binary and for `blaze debug`.

The compiler loads `.blzlib` files by extracting the tar, reading the metadata, and linking the object file directly.  If the target triple or opt‑level does not match, it falls back to compiling the library from source.
