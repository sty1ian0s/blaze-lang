# Phase 4 – Ecosystem Crate: `blaze‑toml`

> **Goal:** Specify the `blaze‑toml` crate, which provides a fast, zero‑copy TOML parser and writer that integrates with the `blaze‑serde` serialization framework. It supports the full TOML v1.0 specification, including all data types (strings, integers, floats, booleans, offset‑datetimes, local‑datetimes, local‑dates, local‑times, arrays, tables, inline tables, arrays of tables). The parser is a recursive‑descent, byte‑oriented state machine, and the writer emits TOML conforming to the standard.

---

## 1. TOML Value Type

### 1.1 `TomlValue`

```
pub enum TomlValue {
    String(Text),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    Datetime(TomlDatetime),
    Array(Vec<TomlValue>),
    Table(Map<Text, TomlValue>),
}
```

- Represents any TOML value. Linear (move semantics). Not `@copy` (contains heap data).
- `TomlDatetime` is a newtype over `Text` that preserves the original string representation to allow round‑tripping without losing precision or timezone information. The crate also provides conversion to/from `std::time::SystemTime` for convenience.

### 1.2 `TomlDatetime`

```
pub struct TomlDatetime(Text);
impl TomlDatetime {
    pub fn to_string(&self) -> &str;
    pub fn to_system_time(&self) -> Result<SystemTime, TomlError>;
}
```

- TOML supports four datetime formats: offset‑datetime, local‑datetime, local‑date, local‑time. All are stored as a string; conversion to a specific type is attempted by `to_system_time` or dedicated methods `as_offset_datetime`, `as_local_date`, etc.

---

## 2. Parsing

### 2.1 `from_str`

```
pub fn from_str(source: &str) -> Result<TomlValue, TomlError>;
```

- Parses a complete TOML document (UTF‑8) into a `TomlValue::Table` representing the root table.
- Handles:
  - Keys: bare keys, basic strings, literal strings.
  - Values: strings, integers (decimal, hex, octal, binary, underscores), floats (including special values `inf`, `nan`), booleans, datetimes.
  - Tables: `[table]` and dotted keys `[a.b.c]`.
  - Inline tables: `{key = value, …}`.
  - Arrays: `[ … ]` and arrays of tables `[[array]]`.
  - Comments: `#` to end of line.
- Duplicate keys cause an error. The parser maintains a stack of tables to handle arrays of tables correctly.

### 2.2 `from_bytes`

```
pub fn from_bytes(bytes: &[u8]) -> Result<TomlValue, TomlError>;
```

- Same as `from_str` but accepts a byte slice, which must be valid UTF‑8.

---

## 3. Writing

### 3.1 `to_string`

```
pub fn to_string(value: &TomlValue) -> Text;
```

- Serializes a `TomlValue` to a compact TOML string (minimal whitespace, but compliant). If the root is not a `Table`, it is wrapped in a table.

### 3.2 `to_string_pretty`

```
pub fn to_string_pretty(value: &TomlValue) -> Text;
```

- Pretty‑prints with newlines, indentation, and blank lines between tables.

### 3.3 `to_writer`

```
pub fn to_writer<W: Write>(writer: &mut W, value: &TomlValue) -> Result<(), std::io::Error>;
pub fn to_writer_pretty<W: Write>(writer: &mut W, value: &TomlValue) -> Result<(), std::io::Error>;
```

- Writes directly to a `Write` stream.

---

## 4. Integration with `blaze‑serde`

The crate implements the `blaze‑serde` `Serializer` and `Deserializer` traits, enabling any type with `Serialize` / `Deserialize` to be read from or written to TOML.

### 4.1 `Serializer`

```
pub struct Serializer<W: Write> { /* … */ }
impl<W: Write> serde::Serializer for Serializer<W> { … }
```

- Created by `serde_toml::Serializer::new(writer)`.

### 4.2 `Deserializer`

```
pub struct Deserializer<'de> { /* … */ }
impl<'de> serde::Deserializer<'de> for Deserializer<'de> { … }
```

- Created by `serde_toml::Deserializer::from_str(s)` or `from_slice(bytes)`.

### 4.3 Convenience Functions

```
pub fn to_writer<W: Write, T: Serialize>(writer: W, value: &T) -> Result<()>;
pub fn to_string<T: Serialize>(value: &T) -> Result<Text>;
pub fn from_str<'a, T: Deserialize<'a>>(s: &'a str) -> Result<T>;
```

---

## 5. Error Handling

```
pub struct TomlError {
    msg: Text,
    line: usize,
    column: usize,
}

impl TomlError {
    pub fn line(&self) -> usize;
    pub fn column(&self) -> usize;
    pub fn message(&self) -> &str;
}
```

- Errors carry the line and column (1‑based) of the error occurrence for easy debugging. Examples of errors: duplicate key, missing equals sign, unterminated string, invalid number, unexpected token.

---

## 6. Implementation Notes

- The parser is a hand‑written recursive‑descent that reads a byte slice and maintains a context stack. It does not allocate until it builds the `TomlValue` tree; strings and identifiers may borrow from the original input for zero‑copy where possible, but for simplicity, all leaf values are owned (explicitly allocated in arena). The spec says “zero‑copy” but for TOML’s escaping rules, owned strings are always required.
- Arrays of tables (`[[array]]`) are parsed by appending to a `Vec<TomlValue>` associated with the key. The parser internally maintains a mapping of current table paths.
- Integer parsing uses `i64`; if a value exceeds `i64::MAX` or `i64::MIN`, it is stored as an error. TOML’s integer range is signed 64‑bit.
- Floats are parsed as `f64`; special values `inf`, `+inf`, `-inf`, `nan`, `+nan`, `-nan` are supported and stored as the corresponding IEEE bit patterns.
- Datetimes are preserved as strings to handle the various TOML formats; conversion to `SystemTime` is provided via a feature flag (default) using the OS’s timezone database.

---

## 7. Testing

- **Valid TOML:** Parse a TOML document that exercises all data types, table types, inline tables, arrays of tables, and verify the resulting `TomlValue` tree.
- **Round‑trip:** Parse TOML, serialize to string, re‑parse, and compare the original and final `TomlValue` (ignoring whitespace and comment differences).
- **Invalid TOML:** Provide documents with duplicate keys, missing delimiters, unterminated strings, invalid numbers; verify error messages and locations.
- **Datetime round‑trip:** Parse a datetime string, serialize back, and verify the string is unchanged.
- **serde integration:** Derive `Serialize`/`Deserialize` for a struct with various field types, serialize to TOML, deserialize, and compare.
- **Edge cases:** Empty document, empty table, empty array, keys with dots and quotation marks.

All tests must pass before moving to the next crate.
