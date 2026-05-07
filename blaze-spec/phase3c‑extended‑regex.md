# Blaze Phase 3c – Extended Library: Regular Expressions (`std::regex`)

> **Goal:** Implement the `std::regex` module exactly as specified.  This module provides a compile‑time or runtime regular expression engine that searches text for patterns.  The engine supports the common regex syntax (characters, classes, repetition, grouping, alternation, anchors) and returns match locations.  Performance should be competitive with backtracking engines for typical patterns, but no exponential blow‑up is guaranteed (avoiding catastrophic backtracking by using an automaton‑based approach).

---

## 1. `Regex` Type

### 1.1 Struct

```
pub struct Regex {
    // private compiled representation
}
```

- A compiled regular expression that can be used to search text.
- It is linear (must be consumed exactly once).  Cloning a `Regex` is not permitted; instead, create a new one with the same pattern if multiple copies are needed.

### 1.2 Construction

```
impl Regex {
    pub fn new(pattern: &str) -> Result<Regex, Error>;
}
```

- Compiles the given regex pattern string.  Returns `Ok(Regex)` if the pattern is valid, or `Err(Error)` with a description of the syntax error.
- The regex syntax is described in Section 3.

---

## 2. Matching Methods

All matching methods operate on a `&str` input and return information about the first match or whether a match exists.

### 2.1 `is_match`

```
impl Regex {
    pub fn is_match(&self, text: &str) -> bool;
}
```

- Returns `true` if the pattern matches anywhere in `text`.

### 2.2 `find`

```
impl Regex {
    pub fn find(&self, text: &str) -> Option<Match>;
}
```

- Searches for the first match of the pattern in `text`, returning the byte offsets and the matched string slice as a `Match`, or `None` if no match.

### 2.3 `captures`

```
impl Regex {
    pub fn captures(&self, text: &str) -> Option<Captures>;
}
```

- Returns the overall match and any captured groups as a `Captures` object.  If no match, returns `None`.

---

## 3. Match Types

### 3.1 `Match`

```
pub struct Match {
    start: usize,
    end: usize,
    text: Text,   // the matched substring (owned copy)
}

impl Match {
    pub fn start(&self) -> usize;
    pub fn end(&self) -> usize;
    pub fn as_str(&self) -> &str;
}
```

- `start()`: byte offset of the first character of the match.
- `end()`: byte offset of the character following the match.
- `as_str()`: returns the matched substring as a string slice.

### 3.2 `Captures`

```
pub struct Captures {
    // captures: Vec<Option<Match>> (capture group 0 is the overall match)
}
```

- Represents the captured groups for a successful match.
- Capture group 0 is always the entire match; subsequent groups correspond to parenthesized sub‑expressions in order of opening parenthesis.

```
impl Captures {
    pub fn get(&self, index: usize) -> Option<Match>;
    pub fn len(&self) -> usize;
}
```

- `get(index)`: returns the `Match` for capture group `index`, or `None` if that group did not participate in the match.
- `len()`: returns the number of capture groups (including group 0).

---

## 4. Regex Syntax

The supported syntax is a subset of Perl‑compatible regular expressions, including:

- **Literals:** any character except meta‑characters matches itself.
- **Meta‑characters:** `. ^ $ * + ? { } [ ] \ | ( )`
- **Character classes:** `[abc]`, ranges `[a-z]`, negated `[^...]`, predefined classes: `\d`, `\D`, `\w`, `\W`, `\s`, `\S`, `.` (any character except newline).
- **Repetition:** `*` (zero or more), `+` (one or more), `?` (zero or one), `{n}`, `{n,}`, `{n,m}`.  Repetition is **greedy** by default; `?` suffix makes it lazy (e.g., `*?`, `+?`).
- **Concatenation** and **Alternation** (`|`).
- **Anchors:** `^` (start of string), `$` (end of string).
- **Grouping:** `( ... )` for capturing, `(?: ... )` for non‑capturing.
- **Escapes:** `\ ` followed by a meta‑character matches the character literally; `\n`, `\r`, `\t` etc. for special characters.

**Not supported** (in version 1.0): backreferences, look‑ahead/behind, atomic groups, Unicode property classes (`\p{…}`), conditionals, recursion.  The engine must report a syntax error if an unsupported feature is used.

---

## 5. Error Handling

```
pub struct Error {
    message: Text,
    pos: usize,        // byte offset in pattern where error occurred (0‑based)
}

impl Error {
    pub fn message(&self) -> &str;
    pub fn pos(&self) -> usize;
}
```

- `message` describes the syntax error.
- `pos` points to the approximate location in the pattern.

---

## 6. Implementation Guidance

- The engine uses a finite automaton (NFA/DFA hybrid) to guarantee linear time search and avoid catastrophic backtracking.
- At compile time, the pattern is parsed into an AST, then converted into an NFA with ε‑transitions, and then optionally compiled into a DFA for the forward scan.  Capture groups are handled via tagged NFA states.
- The implementation may use the `std::collections` (Map, Vec) for internal state storage.
- The `Regex` struct is opaque; its size and alignment are unspecified but shall be `Dispose` if internal allocations are made.

---

## 7. Testing

- **Valid patterns:** Test simple literal matches, character classes, repetitions, greedy vs lazy, anchors, grouping, alternation.
- **Invalid patterns:** Provide malformed patterns and verify that `new` returns an `Err` with a meaningful message.
- **Capture groups:** Verify that `captures` returns the correct sub‑matches for parenthesized groups, including nested groups and non‑participating groups (returning `None` for those indexes).
- **Edge cases:** empty pattern (should match every empty string? Actually empty regex should match at every position; we can decide to allow it as a valid pattern that matches the empty string before any character).  Pattern with only alternation, etc.
- **Performance:** (Optional) test that a pattern with nested quantifiers does not cause exponential blow‑up on a non‑matching string.  A test could verify that the engine completes in reasonable time for a worst‑case like `(a*)*b` against `aaaaaaaaaaaaaaaaaaa` (should be linear, not catastrophic).  If the engine uses NFA simulation, it will be fine.

All tests must pass before moving to the next module.
