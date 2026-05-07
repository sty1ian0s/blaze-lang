# Blaze Phase 3d – Specialized Library (Optional): GPU Abstractions (`std::gpu`)

> **Goal:** Define the optional `std::gpu` module for GPU programming.  This module provides types and functions for allocating memory on GPUs, transferring data between host and device, compiling and launching compute kernels, and synchronizing execution.  Because GPU support is highly platform‑dependent, this module is optional and only required on targets that support GPU offloading.  All GPU operations carry the `gpu` effect (which implies `parallel`).

---

## 1. GPU Device Selection

### 1.1 `Device`

```
pub struct Device { /* opaque */ }
```

- Represents a physical GPU device.  Multiple devices may be available on a system.

- **Methods:**
```
impl Device {
    pub fn count() -> usize;
    pub fn get(index: usize) -> Option<Device>;
    pub fn name(&self) -> &str;
    pub fn memory_total(&self) -> usize;
    pub fn memory_free(&self) -> usize;
}
```

- `count()` returns the number of GPUs in the system.
- `get(index)` returns the device at that index, or `None` if out of range.
- `name()` returns a human‑readable name (e.g., `"NVIDIA GeForce RTX 4090"`).
- `memory_total` / `memory_free` report memory capacity and available memory in bytes.

---

## 2. GPU Memory

### 2.1 `GpuBuffer<T>`

```
pub struct GpuBuffer<T> {
    device: Device,
    ptr: *mut T,
    len: usize,
}
```

- An array of `T` allocated on a specific GPU device.  This type is **linear**; it must be explicitly freed using `dispose`, which releases the GPU memory.  It is not `@copy`.

- **Constructors:**
```
impl<T> GpuBuffer<T> {
    pub fn new(device: &Device, count: usize) -> Result<GpuBuffer<T>, Error>;
    pub unsafe fn from_raw(device: &Device, ptr: *mut T, len: usize) -> GpuBuffer<T>;
}
```

- **Methods:**
```
impl<T> GpuBuffer<T> {
    pub fn len(&self) -> usize;
    pub fn as_ptr(&self) -> *const T;
    pub fn as_mut_ptr(&mut self) -> *mut T;
    pub fn copy_from_host(&mut self, host_data: &[T]) -> Result<(), Error>;
    pub fn copy_to_host(&self, host_data: &mut [T]) -> Result<(), Error>;
}
```

- `copy_from_host` transfers data from the host (CPU) memory to the GPU buffer.  The host slice length must equal the buffer length.
- `copy_to_host` transfers data back to the host.
- `unsafe` raw pointer methods allow interoperability with kernel launch APIs.

### 2.2 `Dispose` for `GpuBuffer`

```
impl<T> Dispose for GpuBuffer<T> {
    fn dispose(&mut self);
}
```

- Frees the GPU memory and resets the buffer to a non‑allocated state.

---

## 3. GPU Kernels

Kernels are functions written in a subset of Blaze (or an external shading language) and compiled at build time to GPU‑specific binaries.  The `std::gpu` module provides the runtime API to launch kernels.

### 3.1 `Kernel`

```
pub struct Kernel { /* opaque */ }
```

- Represents a compiled GPU kernel.  Kernels are typically generated from Blaze functions annotated with `@gpu_kernel` (a separate attribute), but the runtime API only deals with pre‑compiled binary kernels.

- **Methods:**
```
impl Kernel {
    pub fn from_ptx(device: &Device, ptx: &str, entry: &str) -> Result<Kernel, Error>;
    pub fn launch(&self, args: &[KernelArg], grid: (u32, u32, u32), block: (u32, u32, u32)) -> Result<(), Error>;
}
```

- `from_ptx` loads a kernel from a PTX (or SPIR‑V) binary string and specified entry point name.  Returns an error if compilation fails.
- `launch` executes the kernel on the GPU with the given grid and block dimensions.  `KernelArg` is a type representing a single argument (pointer to a `GpuBuffer`, integer, or float).

### 3.2 `KernelArg`

```
pub enum KernelArg<'a> {
    Buffer(&'a dyn GpuBufferAny),
    Int32(i32),
    Float32(f32),
    // … more types as needed
}
```

- Represents a single argument passed to a kernel launch.  `GpuBufferAny` is a supertrait for any typed buffer.

---

## 4. Synchronization

### 4.1 `sync`

```
pub fn sync() -> Result<(), Error>;
```

- Blocks the CPU until all previously launched GPU operations on the current device have completed.  Returns an error if a kernel or memory operation failed.

### 4.2 Events (Optional)

If fine‑grained scheduling is needed, the module may provide `Event` types.  For the 1.0 version, only `sync()` is required.

---

## 5. Error Handling

```
pub struct Error(GpuErrorKind, Text);

pub enum GpuErrorKind {
    OutOfMemory,
    InvalidDevice,
    KernelCompilationFailed,
    LaunchFailed,
    TransferFailed,
    Timeout,
}
```

- Carries a machine‑readable error kind and human message.

---

## 6. Implementation Notes

- On platforms with CUDA, the module uses the CUDA driver API.  On platforms with Vulkan, it may use Vulkan compute.  On platforms without GPU support, all functions return `ErrorKind::InvalidDevice` or panics, depending on the security policy.
- The `gpu` effect marker ensures that functions calling GPU operations cannot be accidentally called from `pure` or `alloc`‑only contexts.
- Kernel compilation from Blaze source is not part of this module; that is the responsibility of a separate `blaze‑gpu‑compiler` tool (in the ecosystem).  This module only provides runtime loading of compiled binaries.

---

## 7. Testing

GPU tests require a physical GPU and a functioning driver; they are optional but strongly recommended on supported hardware.

- **Device enumeration:** Verify that at least one GPU is found (if hardware present).
- **Memory allocation:** Create a `GpuBuffer<i32>`, copy host data to it, copy back, and verify the data integrity.
- **Kernel launch:** Write a minimal PTX kernel (or use a precompiled simple kernel) that adds two arrays, launch it via `Kernel::launch`, and verify the result.
- **Synchronization:** Ensure that after `launch`, a `sync()` call waits for completion, after which host memory contains the correct results.

All tests must be guarded by `@cfg(target_has_gpu = "true")` or a similar compile‑time flag and skipped on CPU‑only builds.
