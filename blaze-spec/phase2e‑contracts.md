# Blaze Phase 2e – Contracts, Invariants, and Testing Attributes

> **Goal:** Implement function contracts (`requires`/`ensures`), struct invariants (`contract_invariant`), the `@fuzz` and `@test_case` macros, doctests, and the automatic removal of contract checks in release builds.  The parser already accepts the syntax; this phase adds the semantic analysis and code generation for runtime checks, as well as the test runner integration.

---

## 1. Function Contracts

### 1.1 Syntax
```
contract_clause = { "requires" expr } { "ensures" expr }
```
- A function may have zero or more `requires` (preconditions) and zero or more `ensures` (postconditions).
- Each expression must be of type `bool` and may refer to the function’s parameters, the special variable `result` (in `ensures` only), and the function’s state.
- Contracts cannot have side effects (they are checked in debug mode and must be pure).

### 1.2 Semantics
- In **debug mode** (default `blaze build` without `--release`):
  - Before the function body executes, all `requires` clauses are evaluated.  If any is `false`, the program panics with a message stating which contract failed.
  - After the function body executes (and before the return value is passed to the caller), all `ensures` clauses are evaluated.  If any is `false`, a panic occurs.
- In **release mode** (`--release`):
  - All `requires` and `ensures` clauses are **removed** from the compiled code.  They have zero runtime cost.

### 1.3 Interaction with Linear Types
- Contracts may read from parameters and the result, but must not consume linear values (i.e., they must only take references).
- The compiler verifies that contract expressions are pure (empty effect set).

---

## 2. Struct Invariants

### 2.1 Syntax
```
struct_invariant = "contract_invariant" expr ";"
```
- Placed inside a struct declaration, after the fields.
- The expression must be `bool` and may refer to the struct’s fields via `self.field`.

### 2.2 Semantics
- In **debug mode**:
  - After construction of the struct (via `StructName{...}`), the invariant is checked.
  - After any method that takes `&mut self` returns, the invariant is checked.
  - If the invariant is `false`, a panic occurs with a message indicating which struct’s invariant failed.
- In **release mode**:
  - All invariant checks are **removed**, just like function contracts.

---

## 3. Testing Attributes

### 3.1 `@fuzz`
- Applied to a **pure** function that has at least one `requires` and one `ensures` clause.
- Syntax: `@fuzz(iterations = N)` where `N` is an integer literal (default `1_000_000`).
- The compiler generates a fuzz harness that:
  - Generates random inputs that satisfy the `requires` clause (using the type’s domain).
  - Calls the function with those inputs.
  - Checks the `ensures` clause on the output.
  - Panics if any check fails.
- The harness is only compiled and run during `blaze test`; it is not part of the production binary.
- The fuzz harness is executed with a deterministic PRNG seeded by the test runner, reproducible with `--reproducible`.

### 3.2 `@test_case`
- Applied to a `@test` function.
- Syntax: `@test_case([ (arg1, arg2, ..., expected), ... ])`
- The function must have parameters matching the tuple arity and types, and an expected value (if checking result) or assert inside.
- For each tuple, the test function is called and must not panic.
- The test runner reports each case separately.

### 3.3 Doctests
- Doc comments `///` and `//!` may contain Markdown code blocks (triple backticks).
- The compiler extracts code blocks and treats them as tests.
- Each doctest is compiled and run; if it panics or fails an assertion, the test fails.
- Doctests can reference the enclosing module’s public API.

---

## 4. Release‑Mode Removal

### 4.1 Contracts
- When compiling with `--release` (or equivalent profile), all `requires`, `ensures`, and `contract_invariant` checks are **omitted** from the generated code.
- The compiler simply does not emit the evaluation of those expressions.
- The expressions themselves are still type‑checked, but they are dead code.

### 4.2 `debug_assert!`
- The standard library provides a macro `debug_assert!` that expands to nothing in release mode.
- Its behavior is identical to C’s `assert.h` when `NDEBUG` is set.

### 4.3 Configuration Override (future)
- The `blaze.toml` profile settings `keep_contracts` and `keep_assertions` may override this behavior, but for Phase 2e we simply follow the default: **debug checks, release no checks**.

---

## 5. Implementation Notes

### 5.1 Contract Evaluation
- The compiler generates a wrapper around each function that, in debug mode, evaluates the contracts.
- The wrapper is a small trampoline: check `requires` → call original → check `ensures`.
- For struct invariants, the compiler inserts the check after every `&mut self` method returns and after construction.

### 5.2 Fuzz Harness Generation
- The compiler synthesises a new function for each `@fuzz` annotations.
- That function uses the reflection API (minimal at this point) to generate random values of the parameter types within the `requires` domain.
- The generated harness is compiled and linked as a test.

### 5.3 Doctest Extraction
- The parser retains documentation comments as part of the AST.
- The test runner (`blaze test`) traverses all doc comments, extracts code blocks, compiles them as separate tiny files, and runs them.

---

## 6. Testing

For this phase:

- Write functions with `requires`/`ensures` and verify they are executed in debug mode and removed in release (by checking generated code).
- Write a struct with `contract_invariant`; test that invalid construction panics in debug.
- Apply `@fuzz` to a simple pure function with contracts; verify that the fuzz harness is generated and catches a deliberately introduced bug.
- Use `@test_case` on a test function and ensure all cases run.
- Write doctests in `///` comments and run `blaze test` to confirm they pass.
- Test that `--release` removes all contract evaluations (using a simple script that checks the absence of the check code in the emitted binary or assembly).
- All tests must pass before Phase 2f begins.
