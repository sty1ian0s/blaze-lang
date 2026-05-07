# Blaze Phase 2b – Generics, Traits, and Impls

> **Goal:** Extend the compiler to support generic functions, generic structs/enums, trait declarations, trait implementations, trait bounds, `dyn Trait` objects, `where` clauses, and the coherence rules.  The parser already accepts the full syntax; this phase adds semantic analysis and code generation for generics and traits.

---

## 1. Generic Parameters

### 1.1 Syntax (already parsed)
```
generic_params = "[" generic_param { "," generic_param } [ "," ] "]"
generic_param  = ident [ ":" bound ] | "effect" ident
bound          = path { "+" path }
```
- Effect parameters (e.g., `effect E`) are **not** implemented in this phase; they are parsed but produce a “not yet supported” error.  We focus on type generics.

### 1.2 Semantics
- A generic declaration introduces type parameters.  Each type parameter may have trait bounds.
- When a generic function is called, type arguments are either provided explicitly via turbofish (`func::<i32, bool>(x)`) or inferred from the argument types.
- Inference: if the type of a formal parameter is a type variable, we unify the actual argument type with that variable.  If unification fails, we report an error.
- If any type variable remains ambiguous after processing all arguments, the compiler emits an error requiring an explicit type argument.

### 1.3 Monomorphisation
- For each unique set of concrete type arguments, the compiler generates a separate copy of the function (or struct/enum definition).
- The generated code is identical to what the user would have written by hand for those concrete types.
- No runtime overhead: all generic dispatch is static.

---

## 2. Generic Structs and Enums

### 2.1 Declaration
```
struct Vec<T> { data: *mut T, len: usize, cap: usize }
enum Option<T> { Some(T), None }
```
- Type parameters are in scope for fields and variant payloads.
- When a generic struct/enum is instantiated with concrete types, monomorphisation creates a distinct type for each instantiation.

### 2.2 Implementation Considerations
- The compiler must handle generic field types, field access, and construction.
- For enums, the tag layout may depend on the number of variants; the tag size is the same for all instantiations of a generic enum.

---

## 3. Traits

### 3.1 Trait Declaration
```
trait_decl = "trait" ident [ generic_params ] [ ":" bound ] "{" { trait_item } "}"
trait_item = fn_decl_no_body | const_decl | type_decl
```
- A trait can declare associated functions (without body), associated constants, and associated types.
- A trait may have super‑traits (the `:` bound).  `T: Foo + Bar` means the trait requires both `Foo` and `Bar`.
- Default implementations for trait methods are **not** yet supported; they will be added later.

### 3.2 Trait Bounds
- A type parameter can be constrained: `T: Trait` or `T: Trait1 + Trait2`.
- Where clauses provide an alternative syntax:
  ```
  fn example<T>(x: T) where T: Clone + Debug { ... }
  ```
  They are syntactic sugar; the compiler adds the bounds to the respective parameter.

### 3.3 Impl Blocks
```
impl_decl = "impl" [ generic_params ] ( type "{" { impl_item } "}" | bound "for" type "{" { impl_item } "}" )
```
- Inherent impls: `impl Foo { fn ... }` – defines methods on a specific type.
- Trait impls: `impl Trait for Type { fn ... }` – provides implementations for the trait’s methods.

### 3.4 Method Resolution
- When calling a method `x.foo()`, the compiler searches for a method `foo` in the following order:
  1. Inherent methods of the type.
  2. Methods from traits implemented by the type.
- If multiple traits provide a method with the same name, the call is ambiguous unless the developer uses fully‑qualified syntax: `Trait::foo(x)`.

### 3.5 `dyn Trait` (Existential Types)
- `dyn Trait` represents a dynamically‑dispatched type that implements the trait.
- A value of type `dyn Trait` is a pair (pointer to data, pointer to vtable).
- The vtable includes function pointers for each trait method, plus a destructor if the type implements `Dispose`.
- Construction: any concrete type implementing the trait can be coerced to `dyn Trait` via explicit cast (`val as dyn Trait`) or implicitly when the context demands it.
- Restrictions: `dyn Trait` cannot be used for traits with associated types or `Self` in return position unless the object safety rules are satisfied (see below).

### 3.6 Object Safety
A trait is **object safe** if:
- All methods either take `self` by reference (`&self` or `&mut self`) or do not have a receiver.
- No method returns `Self` (except indirectly via `Box<Self>` if supported – but `Box` is not yet available).
- No associated constants or types are used in method signatures unless the trait object is bounded with explicit associated types (further spec later).

The compiler must reject `dyn Trait` for traits that violate object safety.

---

## 4. Coherence and Orphan Rules

### 4.1 Coherence
- For any pair (Trait, Type), there must be **at most one** implementation in the entire program.
- The compiler checks at link time (or when building the final binary) that no two crates provide conflicting impls.

### 4.2 Orphan Rules (per crate)
- An `impl Trait for Type` is allowed **only if**:
  - the trait is defined in the current crate, **or**
  - the type is defined in the current crate.
- This prevents two crates from simultaneously implementing a foreign trait for a foreign type.
- Violation causes a compile‑time error.

---

## 5. Code Generation

### 5.1 Monomorphisation
- The compiler generates a separate instantiation for each concrete set of type arguments.
- For each instantiation, the internal monomorphised name is something like `Vec_i32` (name mangling).

### 5.2 Vtables
- For each trait and each concrete type implementing it, the compiler generates a vtable structure.
- The vtable layout is: destructor pointer (if any), followed by function pointers in the order of the trait’s methods.

### 5.3 Dynamic Dispatch
- A `dyn Trait` value is represented as: `{ void* data; void* vtable; }`.
- Method calls are compiled to an indirect call through the vtable.

---

## 6. Testing

For each feature in this phase, the following tests must be written:

- **Generic functions:** instantiation with explicit turbofish, type inference, ambiguous inference error.
- **Generic structs/enums:** construction, field access, method calls.
- **Trait declarations and implementations:** inherent methods, trait methods, default method calls.
- **Trait bounds:** both inline bounds and where‑clause bounds; check that the compiler rejects calls where bounds are not met.
- **dyn Trait:** construction from concrete type, method dispatch, object safety rejection.
- **Coherence/Orphan:** conflicting impls in the same crate must be rejected; orphan rule violations must be caught.
- All tests must pass before Phase 2c begins.
