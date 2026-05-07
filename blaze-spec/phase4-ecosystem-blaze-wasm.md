# Phase 4 – Ecosystem Crate: `blaze‑wasm`

> **Goal:** Specify the `blaze‑wasm` crate, which provides a WebAssembly runtime embedded in Blaze.  It allows loading, compiling, instantiating, and executing WebAssembly modules (.wasm) with full access to Blaze’s linear memory model, regions, and actor system.  The crate supports both the MVP (v1) WebAssembly specification and selected proposals (sign‑extension, mutable globals, SIMD).  All execution carries the `io` effect if the host environment provides external functions that perform I/O.

---

## 1. Core Concepts

A WebAssembly module contains functions, globals, memories, and tables, all of which are accessible from the host (Blaze) after instantiation.  The crate provides:

- **`Engine`** – the top‑level runtime that compiles and caches modules.
- **`Module`** – a compiled WebAssembly module.
- **`Instance`** – an instance of a module, with its own memory, globals, and exports.
- **`Memory`** – a linear memory block (owned or shared).
- **`Func`** – a callable function (imported or exported).
- **`Value`** – an enum representing Wasm values (`i32`, `i64`, `f32`, `f64`, `v128`).
- **`WasmError`** – errors from compilation, instantiation, or execution.

All types are linear where appropriate (Engine, Module, Instance, Memory).

---

## 2. `Engine`

### 2.1 Struct

```
pub struct Engine {
    // configuration, compiled code cache
}
```

- Linear; `Dispose` releases all cached modules and internal data structures.

### 2.2 Methods

```
impl Engine {
    pub fn new(config: &EngineConfig) -> Engine;
    pub fn compile(&self, wasm_bytes: &[u8]) -> Result<Module, WasmError>;
    pub fn instantiate(&self, module: &Module, imports: &Imports) -> Result<Instance, WasmError>;
}
```

- **`compile`**: validates and compiles the given WebAssembly bytecode into internal representation.  The returned `Module` can be instantiated multiple times.
- **`instantiate`**: creates a new `Instance` of the given module, connecting the imports (functions, memories, globals) from the host.

### 2.3 `EngineConfig`

```
pub struct EngineConfig {
    pub max_memory_pages: u32,
    pub max_table_elements: u32,
    pub enable_simd: bool,
    pub enable_threads: bool,
    pub enable_bulk_memory: bool,
}
```

Defaults: 65536 pages (4 GiB), unlimited tables, SIMD/threads/bulk‑memory off by default (feature flags enable them).

---

## 3. `Module`

```
pub struct Module {
    // compiled module data
}
```

- Linear; not `@copy` (contains internal heap allocations).
- Implements `Send` + `Sync` (safe to share across threads).

---

## 4. `Instance`

```
pub struct Instance {
    module: Module,
    memory: Option<Memory>,
    exports: Exports,
}
```

- Linear; `Dispose` removes the instance.

### 4.1 Methods

```
impl Instance {
    pub fn get_export(&self, name: &str) -> Option<&Extern>;
    pub fn memory(&self) -> Option<&Memory>;
    pub fn call(&self, func_name: &str, args: &[Value]) -> Result<Vec<Value>, WasmError>;
}
```

- **`get_export`**: returns a reference to an exported item (function, memory, global, table) wrapped in `Extern`.
- **`memory`**: convenience accessor for the instance’s default memory.
- **`call`**: invokes an exported function by name, passing a slice of `Value`s, and returns the result values.

---

## 5. `Extern` Enum

```
pub enum Extern {
    Func(Func),
    Memory(Memory),
    Global(Global),
    Table(Table),
}
```

- Each variant holds an owned reference to the respective resource.
- Can be used to build `Imports` when instantiating another module.

---

## 6. `Import` Building

### 6.1 `Imports`

```
pub struct Imports {
    functions: Map<(Text, Text), Func>,
    memories: Map<(Text, Text), Memory>,
    globals: Map<(Text, Text), Global>,
    tables: Map<(Text, Text), Table>,
}
```

- Represents the host environment’s exports that a module can import.
- Filled by the user via a builder pattern:

```
let imports = Imports::new()
    .func("env", "log", host_log_func)?
    .memory("env", "memory", host_memory)?
    .done();
```

### 6.2 `Func` (Host Functions)

Host functions can be created from Blaze closures:

```
pub struct Func { /* … */ }
impl Func {
    pub fn new<F>(f: F) -> Func where F: Fn(&[Value]) -> Result<Vec<Value>, WasmError> + Send + 'static;
    pub fn call(&self, args: &[Value]) -> Result<Vec<Value>, WasmError>;
}
```

- The host function can capture state (linear or otherwise) via the closure.
- If the host function interacts with memory, it can access the instance’s memory via a provided pointer (by calling `Instance::memory`).

---

## 7. `Memory`

```
pub struct Memory {
    buf: Vec<u8>,
    max_pages: u32,
}
```

- Linear; `Dispose` frees the underlying buffer.
- Supports `read`/`write` methods:
  - `pub fn read(&self, offset: usize, buf: &mut [u8]) -> Option<()>`
  - `pub fn write(&mut self, offset: usize, data: &[u8]) -> Option<()>`
- Memory size is always a multiple of page size (64 KiB).  Grows using `grow(pages) -> Option<u32>`.

---

## 8. `Global` and `Table`

```
pub struct Global { value: Value, mutable: bool }
impl Global {
    pub fn get(&self) -> Value;
    pub fn set(&mut self, value: Value) -> Result<(), WasmError>;
}

pub struct Table { elements: Vec<Func>, max: Option<u32> }
impl Table {
    pub fn get(&self, index: u32) -> Option<&Func>;
    pub fn set(&mut self, index: u32, func: Func) -> Option<()>;
    pub fn grow(&mut self, delta: u32) -> Option<u32>;
}
```

---

## 9. `Value` Enum

```
pub enum Value {
    I32(i32),
    I64(i64),
    F32(f32),
    F64(f64),
    V128(u128),   // SIMD value
}
```

Implements `From`/`Into` for basic types.

---

## 10. Error Handling

```
pub enum WasmError {
    Compile(Text),
    Instantiate(Text),
    Runtime(RuntimeError),
    MemoryOutOfBounds,
    Unreachable,
    TypeMismatch(Text),
    ImportNotFound(Text),
}

pub struct RuntimeError {
    pub message: Text,
    pub function: Option<Text>,
    pub wasm_stack_trace: Vec<Frame>,
}
```

- `RuntimeError` carries a message and a Wasm stack trace captured at the point of trap.

---

## 11. Implementation Notes

- The crate uses the native Blaze‑Machine backend (or LLVM) to compile Wasm modules to native code, providing near‑native execution speed.  Alternatively, a portable interpreter can be used for platforms without native backends.
- The `Memory` struct is directly exposed as a Blaze byte buffer, enabling zero‑copy access to Wasm linear memory from host functions.
- When the `threads` feature is enabled, `Memory` can be shared across actors via `Arc` (but remains linear in the default mode).

---

## 12. Testing

- **Compile and instantiate:** Load a simple `.wasm` binary (e.g., `(module (func (export "add") (param i32 i32) (result i32) (i32.add (local.get 0) (local.get 1))))`), instantiate it, call `"add"` with arguments, and verify the result.
- **Memory access:** Instantiate a module with initial memory, call a function that writes to memory, then read the memory buffer from the host and verify the bytes.
- **Host functions:** Import a host function that performs an operation (e.g., `console.log` equivalent), call it from Wasm, and verify the closure is invoked correctly.
- **Error handling:** Provide malformed Wasm bytes, expect `WasmError::Compile`.  Call a function with wrong argument types, expect `TypeMismatch`.
- **Dispose:** Ensure that dropping an `Instance` frees its memory and globals.
- All tests must pass on all supported targets (x86‑64, aarch64, possibly wasm32 itself for meta).
