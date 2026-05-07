# Blaze Phase 3b – Systems Library: File System (`std::fs`)

> **Goal:** Implement the `std::fs` module exactly as specified.  This module provides functions for common file system operations, wrapping the underlying OS system calls with Blaze’s error handling and types.

---

## 1. Public Functions

### 1.1 `read_to_text`

```
pub fn read_to_text(path: &str) -> Result<Text, io::Error>;
```

- Opens the file at `path` for reading, reads its entire content, and interprets it as UTF‑8.
- Returns `Ok(Text)` on success, or an `io::Error` (e.g., `NotFound`, `PermissionDenied`, `InvalidInput` if the file is not valid UTF‑8).
- Internally uses `File::open(path)?.read_to_text()`.

### 1.2 `read`

```
pub fn read(path: &str) -> Result<Vec<u8>, io::Error>;
```

- Reads all bytes from the file at `path` into a `Vec<u8>`.
- Uses `File::open(path)?.read_to_end(&mut buf)`.

### 1.3 `write`

```
pub fn write(path: &str, data: &[u8]) -> Result<(), io::Error>;
```

- Creates (or truncates) the file at `path` and writes the given byte slice into it.
- Uses `File::create(path)?.write_all(data)` (retries short writes until done).
- Returns `Ok(())` on success, or an `io::Error` on failure.

### 1.4 `create_dir`

```
pub fn create_dir(path: &str) -> Result<(), io::Error>;
```

- Creates a new directory at `path`.
- Fails with `AlreadyExists` if the directory already exists (or similar error kind `PermissionDenied`).
- Internally calls the OS `mkdir` system call.

### 1.5 `metadata`

```
pub fn metadata(path: &str) -> Result<Metadata, io::Error>;
```

- Retrieves metadata (size, type) of the file system object at `path` without opening it.
- Returns a `Metadata` struct (see below) or an `io::Error`.

---

## 2. `Metadata` Struct

```
pub struct Metadata {
    pub size: u64,
    pub is_file: bool,
    pub is_dir: bool,
    // additional fields may be added later (permissions, modification time, etc.)
}
```

- `size` – length of the file in bytes (or 0 for directories).
- `is_file` – `true` if the path is a regular file.
- `is_dir` – `true` if the path is a directory.

### 2.1 Constructor (internal)

`Metadata` is constructed from the raw OS stat structure inside the implementation of `metadata()` and `File::metadata()`.  It is not intended to be created directly by users.

---

## 3. Implementation Notes

- The module relies on `std::io` for basic file I/O and error handling.
- `read_to_text` and `read` use `File::open` and then `read_to_text`/`read_to_end` from the `Read` trait.
- `write` uses `File::create` and then loops `write` until the entire buffer is written.
- `create_dir` calls the OS‑specific `mkdir` function; on Unix, it uses `libc::mkdir(path, 0o777)`; on Windows, `CreateDirectoryW`.
- `metadata` calls the OS stat function; on Unix `stat`, on Windows `GetFileAttributesEx`.

---

## 4. Testing

- **`read_to_text`:** Write a temporary file with known content, call `read_to_text`, verify the returned `Text`.
- **`read`:** Same but verify the byte vector.
- **`write`:** Write bytes to a temporary file, then read back and compare.
- **`create_dir`:** Create a directory, verify it exists using `metadata` (see that `is_dir` returns true).
- **`metadata`:** Query an existing file and check `size`, `is_file`, `is_dir`.  For a directory, check `is_dir`.
- **Error handling:** Try to read a non‑existent file – expect `ErrorKind::NotFound`.  Try to create a directory in a location without permissions – expect appropriate error.

All tests must pass before moving to the next systems library module.
