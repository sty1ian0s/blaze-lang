# Blaze Phase 3c – Extended Library: Compile‑Time Reflection (`std::meta`)

> **Goal:** Implement the `std::meta` module exactly as specified.  This module provides the types and functions needed for compile‑time reflection: querying types, fields, variants, functions, and attributes.  It is used by `@comptime` macros and by the `quote!` macro.  All functions are `@comptime`‑only and cannot be called at runtime.

---

## 1. Reflection Types

### 1.1 `Type`

```
pub struct Type {
    // private, opaque; carries compile‑time type information
}
```

Represents a Blaze type (primitive, struct, enum, array, trait object, etc.).

- **Methods (comptime only):**

```
impl Type {
    pub fn name(&self) -> &'static str;
    pub fn size(&self) -> usize;
    pub fn align(&self) -> usize;
    pub fn is_primitive(&self) -> bool;
    pub fn is_struct(&self) -> bool;
    pub fn is_enum(&self) -> bool;
    pub fn is_union(&self) -> bool;
    pub fn is_tuple(&self) -> bool;
    pub fn is_array(&self) -> bool;
    pub fn is_slice(&self) -> bool;
    pub fn is_fn(&self) -> bool;
    pub fn is_trait_object(&self) -> bool;
    pub fn is_impl_trait(&self) -> bool;
    pub fn fields(&self) -> Option<Vec<Field>>;      // None if not a struct/tuple/union
    pub fn variants(&self) -> Option<Vec<Variant>>;   // None if not an enum
}
```

- `name()` returns the fully qualified name of the type (e.g., `"std::collections::List<i32>"`).  For generics, the concrete type arguments are included.
- `size()` and `align()` are the compile‑time constants.
- `fields()` returns a list of `Field` for struct/tuple/union types; for enums, `variants()` returns a list of variants.

### 1.2 `Field`

```
pub struct Field {
    pub name: &'static str,
    pub ty: Type,
    pub vis: Visibility,
    pub has_default: bool,
    // … other properties
}
```

Represents a field of a struct or union, or an element of a tuple.

- `name` – field name (for tuple structs, the field name is `"0"`, `"1"`, etc.).
- `ty` – type of the field.
- `vis` – visibility (`pub`, `pub(opaque)`, or private).
- `has_default` – `true` if the field has a default value expression.

### 1.3 `Variant`

```
pub struct Variant {
    pub name: &'static str,
    pub fields: Vec<Field>,   // empty for variant with no payload
    pub discriminant: Option<i64>,
    pub is_tuple: bool,       // true if variant payload is a tuple‑like
}
```

- Represents one variant of an enum.
- `discriminant` is present for variants with an explicit discriminant value.

### 1.4 `FnDef`

```
pub struct FnDef {
    pub name: &'static str,
    pub params: Vec<FnParam>,
    pub return_type: Type,
    pub effect_set: EffectSet,
    pub is_async: bool,
    // ... more metadata
}

pub struct FnParam {
    pub name: &'static str,
    pub ty: Type,
    pub has_default: bool,
}
```

- Represents a function signature.  Used to inspect functions at compile time (e.g., to generate bindings).

### 1.5 `EffectSet`

```
pub struct EffectSet { /* … */ }
```

Represents the effect set of a function (pure `∅` or a set of effect flags).  Provides methods like `contains(&self, effect: &str) -> bool`, `is_pure(&self) -> bool`, etc.

### 1.6 `Visibility`

```
pub enum Visibility {
    Private,
    Pub,
    PubOpaque,
}
```

### 1.7 `Attribute`

```
pub struct Attribute {
    pub name: &'static str,
    pub args: Vec<AttributeArg>,
}

pub struct AttributeArg {
    pub name: &'static str,
    pub value: Option<ExprTokenStream>,
}
```

- An `Attribute` represents a `@name(args…)` annotation on an item.

---

## 2. Core Reflection Functions

All of the following are `@comptime` only (they can only be called from `@comptime` macros or `quote!` contexts).

### 2.1 Type Inspection

```
pub fn type_of<T>() -> Type;
pub fn fields_of<T>() -> Vec<Field>;
pub fn name_of<T>() -> &'static str;
pub fn variants_of<T>() -> Vec<Variant>;
pub fn attributes_of(item: &Any) -> Vec<Attribute>;
```

- `type_of()` returns the `Type` of `T`.
- `fields_of()` returns the fields of `T` (if `T` is a struct/tuple/union).
- `name_of()` returns the fully qualified name of `T`.
- `variants_of()` returns the variants of an enum `T`.
- `attributes_of(item)` takes a reference to any compile‑time item (declaration) and returns the list of attributes.  The `&Any` parameter is a special type that the compiler provides; users cannot construct it, but it is given to `@comptime` macro implementations.

### 2.2 Token Stream Macros

```
pub fn quote(ts: TokenStream) -> Expr;
```

- The `quote!` macro produces a `TokenStream`, and `std::meta::quote` (or built‑in) turns it into an expression that can be spliced into generated code.  (Actually `quote!` is a compiler built‑in; `std::meta` just re‑exports the `TokenStream` and `quote` function.)

### 2.3 `TokenStream`

```
pub struct TokenStream { /* … */ }
```

- Represents a sequence of tokens.  Constructible via `quote!` or by parsing a string literal using `TokenStream::from_str(s) -> Result<…>`.  Splice `#ts` inside `quote!`.

### 2.4 `Expr`

```
pub struct Expr { /* … */ }
```

- Represents a compile‑time evaluated expression.  Not commonly used directly; the compiler unifies it with `TokenStream`.

---

## 3. Integration with @comptime

This module is the runtime library for the `@comptime` interpreter.  When a `@comptime` function is compiled, calls to `type_of`, `fields_of`, etc., are evaluated by the `@comptime` interpreter, which obtains the type information from the compiler’s internal symbol tables.

All the types (`Type`, `Field`, etc.) exist only at compile time and have no runtime representation.  Therefore, none of these types implement `Dispose` or `Clone`; they are implicitly `@copy` because they are small handles to compiler‑internal data.

---

## 4. Testing

Because these functions are compile‑time only, testing requires writing `@comptime` test functions that call the reflection API and assert properties, then running them via `blaze test` with a special mode that evaluates `@comptime` tests at compile time.  For Phase 3c, we can test that:

- `type_of::<i32>()` returns a `Type` whose `name` is `"i32"`.
- `fields_of::<SomeStruct>()` returns the expected list of fields with correct names and types.
- `attributes_of` returns the attributes placed on an item.
- `TokenStream::from_str` can parse a simple expression and `quote!` can produce a valid token stream.

All tests must pass before moving to the next module.
