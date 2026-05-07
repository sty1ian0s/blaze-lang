# Blaze Phase 3a – Core Library: I/O (`std::io`)

> **Goal:** Implement the `std::io` module exactly as specified.  Every trait, struct, enum, and function listed here must be provided exactly as specified.  Tests must be written before implementation.

---

## 1. Traits

### 1.1 `Read`

```
pub trait Read {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Error>;
    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> Result<usize, Error>;
    fn read_to_text(&mut self) -> Result<Text, Error>;
}
```

**Behaviour:**
- `read(buf)`: Reads up to `buf.len()` bytes into the buffer.  Returns the number of bytes read (0 indicates EOF, provided no error).  If an error occurs, the number of bytes already transferred is unspecified.  May block if the underlying source blocks.
- `read_to_end(buf)`: Reads all remaining data, appending to `buf`.  Returns total bytes read.  Stops on EOF or error.
- `read_to_text()`: Reads all remaining data, interpreting it as UTF‑8, and returns a `Text` (from `std::string`).  Returns an error of kind `InvalidInput` if the data is not valid UTF‑8.

### 1.2 `Write`

```
pub trait Write {
    fn write(&mut self, buf: &[u8]) -> Result<usize, Error>;
    fn flush(&mut self) -> Result<(), Error>;
}
```

**Behaviour:**
- `write(buf)`: Writes the entire buffer, or returns the number of bytes written before an error occurred.  Short writes are possible; callers should retry if needed.  The default implementation of `write` attempts the whole buffer, but specific types may implement more efficient strategies.
- `flush()`: Ensures all buffered data reaches the destination.  Returns `Ok(())` or an error.

---

## 2. File

### 2.1 `File` struct

```
pub struct File {
    fd: i32,
    is_stdout: bool,
}
```

The `fd` field holds the operating system file descriptor.  `is_stdout` indicates whether the file should not be closed on disposal (e.g., standard output).

### 2.2 Methods

```
impl File {
    pub fn open(path: &str) -> Result<File, Error>;
    pub fn create(path: &str) -> Result<File, Error>;
    pub fn metadata(&self) -> Result<Metadata, Error>;
    pub fn set_nonblocking(&mut self, nonblocking: bool) -> Result<(), Error>;
}
```

**Behaviour:**
- `open(path)`: Opens an existing file for reading only.  Returns `ErrorKind::NotFound` if the file does not exist, `ErrorKind::PermissionDenied` on access violations.
- `create(path)`: Creates a new file or truncates an existing one, opening it for writing only.  Returns `ErrorKind::PermissionDenied` if the file cannot be created.
- `metadata()`: Retrieves file metadata (size, directory flag).  Returns `ErrorKind::Other` on failure.
- `set_nonblocking(nonblocking)`: Sets the file descriptor to non‑blocking mode if `nonblocking` is true, or blocking if false.  Returns an error if the operation fails (e.g., on a pipe that doesn’t support it).

### 2.3 `Read` and `Write` implementations

```
impl Read for File {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Error>;
    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> Result<usize, Error>;
    fn read_to_text(&mut self) -> Result<Text, Error>;
}

impl Write for File {
    fn write(&mut self, buf: &[u8]) -> Result<usize, Error>;
    fn flush(&mut self) -> Result<(), Error>;
}
```

These delegate to the system calls `read`/`write`/`fsync`.  Error codes from the OS are translated to `ErrorKind` values.

### 2.4 `Dispose` for `File`

```
impl Dispose for File {
    fn dispose(&mut self);
}
```
Calls `close` on the file descriptor unless `is_stdout` is true.

---

## 3. Metadata

```
pub struct Metadata {
    size: u64,
    kind: u8,   // 0 = file, 1 = directory
}
impl Metadata {
    pub fn len(&self) -> u64 { self.size }
    pub fn is_file(&self) -> bool { self.kind == 0 }
    pub fn is_dir(&self) -> bool { self.kind == 1 }
}
```

`Metadata` is obtained from `File::metadata()` and carries information from the file system.

---

## 4. Standard Streams

### 4.1 Types

```
pub struct Stdout { fd: i32 }
pub struct Stderr { fd: i32 }
pub struct Stdin  { fd: i32 }
```

### 4.2 Constructor functions

```
pub fn stdout() -> Stdout;
pub fn stderr() -> Stderr;
pub fn stdin()  -> Stdin;
```

These return handles for the standard output, error, and input streams, respectively.

### 4.3 Trait implementations

```
impl Write for Stdout {
    fn write(&mut self, buf: &[u8]) -> Result<usize, Error>;
    fn flush(&mut self) -> Result<(), Error>;
}
impl Write for Stderr {
    fn write(&mut self, buf: &[u8]) -> Result<usize, Error>;
    fn flush(&mut self) -> Result<(), Error>;
}
impl Read for Stdin {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Error>;
    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> Result<usize, Error>;
    fn read_to_text(&mut self) -> Result<Text, Error>;
}
```

`Stdout`/`Stderr` write to file descriptors 1 and 2; `Stdin` reads from descriptor 0.  Flush on `Stdout`/`Stderr` is a no‑op (or calls `fsync` if desired, but the specification does not require it – we will simply return `Ok(())`).

---

## 5. Error Handling

### 5.1 `Error` struct

```
pub struct Error(ErrorKind, Text);

impl Error {
    pub fn new(kind: ErrorKind, msg: &str) -> Error;
    pub fn kind(&self) -> ErrorKind;
    pub fn message(&self) -> &str;
}
```

### 5.2 `ErrorKind` enum

```
pub enum ErrorKind {
    NotFound = 0,
    PermissionDenied = 1,
    ConnectionRefused = 2,
    ConnectionReset = 3,
    Interrupted = 4,
    UnexpectedEof = 5,
    InvalidInput = 6,
    TimedOut = 7,
    Other = 8,
}
```

These represent categories of I/O errors.  The `[Debug]` and `[Display]` implementations should be derived using `@derive(Debug, Display)` later, but for now we provide manual implementations.

---

## 6. Testing

For each trait and type, write tests that:

- **`Read` trait (with a mock):** Implement a simple `Read` that returns fixed data and verifies `read`, `read_to_end`, `read_to_text` (including invalid UTF‑8 case).
- **`Write` trait (with a mock):** Collect written bytes, verify `write` and `flush`.
- **`File`:** Use temporary files to test `open`, `create`, `read`, `write`, `metadata`, and `set_nonblocking`.  Test error cases (missing file, permissions).  Ensure `Dispose` closes the descriptor.
- **Standard streams:** Test that `stdout()`, `stderr()`, and `stdin()` can be obtained and used (at least that writes to stdout/stderr succeed, and that reading from stdin does not panic).  For automated testing, stdin can be redirected.
- **`Error` and `ErrorKind`:** Ensure error creation and retrieval work correctly.

All tests must pass before the next core library module is implemented.
