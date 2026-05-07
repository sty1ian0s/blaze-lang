# Phase 4 – Ecosystem Crate: `blaze‑json`

> **Goal:** Specify the `blaze‑json` crate, a dependency of `blaze‑serde`.  This crate provides a fast, zero‑copy JSON parser and writer that directly integrates with the `Serialize` and `Deserialize` traits.  It is not required for core conformance but is the standard JSON implementation for Blaze.

---

## 1. JSON Value Type

### 1.1 `JsonValue` Enum

```
pub enum JsonValue {
    Null,
    Bool(bool),
    Number(JsonNumber),
    String(Text),
    Array(Vec<JsonValue>),
    Object(Map<Text, JsonValue>),
}
```

- Represents any JSON value.  It is a linear type; cloning requires explicit `.clone()`.
- The `JsonNumber` type wraps an `f64` but also stores the original string representation to avoid loss of precision for integers that exceed `f64` range.

### 1.2 `JsonNumber`

```
pub struct JsonNumber {
    value: f64,
    raw: Text,       // original string as parsed
}
impl JsonNumber {
    pub fn to_i64(&self) -> Option<i64>;
    pub fn to_u64(&self) -> Option<u64>;
    pub fn to_f64(&self) -> f64;
    pub fn to_string(&self) -> &str;
}
```

---

## 2. Parsing

### 2.1 `from_str`

```
pub fn from_str(s: &str) -> Result<JsonValue, Error>;
```

- Parses a JSON string into a `JsonValue`.  The parser is recursive descent and strictly conforms to RFC 8259.
- Supports UTF‑8, Unicode escapes (`\uXXXX`), and surrogate pairs.
- Returns an `Error` with line and column information on syntax error.

### 2.2 `from_bytes`

```
pub fn from_bytes(bytes: &[u8]) -> Result<JsonValue, Error>;
```

- Same as `from_str`, but accepts a byte slice.  The bytes must be valid UTF‑8; if not, returns an error.

### 2.3 `from_reader`

```
pub fn from_reader<R: Read>(reader: R) -> Result<JsonValue, Error>;
```

- Reads the entire stream into a `Text` and parses it.  May be less efficient for large files due to intermediate allocation.

---

## 3. Writing

### 3.1 `to_string`

```
pub fn to_string(value: &JsonValue) -> Text;
```

- Serializes a `JsonValue` to a compact JSON string.  No extra whitespace is added.

### 3.2 `to_string_pretty`

```
pub fn to_string_pretty(value: &JsonValue) -> Text;
```

- Serializes with indentation (2 spaces) and newlines for human readability.

### 3.3 `to_writer`

```
pub fn to_writer<W: Write>(writer: &mut W, value: &JsonValue) -> Result<(), io::Error>;
pub fn to_writer_pretty<W: Write>(writer: &mut W, value: &JsonValue) -> Result<(), io::Error>;
```

- Writes directly to a `Write` stream without intermediate string allocation.

---

## 4. Integration with `blaze‑serde`

The `blaze‑json` crate implements the `Serializer` and `Deserializer` traits from `blaze‑serde` so that any type implementing `Serialize`/`Deserialize` can be automatically converted to/from JSON.

### 4.1 `serde_json::Serializer`

```
pub struct Serializer<W: Write> { /* … */ }
impl<W: Write> serde::Serializer for Serializer<W> { … }
```

- Created by `serde_json::Serializer::new(writer)`.

### 4.2 `serde_json::Deserializer`

```
pub struct Deserializer<'de> { /* … */ }
impl<'de> serde::Deserializer<'de> for Deserializer<'de> { … }
```

- Created by `serde_json::Deserializer::from_str(s)` or `from_slice(bytes)`.

### 4.3 Convenience Functions

```
pub fn to_writer<W: Write, T: Serialize>(writer: W, value: &T) -> Result<()>;
pub fn to_string<T: Serialize>(value: &T) -> Result<Text>;
pub fn from_str<'a, T: Deserialize<'a>>(s: &'a str) -> Result<T>;
```

These are re‑exported by `blaze‑serde` for user convenience.

---

## 5. Error Handling

```
pub struct Error {
    msg: Text,
    line: usize,
    column: usize,
}

impl Error {
    pub fn line(&self) -> usize;
    pub fn column(&self) -> usize;
    pub fn message(&self) -> &str;
}
```

Errors carry a precise location (1‑based) for syntax errors.

---

## 6. Implementation Notes

- The parser uses a byte‑level state machine to avoid allocation for static strings when possible.  For string values that need unescaping, a new `Text` is allocated.
- The `JsonNumber` type preserves the original string so that very large integers (beyond `f64` precision) can be extracted as `i64` if the user expects that.  The trade‑off is that `JsonNumber` is larger than a bare `f64`.
- The `to_string` and `to_writer` functions write JSON in streaming fashion, never buffering the entire output.

---

## 7. Testing

- **Valid JSON:** Parse a variety of JSON objects, arrays, strings, numbers, booleans, null.  Verify round‑tripping through `to_string` → `from_str` preserves logical equality (though string escapes may change).
- **Invalid JSON:** Provide malformed inputs (mismatched braces, unquoted strings, trailing commas) and verify error messages include correct line/column.
- **Deeply nested structures:** Ensure no stack overflow for maximum nesting depth (the parser uses recursion; it should handle at least 100 levels of nesting for arrays/objects).
- **serde integration:** Derive `Serialize` for a struct, serialize to JSON using `to_string`, then deserialize and compare.
