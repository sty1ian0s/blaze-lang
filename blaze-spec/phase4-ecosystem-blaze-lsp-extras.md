# Phase 4 – Ecosystem Crate: `blaze‑lsp‑extras`

> **Goal:** Provide additional, composable Language Server Protocol (LSP) features for Blaze editors and IDEs.  It extends the core `blaze lsp` with advanced code actions, semantic highlighting, inlay hints, call hierarchy, workspace symbol search, and project‑wide refactoring support.  The crate is designed to be loaded as a plugin by the core language server, or used standalone to build custom tooling.  All operations are pure; I/O only occurs when the LSP server reads or writes files.

---

## 1. Core Concepts

- **`Feature`** – a trait that every LSP extra feature implements, allowing the server to query capabilities and register handlers.
- **`SemanticTokens`** – provides semantic highlighting based on the compiler’s typed AST, not just syntax.
- **`InlayHints`** – infers and displays type annotations, parameter names, and other implicit information inline.
- **`CallHierarchy`** – builds incoming and outgoing call graphs for any function, using the effect system and monomorphisation data.
- **`WorkspaceSymbols`** – indexes all public symbols in the workspace for fast fuzzy search.
- **`RefactoringActions`** – offers project‑wide renames, extract function, inline variable, and other safe transformations.

All features operate on the compiler’s `std::meta` reflection and the flat AST; they do not modify source files directly but generate `TextEdit` sequences that the editor applies.

---

## 2. `Feature` Trait

```
pub trait Feature: Send + Sync {
    fn name(&self) -> &'static str;
    fn capabilities(&self) -> ServerCapabilities;
    fn register(&self, server: &mut LspServer);
}
```

- `capabilities` tells the client what the feature supports.
- `register` adds the feature's request handlers to the LSP server.

---

## 3. Semantic Tokens

The standard LSP token types are `keyword`, `string`, `number`, `comment`, etc.  Blaze’s semantic tokens add:

- `struct`, `enum`, `trait`, `type_alias` – for type definitions.
- `function`, `method` – for callable items.
- `parameter`, `variable`, `field` – for data access.
- `mutable` – for `mut` variables and `&mut` references.
- `linear` – for values that must be consumed (shown with a distinct colour).
- `effect` – for effect annotations (`/ io`, `/ pure`, etc.).

The semantic token provider uses the compiler’s symbol table to tag every identifier with its semantic category, even inside complex expressions.

---

## 4. Inlay Hints

### 4.1 Type Hints

For local variables with inferred types, the server can display the inferred type inline:

```
let x = 42;      // → let x: i32 = 42;
```

The hint is computed from the compiler’s metadata and shown as a faded annotation.

### 4.2 Parameter Name Hints

For function calls, the server can display parameter names for literals or expressions where the intent is not obvious:

```
draw_rect(10, 20, 100, 200);   // → draw_rect(x:10, y:20, width:100, height:200);
```

### 4.3 Chaining Hints

For iterator chains, the server can show the type of each intermediate step:

```
data.iter().map(|x| x*2).filter(|x| *x > 10).collect()
// → Vec<i32>   → Map<i32>   → Filter<i32>   → Vec<i32>
```

---

## 5. Call Hierarchy

The server builds incoming and outgoing call graphs using the compiler’s def‑use chains and monomorphisation records.

- **Incoming calls**: for any function `f`, lists all call sites in the workspace that directly invoke `f`.
- **Outgoing calls**: for any function `f`, lists all functions that `f` directly calls.

The graph respects the effect system: calls across effect boundaries (e.g., from a `pure` function to an `io` function) are highlighted as potential errors.

---

## 6. Workspace Symbols

The server maintains an incrementally updated index of every public symbol in the workspace.  The index supports fuzzy matching (substring) and is queried via the LSP `workspace/symbol` request.

The index includes:
- Module path
- Kind (function, struct, enum, trait, etc.)
- Doc comment preview (first line)
- Location (file + line)

The index is built from the pre‑compiled `.blzlib` metadata, so it does not require parsing source files.

---

## 7. Refactoring Actions

### 7.1 Rename

Performs a workspace‑wide rename of any symbol (variable, function, type, module).  The rename uses the compiler’s precise definition‑use information, avoiding false positives.

### 7.2 Extract Function

Given a selected block of code in a pure function, the server can extract it into a new function, automatically inferring the parameter types and return type, inserting a call at the original location, and adding the function to the module.

### 7.3 Inline Variable

Replaces all uses of a local variable with its definition expression, then removes the binding.  Applicable only when the variable is used at most once (affine constraint) or is `@copy`.

### 7.4 Convert to `@derive`

Detects manual trait implementations for `Debug`, `PartialEq`, `Default`, etc., and offers to replace them with `@derive(...)` attributes, reducing boilerplate.

---

## 8. Configuration

All features can be enabled or disabled in `blaze.toml`:

```
[lsp_extras]
semantic_tokens = true
inlay_hints = true
inlay_hints_show_types = true
inlay_hints_show_parameter_names = true
call_hierarchy = true
workspace_symbols = true
refactoring = true
```

The server reads this configuration at startup and advertises only the enabled capabilities.

---

## 9. Error Handling

The crate does not produce user‑facing errors; if a feature fails (e.g., the compiler metadata is missing), it silently disables itself and logs a warning via `blaze‑log`.

---

## 10. Testing

- **Semantic tokens:** Build a small project, request semantic tokens, verify that each token has the expected type and modifiers.
- **Inlay hints:** Request inlay hints for a function with inferred types, verify that the returned hints match the compiler’s inferred types.
- **Call hierarchy:** Build a project with known call chains, request incoming/outgoing calls, verify the graph edges.
- **Rename:** Perform a rename on a function, verify that all call sites are updated and no unrelated symbols change.
- **Extract function:** Extract a block, verify the new function is created and the call is inserted correctly.

All tests run against a mock LSP client that simulates editor interactions.
