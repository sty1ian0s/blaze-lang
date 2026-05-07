# Phase 4 – Ecosystem Crate: `blaze‑debug‑info`

> **Goal:** Provide a pure, data‑oriented library for reading and writing debug information in the Blaze Debug Format (BDF), a superset of DWARF.  It is used by the compiler to emit `.blzdbg` files, by `blaze‑debug` for interactive debugging, and by `blaze‑backtrace` for symbolication.  All parsing is pure; writing to files carries the `io` effect.

---

## 1. Core Types

### 1.1 `DebugInfo`

```
pub struct DebugInfo {
    pub compile_units: Vec<CompileUnit>,
    pub line_program: LineProgram,
    pub address_ranges: Vec<AddressRange>,
    pub strings: StringTable,
}
```

- Represents a complete debug information file, owning all its data linearly.

### 1.2 `CompileUnit`

```
pub struct CompileUnit {
    pub name: Text,
    pub language: Language,
    pub functions: Vec<FunctionDI>,
    pub variables: Vec<VariableDI>,
    pub types: Vec<TypeDI>,
}
```

- A compile unit corresponds to one source file or module.

### 1.3 `LineProgram`

```
pub struct LineProgram {
    pub entries: Vec<LineEntry>,
}

pub struct LineEntry {
    pub address: u64,
    pub file: u32,   // index into string table
    pub line: u32,
    pub column: u32,
}
```

- Maps instruction addresses to source locations.

---

## 2. Parsing

```
pub fn read_from_bytes(data: &[u8]) -> Result<DebugInfo, DebugInfoError>;
pub fn read_from_file(path: &str) -> Result<DebugInfo, DebugInfoError>;  // carries `io`
```

- Parses a `.blzdbg` file (or raw DWARF) into a `DebugInfo` struct.

---

## 3. Writing

```
pub fn write_to_bytes(info: &DebugInfo) -> Result<Vec<u8>, DebugInfoError>;
pub fn write_to_file(info: &DebugInfo, path: &str) -> Result<(), DebugInfoError>;
```

- Serializes the debug information into the BDF binary format.

---

## 4. Lookup Functions

```
impl DebugInfo {
    pub fn find_function(&self, address: u64) -> Option<&FunctionDI>;
    pub fn find_location(&self, address: u64) -> Option<(u32, u32, u32)>; // file, line, col
    pub fn find_type(&self, name: &str) -> Option<&TypeDI>;
}
```

- Efficient lookup using sorted address tables (binary search).

---

## 5. Integration with `blaze‑backtrace` and `blaze‑debug`

- `blaze‑backtrace` uses `DebugInfo::find_function` and `find_location` to symbolicate frames.
- `blaze‑debug` uses the full `DebugInfo` to provide variable inspection and a source‑level debugger.

---

## 6. Error Handling

```
pub enum DebugInfoError {
    Io(std::io::Error),
    InvalidFormat,
    UnsupportedVersion,
    CorruptedData,
}
```

---

## 7. Testing

- **Round‑trip:** Create a small debug info structure, write to bytes, read back, verify equality.
- **Lookup:** Build a debug info with known addresses, call `find_location`, verify correct file/line/col.
- **DWARF compatibility:** Parse a standard DWARF5 file (from a C compiler) and verify that functions are found.

All tests must pass on all platforms.
