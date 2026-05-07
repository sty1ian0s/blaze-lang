# Phase 5 – Ecosystem Crate: `blaze‑cuda`

> **Goal:** Specify the `blaze‑cuda` crate, which provides low‑level, unsafe, data‑oriented bindings to the NVIDIA CUDA GPU computing platform.  It is intended as a foundation for higher‑level GPU abstractions such as `blaze‑tensor` (when targeting NVIDIA GPUs) and for applications that require direct control over GPU kernels via CUDA C/C++ or PTX.  All CUDA resources are wrapped in linear types and carry the `gpu` effect.  This crate is inherently unsafe and should be used only inside `unsafe` blocks or by higher‑level safe wrappers.

---

## 1. Design Philosophy

The CUDA bindings provide direct access to the CUDA Runtime API (version 12.x) and the CUDA Driver API for low‑level control.  Each CUDA resource (device, context, stream, event, memory allocation, module, function) is wrapped in a Blaze struct that implements `Dispose` to call the appropriate cleanup function (`cuMemFree`, `cuCtxDestroy`, etc.).  All functions that modify the CUDA state machine are marked `unsafe`, and the crate does not attempt to hide the complexity or the threading requirements of CUDA.  Higher‑level crates (`blaze‑tensor`, `blaze‑ml`) will use these bindings to implement safe, data‑oriented GPU computing.

---

## 2. Core Types

### 2.1 `Device`

```
pub struct Device { id: i32 }
```

- Represents a CUDA‑capable GPU.  Obtained via `Device::count()` and `Device::get(index)`.
- `Device::count() -> usize` (safe).
- `Device::get(index: usize) -> Option<Device>` (safe).

### 2.2 `Context`

```
pub struct Context { inner: CUcontext }
```

- Linear; created by `Device::primary_context()` or `Device::create_context()`.  `Dispose` calls `cuCtxDestroy`.
- A context must be current on the calling thread to issue CUDA commands.  The crate provides `Context::set_current(&self)` (unsafe) and `Context::pop_current()` (unsafe) to manage the thread‑local context stack.

### 2.3 `Stream`

```
pub struct Stream { inner: CUstream }
```

- Linear; created by `Context::create_stream()`.  `Dispose` calls `cuStreamDestroy`.
- Commands issued to a stream execute asynchronously relative to the host.  Stream functions: `synchronize()`, `wait_event(event)`.

### 2.4 `Event`

```
pub struct Event { inner: CUevent }
```

- Linear; created by `Context::create_event()`.  `Dispose` calls `cuEventDestroy`.
- `record(stream)` records an event in a stream; `synchronize()` blocks until the event occurs; `elapsed_time(start, end)` measures duration.

---

## 3. Memory Management

### 3.1 `DeviceMemory`

```
pub struct DeviceMemory {
    ptr: *mut u8,
    size: usize,
}
```

- Linear; allocated by `DeviceMemory::new(size: usize) -> Result<DeviceMemory, CudaError>` (calls `cuMemAlloc`).  `Dispose` calls `cuMemFree`.  The pointer is valid only while the owning context is current.
- Methods:
  - `pub unsafe fn as_ptr(&self) -> *mut u8;`
  - `pub fn size(&self) -> usize;`
  - `pub unsafe fn copy_to_host(&self, host: *mut u8, size: usize) -> Result<(), CudaError>;`
  - `pub unsafe fn copy_from_host(&self, host: *const u8, size: usize) -> Result<(), CudaError>;`
  - `pub unsafe fn copy_to_device(&self, dst: &DeviceMemory, size: usize) -> Result<(), CudaError>;`
  - `pub unsafe fn memset(&self, value: u8, size: usize) -> Result<(), CudaError>;`

### 3.2 `UnifiedMemory`

```
pub struct UnifiedMemory {
    ptr: *mut u8,
    size: usize,
}
```

- Analogous to `DeviceMemory` but uses `cuMemAllocManaged`.  Accessible from both host and device.  `Dispose` calls `cuMemFree`.

---

## 4. Modules and Kernels

### 4.1 `Module`

```
pub struct Module { inner: CUmodule }
```

- Linear; created by `Module::from_ptx(ptx: &str) -> Result<Module, CudaError>` or `Module::from_file(path: &str) -> Result<Module, CudaError>`.  `Dispose` calls `cuModuleUnload`.
- Methods:
  - `pub fn get_function(&self, name: &str) -> Result<Function, CudaError>;`

### 4.2 `Function`

```
pub struct Function { inner: CUfunction }
```

- A handle to a kernel, obtained from a `Module`.
- Methods:
  - `pub unsafe fn launch(&self, grid: (u32, u32, u32), block: (u32, u32, u32), shared_mem: u32, stream: &Stream, args: &[*mut c_void]) -> Result<(), CudaError>;`
  - `pub fn max_threads_per_block(&self) -> i32;`
  - `pub fn shared_size_bytes(&self) -> usize;`
  - `pub fn optimal_block_size(&self) -> i32;`

---

## 5. Graphics Interop (Optional)

For applications that need to share buffers between CUDA and a graphics API (e.g., `blaze‑wgpu`, `blaze‑vulkan`), the crate provides:

```
pub struct GraphicsResource { /* … */ }
```

- Functions: `register_buffer`, `register_image`, `map_resources`, `unmap_resources`, `get_mapped_pointer`.  These are feature‑gated on `graphics` and are inherently unsafe.

---

## 6. Error Handling

```
pub enum CudaError {
    InvalidValue,
    OutOfMemory,
    NotInitialized,
    Deinitialized,
    NoDevice,
    InvalidDevice,
    InvalidImage,
    InvalidContext,
    ContextAlreadyCurrent,
    MapFailed,
    UnmapFailed,
    ArrayIsMapped,
    AlreadyMapped,
    NoBinaryForGpu,
    AlreadyAcquired,
    NotMapped,
    InvalidSource,
    FileNotFound,
    NvlinkFailed,
    InvalidPtx,
    UnloadingFailed,
    LaunchFailed,
    LaunchOutOfResources,
    LaunchTimeout,
    LaunchIncompatibleTexturing,
    Unknown(i32),
}
```

- All CUDA functions that return a `CUresult` are mapped to this error type.

---

## 7. Implementation Notes

- The crate links dynamically to the CUDA driver library (`libcuda.so` on Linux, `nvcuda.dll` on Windows) and loads symbols at runtime via `dlopen`/`GetProcAddress` to support multiple CUDA versions.  All function pointers are stored in a global `CudaDriver` singleton initialised on first use.
- Context management is thread‑local; the crate does not automatically set the context.  The user (or higher‑level crate) must ensure that a valid context is current before calling any CUDA function.
- `DeviceMemory` and `UnifiedMemory` do not perform any automatic tracking; the user must ensure that the memory is not freed while still in use on a stream.  The type system does not enforce this (as stream lifetimes are dynamic), but the crate documents this requirement clearly.

---

## 8. Testing

- **Device enumeration:** Call `Device::count()` and `Device::get(0)` on a system with a CUDA‑capable GPU; verify success.  On systems without a GPU, skip.
- **Context creation:** Create a context on device 0, make it current, run a simple kernel (e.g., a PTX snippet that increments a buffer), and verify the result.
- **Memory copy:** Allocate device memory, copy from host, copy back, verify data integrity.
- **Stream synchronization:** Create a stream, launch a kernel that fills memory, synchronize the stream, verify the memory is filled.
- **Error handling:** Attempt to launch a kernel with an invalid grid size; expect `LaunchOutOfResources`.
- All tests must run on a machine with an NVIDIA GPU and CUDA driver installed.
