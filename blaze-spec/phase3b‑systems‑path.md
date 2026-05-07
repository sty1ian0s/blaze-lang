# Blaze Phase 3b – Systems Library: Path Manipulation (`std::path`)

> **Goal:** Implement the `std::path` module exactly as specified.  This module provides the `Path` type for representing and manipulating file system paths in a platform‑independent manner.

---

## 1. `Path` Struct

```
pub struct Path {
    inner: Text,
}
```

- `Path` wraps a `Text` (i.e., `String`) that holds the path string using the platform’s native separator (`/` on Unix, `\` on Windows).  
- It is a linear type; cloning a `Path` requires an explicit `.clone()` call (derived via `@derive(Clone)` or manual).

### 1.1 Construction

```
impl Path {
    pub fn new(s: &str) -> Path;
}
```

- Creates a `Path` from a string slice.  The string is copied into an owned `Text`.

### 1.2 Accessors

```
impl Path {
    pub fn as_str(&self) -> &str;
}
```

- Returns a reference to the underlying path string.

### 1.3 Joining

```
impl Path {
    pub fn join(&self, other: &str) -> Path;
}
```

- Appends `other` to the current path, inserting the platform’s separator if necessary.  
- If `other` starts with a separator (i.e., is absolute), it replaces the current path entirely (platform‑specific behaviour).  
- Returns a new `Path` with the joined string.

### 1.4 Parent Directory

```
impl Path {
    pub fn parent(&self) -> Option<Path>;
}
```

- Returns the parent directory of the path, or `None` if the path has no parent (e.g., root directory, or a single file without a directory component).  
- The parent is determined by stripping the last component after the final separator.

### 1.5 File Extension

```
impl Path {
    pub fn extension(&self) -> Option<&str>;
}
```

- Returns the file extension (the substring after the last `.`) if one exists and is not the entire file name.  Returns `None` if there is no extension or if the dot is at the beginning (hidden files).  
- The returned string slice is borrowed from the inner `Text`.

---

## 2. Platform‑Specific Behaviour

- The separator is defined as `'/'` on Unix‑like platforms and `'\\'` on Windows.  The internal representation store paths with the canonical separator for the target platform.  The `join` method knows how to handle both separators when parsing `other`.
- Root detection: on Unix, root is `"/"`; on Windows, root can be a drive letter (`"C:\"`) or UNC path.  `parent()` returns `None` for these.
- Path normalization (collapsing `..` and `.`) is **not** performed automatically; the path is stored as provided.  A future extension may add a `normalize` method.

---

## 3. Trait Implementations

- `Clone` (provided by `@derive(Clone)` or manual) – deep copies the `Text`.
- `Dispose` (provided because `Text` is dispose) – the inner `Text` is disposed automatically when the `Path` falls out of scope.
- `PartialEq`, `Eq`, `PartialOrd`, `Ord` if derived later – for now, we don’t derive them, but manual comparison methods could be added if needed.

---

## 4. Testing

- **Construction and display:** Create a path and verify `as_str()` returns the original string.
- **Join:** Join two components and verify correct separator insertion.  Test joining an absolute path with a relative one and vice versa.
- **Parent:** Test with simple paths (`"/foo/bar"` → `"/foo"`), single component (`"foo"` → `None`), root (`"/"` → `None`).
- **Extension:** Test files with no extension, single dot, multiple dots, hidden files.
- **Platform independence:** Ensure tests behave consistently on both Unix and Windows by using if‑cfg in the test code, or write platform‑specific tests.

All tests must pass before moving to the next module.
