# Blaze Phase 2f – Attributes and Zero‑Cost Abstractions

> **Goal:** Implement all remaining attributes: built‑in expansions, conditional compilation, parser‑facing ZCAs, comptime‑expanded macros (excluding `@fuzz` and `@test_case` which are already done), domain‑specific bundles, and annotation attributes.  The parser already accepts all attribute syntax; this phase adds the expansion logic during parsing or semantic analysis as appropriate.

---

## 1. Built‑in Expansions

These attributes expand into other attributes or trait derivations.

### 1.1 `@data`
- Expands to `@copy` + `@derive(Debug, Binary, Default, PartialEq, PartialOrd)`.
- If the type cannot be `@copy` (size > 16 bytes, contains pointers, etc.), the compiler emits an error.

### 1.2 `@derive(trait_list)`
- Generates an `impl` block for each listed trait.
- Supported traits: `Debug`, `Binary`, `Default`, `PartialEq`, `PartialOrd`, `Clone`, `Display`, `From<T>`, `Into<T>`, `AsRef<T>`, `AsMut<T>`, `Error`, `FromRow`.
- The compiler generates the implementation automatically:
  - `Debug`: format the type name and each field’s debug representation.
  - `PartialEq`: chain `eq` on each field with `&&`.
  - `Default`: construct using each field’s default value.
  - `Clone`: clone each field.
  - `Display`: human‑readable format.
  - `From`/`Into`: for tuple structs with one field.
  - `AsRef`/`AsMut`: for newtype structs.
  - `Error`: for types implementing `Display` (generates `Error` trait impl).
  - `FromRow`: for database row deserialization.
- For each generated trait, if any field lacks the required trait, the derive fails with a clear error.

### 1.3 `@scale`
- Hints the compiler that numeric precision may be automatically scaled (used in graphics/simulation).
- Implementation: no‑op for now, but reserved.

### 1.4 `@variant_partition`
- On a collection of an enum, transforms it into separate arrays per variant (SoA with variant indexing).
- The compiler generates a function that does the partitioning and accesses the arrays directly.

### 1.5 `@sift`
- Creates a boolean index array for branch elimination on condition‑based iteration.
- Implementation: a helper function that evaluates each element and builds a bitmask.

### 1.6 `@hot` / `@cold`
- Hints for the optimizer about frequently/uncommonly executed code.
- The backend may use this for code placement; no semantic effect.

### 1.7 `@native`
- Marks a function as having platform‑specific implementations selected by target.
- Multiple `@native(target_os = "linux")` etc. annotations on the same function.
- The compiler picks the body matching the current target and ignores others.

### 1.8 `@packed`
- Bit‑packed layout for struct fields when not combined with `@layout`.
- The compiler generates getters and setters that extract/insert bits.

---

## 2. Conditional Compilation

### 2.1 `@cfg(condition)`
- Includes the decorated item only if the compile‑time condition is true.
- Conditions: `target_os`, `target_arch`, `feature`, `target_endian`, and combinations via `any(…)`, `all(…)`, `not(…)`.
- Evaluated during parsing; excluded items are removed from the AST entirely.

### 2.2 `@cfg_attr(condition, attrs...)`
- Applies the given attribute(s) only if the condition holds.
- Also evaluated at parse time.

The compiler must know the target platform at build time (from `--target` or host defaults) and any feature flags (`-C feature=...`).

---

## 3. Parser‑Facing Zero‑Cost Abstractions

These expand during parsing before semantic analysis.

### 3.1 `@enum_consts`
- On an enum, generates `const VARIANT = Enum::Variant;` for each variant.
- The constants are placed in the same module.

### 3.2 `@tuple_struct`
- On a struct, generates:
  - A constructor function `fn new(field1: T1, field2: T2, ...) -> Self`.
  - An indexing getter `fn get(&self, i: usize) -> Option<&Field>` (returns reference to the field at index `i` or `None`).

### 3.3 `@newtype(T)`
- On a struct with exactly one field of type `T`, generates:
  - `fn from_inner(val: T) -> Self`
  - `fn into_inner(self) -> T`
  - `impl Deref<Target = T>` and `impl DerefMut` for the struct.

---

## 4. Comptime‑Expanded Macros (Partial List)

These attributes trigger compile‑time function calls to generate code.  (The full list includes many domain‑specific macros; we list here the ones that need immediate implementation.)

### 4.1 Already Implemented
- `@fuzz` and `@test_case` from Phase 2e.

### 4.2 Required for Phase 2f
- `@builder`: generates a builder pattern for a struct.
- `@state_machine`: generates a state machine from an enum of states.
- `@validate`: generates a validation function from struct field annotations.
- `@trace`, `@metrics`, `@observe`: instrumentation macros.
- `@throttle`, `@debounce`, `@retry`: concurrency patterns.
- `@event`, `@inject`, `@lazy`: dependency injection and lazy initialization.
- `@ffi(delegate)`: automatic FFI delegation.
- `@paginate`, `@schedule`, `@cache`, `@circuit_breaker`, `@max_concurrency`, `@idempotent`, `@watch`, `@lockstep`, `@raft`, `@history`, `@rest`, `@tail_sensitive`, `@workflow`.

Each macro is implemented as a comptime function that receives the annotated AST node and returns a token stream with the generated code.  Exact semantics are defined in the extended specification; for now, we implement a minimal version that generates correct scaffolding and defers full logic to later phases if needed.

---

## 5. Domain‑Specific Bundles

These are meta‑attributes that expand to multiple attributes.

| Bundle | Expands to |
|--------|------------|
| `@web` | `@rest, @auth, @cors, @session` |
| `@game`| `@simulate, @ecs, @physics(2d), @audio` |
| `@embedded` | `@no_std, @no_runtime, @panic("abort"), @pool` |
| `@cli_tool` | `@cli, @config, @validate` |
| `@browser` | `@blaze‑web, @wasm, @web` |

- `@browser` adds the `blaze‑web` crate dependency, sets the target to `wasm32-unknown-unknown`, and includes the `@web` bundle for any server‑side API the front‑end may consume.
- The `@wasm` attribute is a new meta‑attribute that activates the LLVM WebAssembly backend and the `blaze‑wasm‑bindgen` pipeline.
- When `@browser` is present, `blaze build` produces a `.wasm` binary and a JavaScript glue file, ready to be served from a standard web server.

The expansion is a simple textual replacement at the attribute level.

---

## 6. Annotation Attributes

These do not expand; they are markers used by the compiler or tooling.

- `@copy` – marks a type as copyable (already implemented in Phase 2a).
- `@comptime` – marks a function as compile‑time only.
- `@layout(...)` – controls memory layout (`soa`, `auto`, `packed`, `C`).
- `@float_epsilon(...)` – sets the epsilon for `ApproxEq`.
- `@overflow` – controls overflow behavior (`wrap`, `panic`).
- `@must_use` – warns if a return value is ignored.
- `@gpu`, `@hardware`, `@parallel` – effect annotations (handled by effect system).
- `@test`, `@bench` – marks test/benchmark functions.
- `@deprecated` – marks an item as deprecated.
- `@cold`, `@reply`, `@feature`, `@no_std`, `@no_runtime`, `@panic("abort")`.
- Window hints: `@slide(N)`, `@window_nobounds`, `@window_accumulate`.
- `@debug_visualizer(closure)` – provides a custom debug display.

The compiler stores annotation attributes in the AST and acts on them as defined in previous phases or simply preserves them for later tooling use.

---

## 7. Attribute Decision Table & LSP

- The language server (`blaze lsp`) uses a pre‑computed table (normative but maintained separately) to suggest attributes when it detects specific patterns (e.g., a struct with many fields → suggest `@derive(Debug)`).
- The table is not part of the compiler core but is loaded by the LSP from a standard location.

---

## 8. Testing

For each attribute category:

- **Built‑in expansions:** Write structs/enums and verify that the generated code compiles and behaves correctly (`@data` must enable copy; `@derive` must produce correct impls).
- **Conditional compilation:** Provide source files with `@cfg` and `@cfg_attr`; compile with different `--target` and `--feature` flags and verify inclusion/exclusion.
- **Parser‑facing ZCAs:** `@enum_consts` must produce usable constants; `@tuple_struct` must provide the constructor; `@newtype` must generate the conversion functions.
- **Comptime macros:** For each implemented macro, write a test that verifies the generated code (e.g., `@builder` must generate a builder struct with setter methods).
- **Bundles:** `@web` etc. must expand correctly.
- **Annotation attributes:** Verify that they are stored and accessible via reflection.
- All tests must pass before the phase is considered complete.
