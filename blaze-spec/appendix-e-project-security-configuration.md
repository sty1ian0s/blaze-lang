# Appendix E – Project Security Configuration

> **Status:** Normative.  This appendix defines the `blaze.toml` configuration schema that controls project‑wide security policies and how contracts, assertions, and safety checks are handled in different build profiles.

---

## E.1 The `blaze.toml` Manifest

Every Blaze crate may contain a `blaze.toml` file in its root directory.  The compiler parses this file before processing any source files.  Unknown keys cause a warning but not an error.  The `[security]` section and the `[profile]` sections are defined here.

## E.2 `[security]` Section

The following keys are normative under `[security]`:

### E.2.1 `public_functions_must_have_contracts`

- **Type:** `bool`
- **Default:** `false`
- **Description:** If `true`, every public function must carry at least one `requires` or `ensures` clause, or be annotated with `@no_contract`.  The `@no_contract` attribute must be accompanied by a comment (on the same line or the preceding line) explaining why no contract is needed.

**Example:**
```toml
[security]
public_functions_must_have_contracts = true
```

### E.2.2 `unsafe_requires_safety_comment`

- **Type:** `bool`
- **Default:** `false`
- **Description:** If `true`, every `unsafe` block must be immediately preceded by a line comment starting with `// SAFETY:`.  The compiler will reject any `unsafe` block that does not have such a comment.

**Example:**
```toml
[security]
unsafe_requires_safety_comment = true
```

### E.2.3 `panic_policy`

- **Type:** string, either `"unwind"` or `"abort"`
- **Default:** `"unwind"`
- **Description:** Specifies the behavior when a panic occurs.
  - `"unwind"`: the panic unwinds the stack, calling `Dispose` on live linear variables (current default).
  - `"abort"`: the process immediately terminates without unwinding.  This may enable dead‑code elimination of unwinding setup and reduce binary size.

**Example:**
```toml
[security]
panic_policy = "abort"
```

### E.2.4 `strict_narrowing`

- **Type:** `bool`
- **Default:** `false`
- **Description:** If `true`, any use of the `as` operator for a narrowing numeric conversion (e.g., `i32` as `i16`) is a compile‑time error unless the expression is wrapped in an `unsafe` block or uses the `try_into` method.  This prevents accidental loss of precision.

**Example:**
```toml
[security]
strict_narrowing = true
```

### E.2.5 `minimum_fuzz_iterations`

- **Type:** integer
- **Default:** `0`
- **Description:** The minimum number of iterations that `@fuzz` harnesses must run during `blaze test`.  The test command will fail if any fuzz harness runs fewer iterations than this number.  A value of `0` means no minimum is enforced.

**Example:**
```toml
[security]
minimum_fuzz_iterations = 100_000
```

## E.3 `[profile]` Section

The `[profile]` section controls how contracts and assertions are treated in different build profiles.  The compiler recognises at least the `debug` and `release` profiles.  Additional custom profiles may be defined.

### E.3.1 `keep_contracts`

- **Type:** `bool`
- **Default:** `true` for `debug`, `false` for `release`
- **Description:** When `true`, all runtime contract checks (`requires`, `ensures`, `contract_invariant`) are compiled into the binary.  When `false`, they are omitted, and no machine code is generated for them.

### E.3.2 `keep_assertions`

- **Type:** `bool`
- **Default:** `true` for `debug`, `false` for `release`
- **Description:** When `true`, all `debug_assert!` invocations are compiled into the binary.  When `false`, the `debug_assert!` macro expands to nothing.

### E.3.3 Example

```toml
[profile.debug]
keep_contracts = true
keep_assertions = true

[profile.release]
keep_contracts = false
keep_assertions = false
```

The compiler shall respect these settings.  If a profile is not explicitly configured, the defaults listed above apply.

## E.4 Interaction with `blaze fix`

The `blaze fix` tool may, when run with the `--aot-ify` flag, automatically insert `@no_contract` annotations on functions that are public but lack contracts, if the project’s `[security]` policy requires contracts.  It will also insert the required `// SAFETY:` comments for `unsafe` blocks when `unsafe_requires_safety_comment` is active.

## E.5 Testing

- Set `public_functions_must_have_contracts = true` and define a public function without a contract; verify the compiler rejects it.
- Set `unsafe_requires_safety_comment = true` and write an `unsafe` block without a comment; verify rejection.
- Set `panic_policy = "abort"` and trigger a panic; verify the process terminates immediately.
- Set `strict_narrowing = true` and attempt a narrowing `as` cast; verify the compiler rejects it.
- Set `minimum_fuzz_iterations = 10` and write a fuzz harness that runs 5 iterations; verify `blaze test` fails.
- Compile with `--release` and verify that contract evaluations and `debug_assert!` are absent from the generated binary.

All tests must pass on all platforms.
