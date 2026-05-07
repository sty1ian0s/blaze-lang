# Phase 4 – Ecosystem Crate: `blaze‑backtrace`

> **Goal:** Provide a pure, zero‑cost backtrace capture and symbolication library for Blaze.  It captures stack traces at runtime with minimal overhead, symbolises them against the binary’s debug information (or external DWARF), and formats them for logging or crash reporting.  The crate is designed for use in `blaze‑log`, `blaze‑debug`, and any application that needs to diagnose failures.  All capture operations are pure; symbolication may carry the `io` effect if debug information is loaded from external files.

---

## 1. Core Types

### 1.1 `Backtrace`

```
pub struct Backtrace {
    frames: Vec<Frame>,
    thread_name: Option<Text>,
}
```

- Represents a captured stack trace, owning a linear vector of frames.
- `thread_name` is populated from the actor name or thread identifier when available.

### 1.2 `Frame`

```
pub struct Frame {
    pub instruction_pointer: usize,
    pub symbol_name: Option<Text>,
    pub file: Option<Text>,
    pub line: Option<u32>,
    pub column: Option<u32>,
}
```

- `instruction_pointer` is the raw program counter at the time of capture.
- The remaining fields are populated after symbolication.

---

## 2. Capture

### 2.1 `Backtrace::capture`

```
impl Backtrace {
    pub fn capture() -> Backtrace;
    pub fn capture_skip(skip: usize) -> Backtrace;
}
```

- `capture` captures the current stack, excluding the backtrace functions themselves.
- `capture_skip` skips additional frames (e.g., for library wrappers).

Stack unwinding is performed by the compiler‑generated unwinding tables (`.eh_frame` on Unix, `PDB`/`PE` on Windows).  Internally, the crate uses `std::hardware` to identify the target architecture and calls the appropriate unwinding intrinsics.  Capture is pure (no allocation except the final `Vec<Frame>`).

---

## 3. Symbolication

### 3.1 `Backtrace::symbolicate`

```
impl Backtrace {
    pub fn symbolicate(&mut self) -> Result<(), BacktraceError>;
    pub fn symbolicated(self) -> Backtrace;
}
```

- Resolves raw instruction pointers to function names, source files, and line numbers.
- Uses DWARF debug information embedded in the binary (`blaze debug` section), a separate `.blzdbg` file, or platform‑specific debug APIs (`dladdr`, `SymFromAddr`, etc.).
- `symbolicated` returns a new `Backtrace` with all frames symbolicated.

### 3.2 `set_debug_path`

```
pub fn set_debug_path(path: &str);
```

- Sets a global override path for external debug information, useful when debugging stripped binaries.

---

## 4. Formatting

```
impl Backtrace {
    pub fn to_string(&self) -> Text;
    pub fn to_string_pretty(&self) -> Text;
}
```

- `to_string` returns a compact representation (one line per frame: `ip symbol [file:line]`).
- `to_string_pretty` returns a multi‑line format suitable for crash reports.

The `Display` trait is also implemented.

---

## 5. Integration with `blaze‑log` and `blaze‑panic`

The crate provides a `panic_handler` attribute:

```
#[backtrace::panic_handler]
fn my_panic_handler(info: &PanicInfo) -> ! {
    let bt = Backtrace::capture().symbolicated();
    eprintln!("{}", bt);
    std::process::abort();
}
```

This can be used as a drop‑in replacement for the default panic handler to automatically print a backtrace on panic.

---

## 6. Error Handling

```
pub enum BacktraceError {
    Io(std::io::Error),
    NoDebugInfo,
    UnsupportedPlatform,
    SymbolNotFound,
}
```

---

## 7. Implementation Notes

- On x86‑64 and aarch64, unwinding uses the native DWARF CFI (Call Frame Information) tables.  The compiler emits these tables into the `.blzdbg` section (or `.debug_frame` in ELF).
- On Windows, the crate uses the Win32 `StackWalk64` API and PDB symbol resolution when available.
- Symbolication is lazy; `Backtrace::capture` does not resolve symbols immediately.  The user must call `symbolicate` or use `symbolicated`.
- In `no_std` environments, capture is still possible (raw instruction pointers only), but symbolication requires an external debug file.

---

## 8. Testing

- **Capture:** Capture a backtrace from a known function, verify the number of frames is non‑zero.
- **Symbolication:** Capture a backtrace in a test function, symbolicate, verify the test function’s name appears.
- **Pretty formatting:** Capture, symbolicate, format, and check that the output contains expected strings.
- **No debug info:** Strip debug info, symbolicate, verify `symbol_name` is `None` but no crash.
- **Thread safety:** Capture backtraces from multiple threads concurrently, verify no data races.

All tests must pass on all platforms.
