# Phase 4 – Ecosystem Crate: `blaze‑wasm‑bindgen`

> **Goal:** Specify the `blaze‑wasm‑bindgen` crate, which automates the generation of WebAssembly bindings from Blaze types and function signatures.  It provides a set of `@comptime` macros and a build‑time tool that converts Blaze modules into `.wasm` components with exported interfaces and imported host bindings.  The crate builds on `blaze‑wasm` and `blaze‑serde`, leveraging Blaze’s data‑oriented design to produce compact, zero‑cost wrappers.

---

## 1. Core Concepts

WebAssembly modules need a specific ABI to communicate with their host.  This crate generates that ABI automatically:

- **`#[wasm_bindgen]`** – an attribute (comptime macro) placed on functions, structs, enums, and impl blocks to instruct the compiler to generate Wasm‑compatible wrappers.
- **`wasm‑bindgen‑tool`** – a command‑line tool (`blaze wasm‑bindgen`) that processes a Blaze library crate and emits a `.wasm` binary with a JavaScript/TypeScript glue file (or raw host bindings for another Wasm‑enabled language).
- **`JsValue`** – a type representing any JavaScript value (for browser‑based hosts).
- **`Closure`** – a linear wrapper around a Blaze closure that can be passed to JavaScript as a callback.
- **`Promise`** – a representation of a JavaScript Promise (if targeting a JS host), allowing async Blaze functions to be awaited in JS.

The crate’s design follows Blaze’s zero‑cost principle: the generated wrappers have no runtime overhead beyond the necessary ABI conversions.

---

## 2. `#[wasm_bindgen]` Macro

### 2.1 Placement

The attribute can be applied to:

- Functions – exported to the host.
- Structs and enums – exposed to the host as classes with methods.
- `impl` blocks – marked methods become methods on the exported class.
- `extern` blocks – specifies imported host functions.

### 2.2 Exported Functions

```
#[wasm_bindgen]
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}
```

The attribute generates:

- A WebAssembly‑exported function `add` with the correct ABI (taking `i32, i32` and returning `i32`).
- If the function uses types not representable as Wasm primitives (e.g., `Text`, `Vec<u8>`, structs), the macro generates serialization/deserialization glue using `blaze‑serde` (JSON or a custom binary format) behind a `*mut u8` + length pair, managed by the host’s memory.

### 2.3 Exported Types

```
#[wasm_bindgen]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

#[wasm_bindgen]
impl Point {
    pub fn new(x: f64, y: f64) -> Point { Point { x, y } }
    pub fn distance(&self, other: &Point) -> f64 {
        ((self.x - other.x).powi(2) + (self.y - other.y).powi(2)).sqrt()
    }
}
```

- The struct becomes a JavaScript class with a constructor and methods.
- Fields are accessed via getters/setters, or the entire struct can be serialized/deserialized if passed by value.

### 2.4 Imported Host Functions

```
#[wasm_bindgen]
extern "C" {
    fn log(s: &str);
}
```

- Generates an import of a host function `log` that takes a pointer+length.  The macro wraps the call safely.

---

## 3. Memory Management

By default, all values are exchanged by copying or serializing.  For complex types, the crate uses a **linear arena** within the Wasm instance, managed by a bump allocator.  The host can request a summary of allocated blocks and free them when done.

The generated code ensures that:

- All allocated blocks are properly disposed, even in the presence of panics (via `defer` blocks in the generated wrappers).
- Linear Blaze types (like `Text`) are not leaked; they are consumed by the host or converted to raw bytes.

---

## 4. `JsValue` and Host Interaction

When targeting a JavaScript host (browser or Node.js), the crate uses the `blaze‑wasm` runtime’s ability to import and export JS functions via a `js` namespace.

### 4.1 `JsValue`

```
pub enum JsValue {
    Undefined,
    Null,
    Bool(bool),
    Number(f64),
    BigInt(i64),
    String(Text),
    Symbol(Text),
    Object(JsObject),
    Array(JsArray),
    TypedArray(JsTypedArray),
    Function(JsFunction),
    Promise(JsPromise),
}
```

- This enum can represent any value that can cross the JS/Wasm boundary.
- Conversions from `JsValue` to Blaze types are provided via `FromJsValue`/`IntoJsValue` traits.

### 4.2 `Closure`

```
pub struct Closure<F: FnOnce(...)> {
    closure: F,
    js_handle: JsValue,
}
```

- A linear wrapper that moves a Blaze closure into a JavaScript function.  When `Dispose` is called, the closure is released on the JS side.
- Used to pass callbacks to JS APIs (e.g., event listeners).

### 4.3 `Promise`

```
pub struct Promise<T> {
    inner: JsPromise,
}
```

- A typed wrapper around a JavaScript Promise.  Provides `await` support in Blaze async functions.
- `Promise::from_future` converts a Blaze `Future` into a JS Promise.
- `Promise::into_future` converts a JS Promise into a `Future<T>`.

---

## 5. `blaze wasm‑bindgen` Tool

This command‑line tool processes a Blaze crate and produces:

1. A `.wasm` binary (or multiple if targeting different features).
2. A JavaScript glue file (`module.js`) that imports the Wasm and wraps the exported functions into idiomatic JS.
3. A TypeScript declaration file (`module.d.ts`).

The tool is invoked as:

```
blaze wasm-bindgen --target web path/to/crate --out-dir pkg/
```

It uses the compiler’s `@comptime` reflection to inspect the crate’s public items and generate the necessary wrappers.

---

## 6. Error Handling

```
pub enum BindgenError {
    Compile(Text),
    UnsupportedType(Text),
    Serialization(Text),
    Deserialization(Text),
    HostError(Text),
}
```

- If a type cannot be automatically converted (e.g., a trait object), the macro produces a compile‑time error with a helpful message, suggesting manual implementation.

---

## 7. Implementation Notes

- The macro expansion occurs at compile time; no runtime reflection is used.
- The generated Wasm binary uses `wasm‑bindgen` ABI conventions (export `__wbindgen_malloc`, `__wbindgen_free`) to coordinate memory with the host.
- The crate is designed to work with any Wasm engine that supports the MVP, with optional SIMD and threads support via feature flags.
- For non‑JS hosts (e.g., a pure‑Wasm microservice), the tool can generate a C header file instead of JavaScript, using the same underlying ABI.

---

## 8. Testing

- **Export a function:** Compile a simple crate with `# [wasm_bindgen]` exporting an `add` function.  Run the generated Wasm in a browser/Node.js test harness and verify the result.
- **Export a struct:** Define a `Point` struct with methods, instantiate it from JavaScript, call methods, and check values.
- **Import a host function:** Provide a JS `log` function, call it from Blaze, and verify that the log message appears correctly.
- **Closure handling:** Pass a Blaze closure as a callback to a JS `setTimeout`, ensure it executes after the timeout.
- **Promise interop:** Call a JS async function that returns a Promise, await it in a Blaze async function, and verify the resolved value.
- **Memory safety:** Repeatedly call functions that allocate and free memory; monitor memory usage to ensure no leaks.
- All tests must pass in at least one Wasm environment (Node.js recommended for CI).
