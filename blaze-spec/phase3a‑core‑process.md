# Blaze Phase 3a – Core Library: Process Management (`std::process`)

> **Goal:** Implement the `std::process` module exactly as specified.  This module provides the `Command` struct for spawning and controlling child processes, along with the `Output` and `ExitStatus` types.

---

## 1. `Command`

### 1.1 Struct

```
pub struct Command {
    program: Text,
    args: Vec<Text>,
    env: Vec<(Text, Text)>,
    current_dir: Option<Text>,
}
```

- `program`: path to the executable.
- `args`: command‑line arguments (excluding the program name).
- `env`: additional environment variables to set or override.
- `current_dir`: if set, the child process’s working directory.

### 1.2 Methods

```
impl Command {
    pub fn new(program: &str) -> Command;
    pub fn arg(&mut self, a: &str) -> &mut Command;
    pub fn env(&mut self, key: &str, val: &str) -> &mut Command;
    pub fn current_dir(&mut self, dir: &str) -> &mut Command;
    pub fn output(&mut self) -> Result<Output, Error>;
    pub fn status(&mut self) -> Result<ExitStatus, Error>;
}
```

- `new(program)`: creates a new `Command` with the executable path and no arguments, no extra environment, and no working directory override.
- `arg(a)`: adds an argument (strings are appended in order).  Returns `self` for chaining.
- `env(key, val)`: sets an environment variable for the child (overrides existing).  Returns `self`.
- `current_dir(dir)`: sets the child’s working directory.  Returns `self`.
- `output()`: runs the command, waits for completion, and returns the collected stdout and stderr as `Output`.  If the command cannot be started, returns an `Error`.
- `status()`: runs the command, waits for completion, and returns the exit status only (stdout/stderr are inherited from the parent).  Returns an `Error` if the command cannot be started.

---

## 2. `Output`

```
pub struct Output {
    pub status: ExitStatus,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}
```

- Contains the exit status of the child process and the captured standard output and standard error streams as raw bytes.

---

## 3. `ExitStatus`

```
pub struct ExitStatus(i32);
```

### 3.1 Methods

```
impl ExitStatus {
    pub fn success(&self) -> bool;
    pub fn code(&self) -> Option<i32>;
}
```

- `success()`: returns `true` if the exit code is 0 (Unix convention) or indicates success on the platform.
- `code()`: returns the raw exit code if it is a normal termination; on Unix, `None` is returned for signal terminations (but for simplicity, we will return the signal number as a negative value? Actually the spec says `Option<i32>` and returns the code if available.  On Unix, `WIFEXITED` true gives the exit status; else `None`.  We'll implement accordingly.)

---

## 4. `Error`

```
pub struct Error(i32);
```

- Represents errors from process spawning.  Contains a platform‑specific error code; we may add a method to retrieve a message later.

---

## 5. Implementation Notes

- On Unix, `output()` and `status()` use `fork`/`exec` (or `posix_spawn`).  On Windows, they use `CreateProcess`.  The C backend wraps these system calls using the platform’s standard library functions.
- Environment variables from the parent are inherited unless overridden by `env()`.
- The `arg` method simply appends strings; the child process receives the program name as `argv[0]`.

---

## 6. Testing

- Execute a simple command (e.g., `echo` on Unix, `cmd /c echo` on Windows) and verify that its output is captured correctly.
- Test that setting environment variables works by running a child that prints an environment variable.
- Verify exit status for success and failure (run a command that exits with a non‑zero code).
- Test `status()` with a child that produces no output.
- All tests must pass before moving to the next module.
