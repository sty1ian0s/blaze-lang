# Appendix A – Optional Extensions

> **Status:** Normative for implementations that choose to support these features.  None of the items in this appendix are required for core conformance.  They are fully specified so that any conforming implementation that wishes to provide them can do so in a compatible way.

---

## A.1 Persistent Regions

A region annotated with `@persistent` may be mapped to non‑volatile storage (e.g., NVDIMM, battery‑backed RAM, or flash‑backed memory) when the target platform provides a suitable driver.

### A.1.1 Syntax

```
region_decl = "region" [ "@persistent" ] block
```

The `@persistent` attribute is placed on a `region { … }` block:

```blaze
@persistent
region {
    let data = Owned::new(vec![1, 2, 3]);
    // data will survive process restarts
}
```

### A.1.2 Semantics

- All allocations made inside a persistent region are placed in memory that retains its contents across process restarts.
- The runtime must provide a driver that implements the `Allocator` trait backed by persistent memory.
- On restart, a persistent region can be re‑opened by specifying the same unique name or identifier (platform‑dependent).  The runtime must provide a function `Region::open_persistent(name: &str) -> Option<Region>` that returns the existing region if it exists, or creates a new one.
- The data stored in a persistent region must be relocatable (the compiler inserts indirection tables for pointers stored in persistent memory, similar to C++ `offset_ptr`).
- Linear types inside a persistent region are **not** automatically disposed on process exit; the programmer must explicitly commit or rollback changes by ending the region normally (commit) or panicking (rollback, if supported by the platform).

### A.1.3 Testing

- Allocate a value in a persistent region, simulate a restart, re‑open the region, and verify the value is still present.
- Test that linear types are correctly recommitted after restart.

---

## A.2 Distributed Execution

`spawn_on(node, actor)` schedules an actor on a remote node.  The returned `Capability` is network‑transparent, meaning messages sent to the capability are automatically serialised and forwarded to the remote actor.

### A.2.1 Syntax

```
spawn_on(node: NodeId, actor_expression: A) -> Capability<A>;
```

`NodeId` is an opaque identifier obtained from a cluster manager or explicitly configured.

### A.2.2 Semantics

- The actor is serialised (all its fields) using `blaze‑serde` and transmitted to the remote runtime.
- The remote runtime deserialises the actor, spawns it locally, and sends back a `Capability`.
- The local `Capability` is a proxy: all `send` operations on it serialise the message and forward it over the network.
- If the remote node becomes unreachable, the capability transitions to a faulted state; pending sends fail with a `NetworkError`.
- Supervision: the remote actor’s supervisor is the local actor that called `spawn_on` (via a network bridge).  Panic escalation follows the same rules as local supervision (3 panics in 10 seconds → escalate).

### A.2.3 Configuration

Distributed execution requires a `blaze.toml` entry listing known nodes:

```toml
[distributed]
nodes = ["192.168.1.10:9090", "192.168.1.11:9090"]
```

### A.2.4 Testing

- Set up two runtimes, spawn an actor remotely, send a message, and verify the response.
- Simulate a network partition and verify that pending sends fail with `NetworkError`.

---

## A.3 LLVM Backend

An alternative code‑generation backend using LLVM can be enabled with `blaze build --backend=llvm`.

### A.3.1 Activation

```
blaze build --backend=llvm --release
```

- The LLVM backend translates the Blaze UIR into LLVM IR and uses the LLVM optimisation pipeline and code generation.
- All Blaze semantics (linearity, regions, effects, determinism, overflow behavior) are preserved; the LLVM backend is merely an alternative code generator.
- The `--reproducible` flag, when used with the LLVM backend, enables LLVM’s deterministic mode (fixed seeds, stable instruction scheduling) and uses CRlibm for transcendentals.

### A.3.2 Compatibility

- The LLVM backend must support all target triples that LLVM supports.  It is the recommended backend for less common architectures where the native Blaze‑Machine backend may not be available.
- The LLVM backend is optional; a conforming implementation may omit it entirely.

### A.3.3 Testing

- Compile a representative set of Blaze programs with `--backend=llvm` and compare their runtime output with the native backend.
- Benchmark both backends to ensure the LLVM backend does not regress performance significantly.

---

## A.4 C/C++ Transpiler (Community)

A community‑led transpiler, tentatively named `c2blaze`, may translate a restricted subset of C/C++ into Blaze source code.  This tool is not maintained by the Blaze Foundation but is strongly encouraged as an ecosystem project.

### A.4.1 Design

- `c2blaze` leverages **Clang’s AST** and **LLVM IR** to analyse the original C/C++ code.
- It automatically detects common patterns:
  - Local dynamic allocation (`malloc`/`free` in the same function) → `region { }` and arena allocation.
  - Read‑only pointers (`const T*` that are never written through) → `&T` references.
  - Small, trivially‑copyable structs → `@copy` annotation.
  - Fixed‑size arrays → `[T; N]`.
  - Null‑terminated strings → `&str` or `Text`.
  - Integer‑return error conventions (e.g., `-1` on failure, `errno`) → `Result<T, E>`.
- Code that cannot be safely converted remains inside `unsafe` blocks, annotated with comments pointing back to the original source.

### A.4.2 Output

The produced Blaze code is a hybrid: safe Blaze where patterns match, `unsafe` blocks elsewhere.  The developer is expected to iteratively refactor the `unsafe` blocks into safe Blaze over time.
