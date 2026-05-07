# Phase 4 – Ecosystem Crate: `blaze‑glob`

> **Goal:** Provide a fast, pure‑Blaze library for matching file paths against glob patterns (wildcards).  It supports the standard glob syntax (`*`, `?`, `[abc]`, `[a‑z]`, `[!…]`, `{a,b,…}`, `**` for recursive matching) and integrates with `std::path` and `std::fs`.  All operations are pure; I/O only occurs when the user explicitly iterates over the file system using a glob.

---

## 1. Core Types

### 1.1 `Pattern`

```
pub struct Pattern {
    text: Text,
    tokens: Vec<PatternToken>,
}
```

- A compiled glob pattern.  Constructed from a string and internal tokenisation.
- Linear (move semantics); not `@copy` because it contains heap‑allocated tokens.

### 1.2 `PatternToken`

```
enum PatternToken {
    Char(char),
    AnyChar,           // '?'
    AnySequence,       // '*' (non‑recursive: matches any characters except path separator)
    RecursiveSequence, // '**' (matches zero or more path components)
    Class { chars: Vec<ClassChar>, negated: bool },
    Alternation(Vec<Pattern>),
    Literal(Text),
}
```

- `ClassChar` is either a single `char` or a range `{ start, end }`.

---

## 2. Compilation

```
impl Pattern {
    pub fn new(glob: &str) -> Result<Pattern, GlobError>;
}
```

- Parses the glob string and compiles it into a `Pattern`.
- The syntax:
  - `*` – matches any sequence of characters except the path separator (`/`).
  - `?` – matches any single character except the path separator.
  - `[abc]` – matches one character from the set; `[!abc]` negates.  Ranges `[a‑z]` are supported.
  - `{a,b,c}` – matches any of the comma‑separated alternatives; nested alternatives are allowed.
  - `**` – when used as an entire path component (e.g., `a/**/b`), matches zero or more directories.  When used inside a component (e.g., `a**b`), it is treated as two consecutive `*` patterns matching any sequence.
  - `\` (backslash) escapes the next meta‑character.
- Returns `Err` if the glob string is malformed (e.g., unclosed bracket).

---

## 3. Matching

```
impl Pattern {
    pub fn matches(&self, path: &str) -> bool;
    pub fn matches_path(&self, path: &Path) -> bool;
}
```

- `matches` tests a string against the pattern.  The path separator is `/` (can be configured to platform separator but default is universal `/`).  The matching is case‑sensitive by default; a builder method `case_insensitive()` returns a case‑insensitive pattern.
- `matches_path` accepts a `Path` and converts to a string for matching.
- Anchoring: the pattern matches the **entire** path by default.  For example, `*.txt` matches `foo.txt` but not `foo.txt.bak`.  If the pattern has no `/` and does not start with `*`, it matches only the file name, not the full path.  The crate implements this via `matches` by first checking if the pattern contains a `/`.  If not, it matches against the last component of the path.  For `matches_path`, the full path is used if the pattern contains `/`; otherwise only the file name.  This behavior is configurable via `Pattern::with_full_path()`.

---

## 4. Iteration (Optional, requires `io`)

When the `fs` feature is enabled, the crate can use `std::fs` and `std::path` to walk a directory tree and find files that match the pattern.

### 4.1 `glob`

```
pub fn glob(pattern: &Pattern, root: &Path) -> Result<impl Iterator<Item = Path>, GlobError>;
```

- Recursively walks the directory tree and yields only the paths that match.  Uses `**` to enable recursive descent; without `**`, only the direct children are searched.
- The iterator is lazy; each call to `next` may perform I/O (`io` effect).

### 4.2 `GlobEntry`

```
pub struct GlobEntry {
    pub path: Path,
    pub is_dir: bool,
}
```

- The iterator yields `GlobEntry` (or simply `Path` if the user calls `glob_paths`).

---

## 5. Supporting Types

- **`CaseSensitivity`** – enum `Sensitive`, `Insensitive`.
- **`GlobError`** – wraps `std::io::Error` and a `ParseError` variant.

---

## 6. Error Handling

```
pub enum GlobError {
    Parse(Text),
    Io(std::io::Error),
}
```

- `Parse` is returned for invalid glob syntax.
- `Io` is returned for file‑system traversal errors.

---

## 7. Implementation Notes

- The tokenisation converts the glob string into a sequence of tokens using a recursive‑descent parser.  Alternations `{a,b}` are expanded into multiple `Pattern` objects (stored in the `Alternation` token), and matching attempts each alternative.
- The matching algorithm uses a simple backtracking over the token list.  For `**`, it tries consuming zero or more path components recursively, which may cause branch‑factor but typical globs are small enough that performance is not a problem.
- The file‑system iterator uses `std::fs::read_dir` and filters lazily, avoiding collecting the entire tree.
- On Windows, the path separator is `\`; the pattern compiler can be set to platform‑native mode via a builder flag.

---

## 8. Testing

- **Basic patterns:** `*.txt` matches `foo.txt`, not `bar.zip`.
- **Character classes:** `[a‑z]*.txt` matches `abc.txt`, not `123.txt`.
- **Alternation:** `{*.rs,*.blz}` matches `main.rs` and `lib.blz`.
- **Recursive:** `src/**/*.rs` matches all `.rs` files under `src/`.
- **Case insensitivity:** `*.TXT` with case‑insensitive flag matches `foo.txt`.
- **File‑system iteration:** Create a temp directory with files, glob it, and verify the returned paths.

All tests must pass on all platforms.
