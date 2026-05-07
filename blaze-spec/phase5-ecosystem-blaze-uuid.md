# Phase 5 – Ecosystem Crate: `blaze‑uuid`

> **Goal:** Specify the `blaze‑uuid` crate, which provides a pure‑Blaze implementation of Universally Unique Identifiers (UUIDs) as defined in RFC 9562.  It supports versions 1 (time‑based), 3 (MD5 hash), 4 (random), 5 (SHA‑1 hash), 6 (reordered time), 7 (Unix time‑based), and 8 (custom).  The crate integrates with `blaze‑serde` for serialization, `std::time` for timestamps, `std::random` for random generation, and `blaze‑crypto` for MD5/SHA‑1 hashing.  All operations are pure except random generation (which depends on the system entropy source, `io` effect) and hashing (which is pure).  UUIDs are small, `@copy`, and can be used as keys in `Map` or as stable identifiers across distributed systems.

---

## 1. Core Type

### 1.1 `Uuid`

```
pub struct Uuid {
    bytes: [u8; 16],
}
```

- `Uuid` is `@copy` (16 bytes, no pointers).  It implements `PartialEq`, `Eq`, `Ord`, `Hash`, `Debug`, `Display`, `Default` (the nil UUID `00000000‑0000‑0000‑0000‑000000000000`).
- The internal representation is always the 16‑octet binary form (big‑endian order for the fields, but stored as raw bytes).  When displayed, it follows the standard 8‑4‑4‑4‑12 hexadecimal format.
- `Uuid` is `Serialize`/`Deserialize` (using the standard string representation for human‑readable formats, or raw bytes for binary formats).

---

## 2. Versions

The `Uuid` struct contains methods to create new UUIDs and to query the version and variant of an existing UUID.

### 2.1 Creation Methods

```
impl Uuid {
    pub fn nil() -> Uuid;
    pub fn v4() -> Uuid;            // random (version 4)
    pub fn v7() -> Uuid;            // time‑based (version 7, Unix epoch in milliseconds)
    pub fn v8(bytes: [u8; 16]) -> Uuid;   // custom (version 8, user‑provided bytes)
    pub fn from_bytes(bytes: [u8; 16]) -> Uuid;
    pub fn from_str(s: &str) -> Result<Uuid, UuidError>;
    pub fn from_slice(b: &[u8]) -> Result<Uuid, UuidError>;
}
```

- `v4` fills 122 random bits, sets version nibble to `4`, and variant to `10xx`.
- `v7` uses the current Unix timestamp in milliseconds (48 bits) as the first part, fills the remaining 74 bits with random data, sets version to `7` and variant appropriately.  If the system clock is not available (e.g., `no_std` without time), `v7` may return an error; but for Blaze’s standard library, `SystemTime::now()` is available (with `io` effect).  The `v7` function carries the `io` effect (because of the time query and random).
- `v8` takes arbitrary 16 bytes, sets version to `8` and variant to `10xx`.  This is useful for custom schemes.
- Additional constructors requiring hashing (`v3`, `v5`) are feature‑gated behind `hashing` because they depend on `blaze‑crypto` for MD5/SHA‑1.  They are provided if the feature is enabled:
  - `pub fn v3(namespace: &Uuid, name: &str) -> Uuid;`  (MD5)
  - `pub fn v5(namespace: &Uuid, name: &str) -> Uuid;`  (SHA‑1)

### 2.2 Query Methods

```
impl Uuid {
    pub fn as_bytes(&self) -> &[u8; 16];
    pub fn to_bytes(self) -> [u8; 16];
    pub fn variant(&self) -> Variant;
    pub fn version(&self) -> Option<Version>;
    pub fn is_nil(&self) -> bool;
    pub fn to_string(&self) -> Text;
}
```

- `variant()` returns the RFC 9562 variant (currently always `Rfc4122`).  `version()` returns the version if it is a standard UUID (1–8), otherwise `None` (for custom UUIDs that don’t set the version nibble properly).

### 2.3 `Version` and `Variant` Enums

```
pub enum Version {
    V1, V2, V3, V4, V5, V6, V7, V8,
}

pub enum Variant {
    Ncs,          // 0xxx (reserved, NCS backward compatibility)
    Rfc4122,      // 10xx (the standard variant)
    Microsoft,    // 110x (reserved, Microsoft Corporation backward compatibility)
    Future,       // 111x (reserved for future definition)
}
```

---

## 3. Serialization

The `Uuid` type implements `blaze::serde::Serialize` and `Deserialize`.  For human‑readable formats (JSON, YAML, TOML), it serializes as a string of the standard 36‑character representation.  For binary formats, it serializes as a raw 16‑byte array.  This is handled automatically by `blaze‑serde`’s derive, but the implementation is provided by the crate so that `#[derive(Serialize, Deserialize)]` on a struct containing a `Uuid` works correctly.

---

## 4. Integration with `std::collections`

Because `Uuid` implements `Eq`, `Ord`, and `Hash`, it can be used as a key in `Map` (B‑tree) and can be stored in slot maps.  Its small size and copy semantics make it ideal as an identifier.

---

## 5. Error Handling

```
pub struct UuidError {
    kind: UuidErrorKind,
    msg: Text,
}

pub enum UuidErrorKind {
    InvalidLength,
    InvalidCharacter,
    InvalidVersion,
    InvalidVariant,
    TimeUnavailable,
    HashError,
}
```

- `from_str` and `from_slice` may return errors if the string or bytes are not a valid UUID representation.

---

## 6. Testing

- **Format round‑trip:** Generate a UUIDv4, convert to string, parse back, and verify equality.
- **Version bits:** Generate UUIDv4, check `version() == Some(Version::V4)`.
- **Nil UUID:** `Uuid::nil().is_nil()` is `true`.
- **Variant:** All generated UUIDs must have variant `Rfc4122`.
- **v7 ordering:** Generate two UUIDv7 quickly, verify the first is less than the second (timestamp ordering).
- **Serialization:** Serialize a UUID to JSON, deserialize, compare.
- **Invalid strings:** Provide malformed strings and verify errors.

All tests must pass on all platforms.
