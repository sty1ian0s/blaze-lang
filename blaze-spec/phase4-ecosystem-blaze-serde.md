# Phase 4 – Ecosystem Crate: `blaze‑serde`

> **Goal:** Specify the `blaze‑serde` crate, the first of the optional ecosystem crates.  This crate provides a data‑oriented serialization and deserialization framework for converting Blaze data structures to and from binary and text formats.  It is not required for core conformance but is strongly recommended for any application that needs to exchange data.

---

## 1. Core Traits

### 1.1 `Serialize`

```
pub trait Serialize {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error>;
}
```

Types that implement `Serialize` can be turned into a stream of bytes or tokens by a `Serializer`.  The `serialize` method accepts a serializer and delegates to its methods, which are called in a specific order to produce a valid data format.

### 1.2 `Deserialize`

```
pub trait Deserialize<'de>: Sized {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error>;
}
```

Types that implement `Deserialize` can be reconstructed from a deserializer, which in turn reads from a byte stream or token source.  The `'de` lifetime parameter ties the deserialized value to the input data, enabling zero‑copy deserialization for types like `&str` and `&[u8]`.

---

## 2. Serializer and Deserializer Traits

### 2.1 `Serializer`

```
pub trait Serializer: Sized {
    type Ok;
    type Error: serde::Error;

    // Required methods
    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error>;
    fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error>;
    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error>;
    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error>;
    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error>;
    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error>;
    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error>;
    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error>;
    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error>;
    fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error>;
    fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error>;
    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error>;
    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error>;
    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error>;
    fn serialize_none(self) -> Result<Self::Ok, Self::Error>;
    fn serialize_some<T: ?Sized + Serialize>(self, value: &T) -> Result<Self::Ok, Self::Error>;
    fn serialize_unit(self) -> Result<Self::Ok, Self::Error>;
    fn serialize_unit_struct(self, name: &'static str) -> Result<Self::Ok, Self::Error>;
    fn serialize_unit_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok, Self::Error>;
    fn serialize_newtype_struct<T: ?Sized + Serialize>(
        self,
        name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>;
    fn serialize_newtype_variant<T: ?Sized + Serialize>(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>;
    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error>;
    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error>;
    fn serialize_tuple_struct(
        self,
        name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error>;
    fn serialize_tuple_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error>;
    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap, Self::Error>;
    fn serialize_struct(
        self,
        name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error>;
    fn serialize_struct_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error>;
}
```

The methods return intermediate state objects (e.g., `SerializeSeq`) that allow the serializer to emit a sequence incrementally.  These are documented in the Serializer’s output format (see Section 3).

### 2.2 `Deserializer`

```
pub trait Deserializer<'de>: Sized {
    type Error: serde::Error;

    fn deserialize_bool<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error>;
    fn deserialize_i8<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error>;
    fn deserialize_i16<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error>;
    fn deserialize_i32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error>;
    fn deserialize_i64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error>;
    fn deserialize_u8<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error>;
    fn deserialize_u16<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error>;
    fn deserialize_u32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error>;
    fn deserialize_u64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error>;
    fn deserialize_f32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error>;
    fn deserialize_f64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error>;
    fn deserialize_char<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error>;
    fn deserialize_str<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error>;
    fn deserialize_bytes<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error>;
    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error>;
    fn deserialize_unit<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error>;
    fn deserialize_unit_struct<V: Visitor<'de>>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>;
    fn deserialize_newtype_struct<V: Visitor<'de>>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>;
    fn deserialize_seq<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error>;
    fn deserialize_tuple<V: Visitor<'de>>(self, len: usize, visitor: V) -> Result<V::Value, Self::Error>;
    fn deserialize_tuple_struct<V: Visitor<'de>>(
        self,
        name: &'static str,
        len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error>;
    fn deserialize_map<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error>;
    fn deserialize_struct<V: Visitor<'de>>(
        self,
        name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>;
    fn deserialize_enum<V: Visitor<'de>>(
        self,
        name: &'static str,
        variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>;
    fn deserialize_identifier<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error>;
    fn deserialize_ignored_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error>;
}
```

The deserializer calls the visitor’s methods, which mirror the serializer’s methods, allowing one‑pass, zero‑copy parsing.

### 2.3 `Visitor`

```
pub trait Visitor<'de>: Sized {
    type Value;

    fn expecting(&self, formatter: &mut Formatter) -> fmt::Result;
    fn visit_bool(self, v: bool) -> Result<Self::Value, Error>;
    fn visit_i8(self, v: i8) -> Result<Self::Value, Error>;
    fn visit_i16(self, v: i16) -> Result<Self::Value, Error>;
    fn visit_i32(self, v: i32) -> Result<Self::Value, Error>;
    fn visit_i64(self, v: i64) -> Result<Self::Value, Error>;
    fn visit_u8(self, v: u8) -> Result<Self::Value, Error>;
    fn visit_u16(self, v: u16) -> Result<Self::Value, Error>;
    fn visit_u32(self, v: u32) -> Result<Self::Value, Error>;
    fn visit_u64(self, v: u64) -> Result<Self::Value, Error>;
    fn visit_f32(self, v: f32) -> Result<Self::Value, Error>;
    fn visit_f64(self, v: f64) -> Result<Self::Value, Error>;
    fn visit_char(self, v: char) -> Result<Self::Value, Error>;
    fn visit_str(self, v: &str) -> Result<Self::Value, Error>;
    fn visit_borrowed_str(self, v: &'de str) -> Result<Self::Value, Error>;
    fn visit_bytes(self, v: &[u8]) -> Result<Self::Value, Error>;
    fn visit_borrowed_bytes(self, v: &'de [u8]) -> Result<Self::Value, Error>;
    fn visit_none(self) -> Result<Self::Value, Error>;
    fn visit_some<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, Error>;
    fn visit_unit(self) -> Result<Self::Value, Error>;
    fn visit_newtype_struct<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, Error>;
    fn visit_seq<A: SeqAccess<'de>>(self, seq: A) -> Result<Self::Value, Error>;
    fn visit_map<A: MapAccess<'de>>(self, map: A) -> Result<Self::Value, Error>;
    fn visit_enum<A: EnumAccess<'de>>(self, data: A) -> Result<Self::Value, Error>;
}
```

---

## 3. Format‑specific Serializers and Deserializers

The crate includes built‑in support for two formats: `blaze‑json` and `blaze‑binary`.  Each is implemented as a separate module providing a `Serializer` and `Deserializer`.

### 3.1 `serde_json::Serializer`

Writes JSON text to a `std::io::Write` output.  Implemented in the `blaze‑json` crate (a dependency of `blaze‑serde`), re‑exported here.

- `fn to_writer<W: Write, T: Serialize>(writer: W, value: &T) -> Result<()>`
- `fn to_string<T: Serialize>(value: &T) -> Result<Text>`

### 3.2 `serde_json::Deserializer`

Reads JSON text from a byte slice or reader, constructing a token stream and calling the visitor.

- `fn from_str<'a, T: Deserialize<'a>>(s: &'a str) -> Result<T>`
- `fn from_reader<R: Read, T: Deserialize<'de>>(reader: R) -> Result<T>` (with `'de` tied to the function scope, requiring allocation)

### 3.3 `serde_binary::Serializer` and `Deserializer`

A compact binary format (similar to BSON but more efficient) that uses variable‑length integers and length‑prefixed strings.  The binary format is not human‑readable but avoids any escaping overhead.

---

## 4. Derive Macros

The crate provides `@derive(Serialize, Deserialize)` via the comptime macro system.  When applied to a struct or enum, the compiler generates an implementation of `Serialize` and `Deserialize` that processes each field/variant in order.

Example:

```
#[derive(Serialize, Deserialize)]
struct Point {
    x: f64,
    y: f64,
}
```

The generated code:

- For `Serialize`: creates a struct serializer, writes the struct name, then serializes each field by calling `serialize_field` with the field name and its value.
- For `Deserialize`: creates a struct visitor, reads each field by name, deserializes it, and returns the struct.

---

## 5. Error Handling

```
pub struct Error {
    message: Text,
    line: Option<usize>,
    column: Option<usize>,
}

impl Error {
    pub fn new(msg: &str) -> Self;
    pub fn with_location(mut self, line: usize, col: usize) -> Self;
}
```

Errors carry a descriptive message and optional location information for text‑based formats.

---

## 6. Testing

- **Round‑trip:** Serialize a variety of values (primitives, structs, enums) to JSON and binary, then deserialize back and verify equality.
- **Edge cases:** Test large integers, floating‑point special values, empty strings, maps, deeply nested structures.
- **Error handling:** Provide malformed JSON and ensure deserialization returns an `Error` with a useful message.
- **Derive correctness:** Define structs with `@derive(Serialize, Deserialize)` and verify that the generated code works without manual implementations.
