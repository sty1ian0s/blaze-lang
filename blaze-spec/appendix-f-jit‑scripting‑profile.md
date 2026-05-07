# Appendix F – JIT Scripting Profile (Expanded)

> **Status:** Normative.  This appendix defines the relaxed rules that apply when a `.blz` file is executed via `blaze run` or `blaze repl`.  It also specifies the `blaze fix --aot-ify` command that converts a JIT script into a fully‑annotated, production‑ready AOT module.

---

## F.1 Activation

The JIT scripting profile is automatically applied to any file loaded by `blaze run` or `blaze repl`.  The compiler detects whether it is compiling for AOT (`blaze build`) or JIT and applies the rules in this appendix accordingly.  The JIT profile is **never** applied to library code compiled by `blaze build`.

---

## F.2 Top‑Level Statement Wrapping

Any expression or statement found at the top level that is not an item (`fn`, `struct`, `use`, …) is automatically wrapped into a synthesised `main` function:

```
fn main() {
    // all top‑level statements in order
}
```

If an explicit `main` function is already present, no wrapping occurs and top‑level statements are a compile‑time error.

---

## F.3 Extended Type Inference

Within the JIT profile, the compiler relaxes annotation requirements:

- **Parameter types** may be omitted.  The compiler infers them from the function body.
- **Return types** may be omitted.  The compiler infers the least upper bound of all returned expressions.
- **Placeholder type `_`** may be used for simple generic functions (`fn identity(x: _) -> _ { x }`), inferred as `fn<T>(x: T) -> T`.
- **`where` clauses** may be omitted for simple bounds; the compiler infers them from usage.

All inference is performed by the same Hindley‑Milner engine used in AOT mode.  Inferred types are stored in the module’s metadata and can be displayed by the LSP.

---

## F.4 Implicit Global Region

Every script executes inside an implicit `region` block that spans the entire program execution.  The default arena allocator is freed when the script exits.  The developer may create explicit `region { … }` blocks for finer control, but for simple scripts no region syntax is required.

---

## F.5 Relaxed Effect Annotations

The effect system still infers the minimal effect set for every function, but explicit effect specifications (`/ io`, `/ pure`, etc.) are **optional**.  If omitted, the inferred set is used.  If provided, it must be a superset of the inferred set or a compile‑time error.

---

## F.6 Automatic Imports

In addition to the standard prelude, the JIT profile automatically imports the following modules as if they were `use`d at the top of every script:

- `std::io::*`
- `std::collections::*`
- `std::iter::*`
- `std::window::*`
- `std::string::*`
- `std::fs::*` (if the host OS permits)
- `std::env::*`
- `std::process::*`

This allows immediate productive use without writing any import statements.  Unused imports are ignored; `blaze fix` will later remove them.

---

## F.7 Implicit Cloning for `@copy` Types

Affine (use‑once) rules are **relaxed** for types that are annotated `@copy` (or would be eligible for `@copy`).  The compiler silently duplicates the value instead of requiring an explicit `.clone()` call.  The programmer may still force a move with the `move` keyword if desired.

Example:
```blaze
let x = 5;
let y = x + x;   // allowed in JIT; x is implicitly copied
```

In AOT mode, `x` would be moved by the first use and the second use would be an error.  `blaze fix` inserts `@copy` annotations on the types that were used this way, or keeps them affine if they were only moved.

---

## F.8 Top‑Level Mutable Statics (`@global`)

The `@global` annotation on a `let mut` binding at the top level declares a mutable global variable.  In JIT mode, the compiler allocates a small backing store for each global and wraps access with a thin, safe `Mutex` (or an atomic where possible).

```blaze
@global let mut counter = 0;

fn increment() {
    counter += 1;
    println(counter);
}
```

In AOT mode, global mutable statics are forbidden.  `blaze fix` rewrites `@global` variables into an explicit `AppState` struct, passes it as a `&mut` parameter to functions that need it, or converts the logic into an actor.

---

## F.9 Automatic Closure Captures

In JIT mode, the compiler analyzes the body of a closure and automatically determines how to capture each variable:

- If a variable is used only once and not by any other closure, it is **moved** into the closure.
- If a variable is used by multiple closures or beyond the lifetime of the closure, it is wrapped in `std::rc::Rc` (or `std::arc::Arc` if needed for threads) and cloned transparently.

The programmer writes closures without specifying `move`, `clone`, or reference captures.  `blaze fix` later rewrites the closures with explicit capture clauses (`move`, `clone`, `&`, etc.) as required for AOT.

---

## F.10 Dynamic Dispatch by Default for Trait Objects

When a function expects an `impl Trait` parameter but the caller passes a concrete type, the JIT compiler may box the value into a `dyn Trait` and use dynamic dispatch instead of monomorphising.  This avoids emitting multiple copies of the same function during rapid iteration.

In AOT mode, `blaze fix` (guided by `@hot` attributes or profiling data) will monomorphise the hot paths and insert explicit `impl Trait` annotations, keeping the performance of static dispatch where it matters.

---

## F.11 Automatic Error Propagation

The JIT mode provides a `try!` macro that wraps any expression that may fail.  If the expression evaluates to an `Err` or `None`, the macro prints the error to `stderr` and terminates the current function (or the script).  The programmer never writes `?` or checks `Result` manually unless they want to.

```blaze
let data = try!(std::fs::read_to_text("config.toml"));
```

In AOT mode, `blaze fix` replaces `try!` with `?` and ensures the enclosing function returns `Result` with the appropriate error type, adding explicit error propagation paths.

---

## F.12 `blaze fix --aot-ify`

The `blaze fix` tool, when invoked with `--aot-ify`, reads a JIT script and produces an equivalent, fully‑annotated Blaze module ready for AOT compilation.

### F.12.1 Transformations Performed

1. **Add type annotations** – parameter types, return types, generic parameters, and `where` clauses are inserted based on the JIT‑inferred types.
2. **Add `@copy` annotations** – types that were implicitly copied are marked `@copy`.
3. **Replace `@global` variables** – an `AppState` struct is generated; globals become fields; functions that used them receive a `&mut AppState` parameter.
4. **Rewrite closures** – implicit captures are replaced with explicit `move`, `clone`, `ref`, etc.
5. **Replace `try!` with `?`** – error propagation is made explicit; functions gain `Result` return types.
6. **Remove unused imports** – the full auto‑imports are pruned; only the actually used ones are written.
7. **Monomorphise hot paths** – `dyn Trait` usages that are marked `#[hot]` (or inferred via profiling) are replaced with `impl Trait` and concrete instantiation.
8. **Format the output** – the resulting source is run through `blaze fmt` for canonical formatting.

### F.12.2 Idempotency

`blaze fix --aot-ify` is idempotent: running it a second time on an already‑fixed file produces no changes.  All inserted annotations preserve the original semantics.

### F.12.3 Integration with Build

Developers may run `blaze fix --aot-ify` manually before a release, or they may configure their `blaze.toml` to run it automatically as a pre‑build step:

```
[profile.release]
jit-to-aot = true   # run blaze fix --aot-ify before compiling
```

---

## F.13 File Extension

Blaze source files use the extension `.blz`.  The JIT scripting profile applies to any file with this extension when executed via `blaze run` or `blaze repl`.  `blaze build` always uses AOT mode and expects a fully annotated module (or one that has been processed by `blaze fix`).
