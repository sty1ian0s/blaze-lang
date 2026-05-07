# Blaze Phase 3a – Core Library: Environment (`std::env`)

> **Goal:** Implement the `std::env` module exactly as specified.  This module provides access to environment variables, the current working directory, and program arguments.

---

## 1. Functions

### 1.1 `var`

```
pub fn var(key: &str) -> Result<Text, VarError>;
```

- Retrieves the value of the environment variable `key`.
- Returns `Ok(value)` if the variable exists, otherwise `Err(VarError)`.
- `VarError` indicates whether the variable was not found or contained invalid Unicode (since environment variables are typically OS‑provided byte sequences, but Blaze requires valid UTF‑8; if the variable contains non‑UTF‑8 bytes, `var` returns an error).  For simplicity, `VarError` will be a unit struct with a descriptive message.

### 1.2 `set_var`

```
pub fn set_var(key: &str, value: &str);
```

- Sets the environment variable `key` to `value`.
- Panics if the operation fails (e.g., invalid key name or insufficient permissions).

### 1.3 `remove_var`

```
pub fn remove_var(key: &str);
```

- Removes the environment variable `key`.
- Panics on failure.

### 1.4 `current_dir`

```
pub fn current_dir() -> Result<Text, io::Error>;
```

- Returns the current working directory as a `Text`.
- Returns an `io::Error` if the directory cannot be obtained (e.g., the directory has been deleted).

### 1.5 `args`

```
pub fn args() -> Vec<Text>;
```

- Returns the command‑line arguments passed to the program, including the program name as the first element.
- Arguments are UTF‑8 strings; if the OS provides non‑UTF‑8 arguments, they are replaced with a placeholder (or the function panics – we will panic on invalid UTF‑8, as the platform should supply valid Unicode arguments in modern environments).

---

## 2. `VarError`

```
pub struct VarError {
    // private
}
```

- `VarError` may provide a method `to_text()` that returns a human‑readable error description.  For now, we keep it minimal.

---

## 3. Implementation Notes

- Underneath, `var`, `set_var`, and `remove_var` call the respective OS functions (e.g., `getenv`, `setenv`, `unsetenv` on Unix; `GetEnvironmentVariable` on Windows).
- `current_dir` uses the appropriate system call.
- `args` is typically obtained from the arguments passed to `main` (or provided by the runtime before the program starts).  In our bootstrapping environment, the C backend passes `argc`/`argv` to the generated main function, and Blaze’s runtime wrapper stores them for retrieval.

---

## 4. Testing

- `var`: set and retrieve an environment variable; verify that a missing variable returns an error.
- `set_var` and `remove_var`: test the basic cycle.
- `current_dir`: verify that the returned directory exists and is not empty.
- `args`: run a test program and check that the arguments are correctly returned (test harness can invoke the test executable with known arguments).

All tests must pass before moving to the next module.
