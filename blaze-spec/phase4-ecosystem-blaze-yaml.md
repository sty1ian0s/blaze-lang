# Phase 4 – Ecosystem Crate: `blaze‑yaml`

> **Goal:** Specify the `blaze‑yaml` crate, which provides a fast, zero‑copy YAML parser and writer that integrates with the `blaze‑serde` serialization framework.  It supports the full YAML 1.2 specification, including mappings, sequences, scalars, anchors, aliases, and tags.  The crate is built on Blaze’s data‑oriented philosophy: the parser uses a hand‑written state machine over a byte buffer, producing a tree of `YamlValue` nodes, and the writer emits YAML without intermediate representations.

---

## 1. YAML Value Type

### 1.1 `YamlValue`

```
pub enum YamlValue {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(Text),
    Bytes(Vec<u8>),
    Seq(Vec<YamlValue>),
    Map(Map<Text, YamlValue>),
    Alias(usize),         // index into an internal anchor table
    Tagged(Text, Box<YamlValue>),
    Anchor(usize, Box<YamlValue>),
}
```

- This is the generic representation of any YAML document.  It is linear (move semantics).  Cloning requires explicit `.clone()`.
- `Alias` refers to a previously encountered anchor by its index; anchors are stored in a `Vec<YamlValue>` during parsing and replaced when the document is fully resolved.
- `Tagged` holds a tag string (e.g., `"!my-type"`) and the associated value.

---

## 2. Parsing

### 2.1 `from_str`

```
pub fn from_str(source: &str) -> Result<YamlValue, YamlError>;
```

- Parses a complete YAML string into a `YamlValue`.  Supports multiple documents (returned as a single `Seq` of documents, unless there is only one document, in which case that document is returned directly—the caller can always use `match`).
- The parser handles:
  - Flow style (`{ … }`, `[ … ]`)
  - Block style (indentation‑sensitive mappings and sequences)
  - Anchors (`&label`) and aliases (`*label`)
  - Tags (`!tag-name`)
  - Literal block scalars (`|`, `|+`, `|-`)
  - Folded block scalars (`>`, `>+`, `>-`)
  - Multi‑line strings, escape sequences, comments
  - Implicit typing: integer, float, boolean, null detection based on YAML 1.2 Core Schema

### 2.2 `from_bytes`

```
pub fn from_bytes(bytes: &[u8]) -> Result<YamlValue, YamlError>;
```

- Same as `from_str` but accepts a UTF‑8 byte slice directly, avoiding a string conversion if the input is already in memory.

---

## 3. Writing

### 3.1 `to_string`

```
pub fn to_string(value: &YamlValue) -> Text;
```

- Serializes a `YamlValue` to a compact YAML string (flow style where possible, minimal whitespace).

### 3.2 `to_string_pretty`

```
pub fn to_string_pretty(value: &YamlValue) -> Text;
```

- Serializes with indentation (2 spaces) and block style for mappings and sequences, producing human‑readable YAML.

### 3.3 `to_writer`

```
pub fn to_writer<W: Write>(writer: &mut W, value: &YamlValue) -> Result<(), std::io::Error>;
pub fn to_writer_pretty<W: Write>(writer: &mut W, value: &YamlValue) -> Result<(), std::io::Error>;
```

- Writes directly to a `Write` stream without intermediate allocation.

---

## 4. Integration with `blaze‑serde`

The crate implements the `blaze‑serde` `Serializer` and `Deserializer` traits, enabling any type with `Serialize` / `Deserialize` to be read from or written to YAML.

### 4.1 `Serializer`

```
pub struct Serializer<W: Write> { /* … */ }
impl<W: Write> serde::Serializer for Serializer<W> { … }
```

- Created by `serde_yaml::Serializer::new(writer)`.

### 4.2 `Deserializer`

```
pub struct Deserializer<'de> { /* … */ }
impl<'de> serde::Deserializer<'de> for Deserializer<'de> { … }
```

- Created by `serde_yaml::Deserializer::from_str(s)` or `from_slice(bytes)`.

### 4.3 Convenience Functions

```
pub fn to_writer<W: Write, T: Serialize>(writer: W, value: &T) -> Result<()>;
pub fn to_string<T: Serialize>(value: &T) -> Result<Text>;
pub fn from_str<'a, T: Deserialize<'a>>(s: &'a str) -> Result<T>;
```

---

## 5. Error Handling

```
pub struct YamlError {
    msg: Text,
    line: usize,
    column: usize,
}

impl YamlError {
    pub fn line(&self) -> usize;
    pub fn column(&self) -> usize;
    pub fn message(&self) -> &str;
}
```

- Errors carry a precise location (1‑based) for syntax errors, anchor resolution failures, and unexpected node types.

---

## 6. Implementation Notes

- The parser uses an array‑based state machine to handle YAML’s context‑sensitive indentation.  Indentation levels are tracked as a stack of column numbers.
- Anchors and aliases are resolved in a second pass after the main tree is built.  The anchor table is stored alongside the document and aliases are replaced by a pointer to the anchored value, handling cycles (YAML allows circular references, but this crate does not support them—attempting to alias a value before its anchor is defined, or creating a cycle, will produce an error).
- The `YamlValue::Alias` variant is used during parsing; after resolution, aliases are replaced.  The public API does not expose aliases.
- Tags are preserved as strings; custom tag handling is delegated to `blaze‑serde`’s type‑based dispatch.

---

## 7. Testing

- **Valid YAML:** Parse YAML documents with various styles, verify the resulting `YamlValue` tree matches expectations.
- **Anchors and aliases:** Test anchor/alias pairs, ensuring referenced values are equal.
- **Scalar types:** Verify that `"42"` is parsed as `Int(42)`, `"3.14"` as `Float(3.14)`, `"true"` as `Bool(true)`, `"null"` as `Null`.
- **Error cases:** Provide malformed YAML (bad indentation, duplicate anchors, etc.) and verify error messages include line/column.
- **serde integration:** Derive `Serialize`/`Deserialize` for a struct, serialize to YAML, then deserialize and compare.
- **Round‑trip:** Ensure `from_str` followed by `to_string` produces canonical YAML that, when reparsed, yields an equal value.

All tests must pass before moving to the next crate.
