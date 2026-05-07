# Appendix D – Coherence and Orphan Rules

> **Status:** Normative.  This appendix specifies the rules that guarantee at most one implementation of a trait for a given type exists in any compiled Blaze program.  These rules prevent conflicts and ensure predictable semantics.

---

## D.1 Coherence Rule

For any pair `(Trait, Type)`, there must be **at most one** `impl Trait for Type` block in the entire linked program.

The compiler enforces this rule during the link phase (when building the final binary).  If two crates provide conflicting implementations, the compiler emits a coherence error message naming both crates and the conflicting trait/type.

## D.2 Orphan Rule (Per Crate)

An `impl Trait for Type` is allowed **only if** at least one of the following is true:

1. The trait `Trait` is defined in the current crate.
2. The type `Type` is defined in the current crate.

This prevents two external crates from independently implementing a foreign trait for a foreign type, which would result in a conflict when those crates are used together.

## D.3 Blanket Implementations

The orphan rule applies to blanket implementations (`impl<T> Trait for T where …`) in the same way.  A blanket implementation is allowed only if the trait is defined in the current crate.

The compiler special‑cases the standard library prelude traits (`Clone`, `Debug`, `Default`, `Eq`, `Ord`, `Hash`, `Send`, `Sync`) for auto‑derivation.  Blanket implementations of these traits provided by the standard library are always allowed, and the orphan rule is relaxed for deriving these traits on user‑defined types.

## D.4 Violation Example (Rejected)

```blaze
// Crate A defines TraitA
// Crate B defines TypeB
// Crate C contains:
impl TraitA for TypeB { … }   // ERROR: both TraitA and TypeB are foreign to Crate C
```

## D.5 Allowed Examples

- `impl Clone for MyType { … }` – allowed, `MyType` is local.
- `impl MyTrait for Vec<i32> { … }` – allowed, `MyTrait` is local.
- `impl Display for MyType { … }` – allowed, `MyType` is local, even though `Display` is foreign.
