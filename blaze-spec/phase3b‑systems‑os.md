# Blaze Phase 3b – Systems Library: Platform OS (`std::os`)

> **Goal:** Implement the `std::os` module exactly as specified.  This module provides platform‑specific extensions, raw system call wrappers, and conditional compilation support for Unix and Windows targets.  All items in this module are inherently unsafe or platform‑dependent and must be used with care.

---

## 1. Module Structure

`std::os` is a **parent module** that does not export any items directly.  Instead, it contains two sub‑modules:

- `std::os::unix` – available only on Unix‑like targets (Linux, macOS, FreeBSD, etc.).
- `std::os::windows` – available only on Windows targets.

Each sub‑module exposes the raw system types, constants, and functions needed to write low‑level code that integrates directly with the operating system.  Both modules are guarded by `@cfg(target_os = "...")` so that the compiler only compiles the relevant one.

---

## 2. `std::os::unix`

### 2.1 Platform‑specific Re‑exports

The `unix` module re‑exports fundamental Unix types and functions from the C standard library (`libc`).  The exact set of exports matches the POSIX API commonly used for file descriptors, process control, and memory mapping.  For brevity, only the most essential items are listed here; a complete 1:1 mapping to POSIX is not required, but the following must be present:

```
pub type c_int = i32;
pub type c_uint = u32;
pub type pid_t = i32;
pub type uid_t = u32;
pub type gid_t = u32;
pub type mode_t = u16;
pub type off_t = i64;
pub type size_t = usize;
pub type ssize_t = isize;

pub const O_RDONLY: c_int = 0;
pub const O_WRONLY: c_int = 1;
pub const O_RDWR: c_int = 2;
pub const O_CREAT: c_int = 64;
pub const O_TRUNC: c_int = 512;
pub const O_APPEND: c_int = 1024;

pub const S_IFMT: mode_t = 0o170000;
pub const S_IFDIR: mode_t = 0o040000;
pub const S_IFREG: mode_t = 0o100000;

// Syscall wrappers (unsafe)
pub unsafe fn open(path: *const u8, flags: c_int, mode: mode_t) -> c_int;
pub unsafe fn close(fd: c_int) -> c_int;
pub unsafe fn read(fd: c_int, buf: *mut u8, count: size_t) -> ssize_t;
pub unsafe fn write(fd: c_int, buf: *const u8, count: size_t) -> ssize_t;
pub unsafe fn lseek(fd: c_int, offset: off_t, whence: c_int) -> off_t;
pub unsafe fn fsync(fd: c_int) -> c_int;
pub unsafe fn ftruncate(fd: c_int, length: off_t) -> c_int;
pub unsafe fn stat(path: *const u8, buf: *mut stat) -> c_int;
pub unsafe fn fstat(fd: c_int, buf: *mut stat) -> c_int;
pub unsafe fn mkdir(path: *const u8, mode: mode_t) -> c_int;
pub unsafe fn unlink(path: *const u8) -> c_int;
pub unsafe fn rmdir(path: *const u8) -> c_int;
```

These functions are **unsafe** and should be called only inside `unsafe` blocks.  They use C calling conventions and assume nul‑terminated byte strings for path arguments.

### 2.2 `stat` Structure

```
#[repr(C)]
pub struct stat {
    pub st_dev: u64,
    pub st_ino: u64,
    pub st_mode: mode_t,
    pub st_nlink: u64,
    pub st_uid: uid_t,
    pub st_gid: gid_t,
    pub st_rdev: u64,
    pub st_size: off_t,
    pub st_blksize: i64,
    pub st_blocks: i64,
    pub st_atime: i64,
    pub st_atime_nsec: i64,
    pub st_mtime: i64,
    pub st_mtime_nsec: i64,
    pub st_ctime: i64,
    pub st_ctime_nsec: i64,
}
```

(Exact layout matches the platform’s `struct stat`; on 64‑bit Linux this definition works.)

### 2.3 Other Constants

```
pub const STDIN_FILENO: c_int = 0;
pub const STDOUT_FILENO: c_int = 1;
pub const STDERR_FILENO: c_int = 2;
```

---

## 3. `std::os::windows`

### 3.1 Windows‑specific Re‑exports

On Windows, the module exposes the necessary types and functions to interface with the Win32 API.  All strings are UTF‑16 (using `*const u16`).  The following types and functions are provided:

```
pub type HANDLE = *mut u8;
pub type DWORD = u32;
pub type LPVOID = *mut u8;
pub type LPCWSTR = *const u16;
pub type BOOL = i32;

pub const GENERIC_READ: DWORD = 0x80000000;
pub const GENERIC_WRITE: DWORD = 0x40000000;
pub const CREATE_NEW: DWORD = 1;
pub const CREATE_ALWAYS: DWORD = 2;
pub const OPEN_EXISTING: DWORD = 3;
pub const FILE_ATTRIBUTE_NORMAL: DWORD = 0x80;
pub const INVALID_HANDLE_VALUE: HANDLE = -1 as HANDLE;

// Unsafe syscall wrappers
pub unsafe fn CreateFileW(
    lpFileName: LPCWSTR,
    dwDesiredAccess: DWORD,
    dwShareMode: DWORD,
    lpSecurityAttributes: *const u8,
    dwCreationDisposition: DWORD,
    dwFlagsAndAttributes: DWORD,
    hTemplateFile: HANDLE,
) -> HANDLE;

pub unsafe fn ReadFile(
    hFile: HANDLE,
    lpBuffer: LPVOID,
    nNumberOfBytesToRead: DWORD,
    lpNumberOfBytesRead: *mut DWORD,
    lpOverlapped: *mut u8,
) -> BOOL;

pub unsafe fn WriteFile(
    hFile: HANDLE,
    lpBuffer: *const u8,
    nNumberOfBytesToWrite: DWORD,
    lpNumberOfBytesWritten: *mut DWORD,
    lpOverlapped: *mut u8,
) -> BOOL;

pub unsafe fn CloseHandle(hObject: HANDLE) -> BOOL;
```

### 3.2 Helper Functions

Additionally, the module provides a conversion helper for UTF‑8 to UTF‑16:

```
pub fn to_utf16(s: &str) -> Vec<u16>;
```

This function allocates a `Vec<u16>` containing the null‑terminated wide string; it is safe and used by higher‑level wrappers.

---

## 4. Implementation Guidance

- The `unix` module is compiled only when `@cfg(target_os = "linux" | "macos" | ...)` holds.
- The `windows` module is compiled only when `@cfg(target_os = "windows")` holds.
- The `c_` types and syscall functions are implemented by directly forwarding to the corresponding C library functions via `extern "C"` blocks.
- All syscall functions are `unsafe` because they deal with raw pointers and assume pre‑validated arguments.
- Higher‑level modules like `std::fs`, `std::io`, and `std::net` (later) will build on top of these raw bindings.

---

## 5. Testing

- **Unix:** Write tests that open a temporary file, write data using unsafe syscalls, and read back.  (Only run on Unix targets.)
- **Windows:** Write similar tests using Win32 API.  (Only run on Windows.)
- **Compile‑time selection:** Confirm that code using `std::os::unix` does not compile on Windows, and vice versa.
- **Path conversion:** Test `to_utf16` with ASCII and non‑ASCII strings, verify the resulting buffer.

All tests must pass before moving to Phase 3c extended library modules.
