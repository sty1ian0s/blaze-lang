# Phase 5 – Ecosystem Crate: `blaze‑gpu‑compute`

> **Goal:** Specify the `blaze‑gpu‑compute` crate, which provides a unified, high‑level, safe, data‑oriented abstraction over GPU compute backends (CUDA, OpenCL, Vulkan compute, and WebGPU).  It is the recommended way for Blaze applications to offload parallel computations to GPUs without dealing with the low‑level details of each backend.  The crate exposes a single `GpuDevice` type, a `GpuBuffer` for memory management, a `GpuKernel` for compiled kernels, and a `GpuContext` for execution.  All GPU operations carry the `gpu` effect.  The crate is built on top of `blaze‑cuda`, `blaze‑opencl`, `blaze‑vulkan` (compute), and `blaze‑wgpu` (compute), automatically selecting the best available backend at runtime.

---

## 1. Core Concepts

The crate abstracts the common pattern for GPU compute:

- **Device discovery** – list all available GPUs, irrespective of backend, with their capabilities.
- **Memory allocation** – allocate `GpuBuffer<T>` on a device, copy to/from host, and perform zero‑copy where supported by the backend.
- **Kernel compilation and caching** – compile a generic `GpuKernel` from WGSL or SPIR‑V (or PTX via feature flags) once and reuse across multiple invocations.
- **Command submission** – enqueue kernel launches, copies, and synchronisation operations on a `GpuStream`, which may execute asynchronously.
- **Error unification** – all backend errors are mapped to a small, unified `GpuError` type.

The API is designed to be safe by default: buffers and kernels are linear resources, streams enforce ordering and synchronisation, and the runtime automatically tracks memory dependencies across streams.

---

## 2. Device and Context

### 2.1 `GpuDevice`

```
pub struct GpuDevice {
    id: u32,
    name: Text,
    backend: GpuBackend,
    total_memory: usize,
    max_compute_units: u32,
    max_work_group_size: u32,
    max_shared_memory: usize,
    supports_unified_memory: bool,
    supports_fp16: bool,
    supports_fp64: bool,
}
```

- `GpuDevice::enumerate() -> Vec<GpuDevice>` lists all available devices from all registered backends (CUDA, OpenCL, Vulkan, WebGPU).  Backends can be disabled by features.
- `GpuDevice::default() -> Result<GpuDevice, GpuError>` returns the first discrete GPU, or integrated if none, or CPU fallback if nothing is available.

### 2.2 `GpuContext`

```
pub struct GpuContext {
    inner: Box<dyn GpuContextImpl>,
}
```

- Linear; created for a specific `GpuDevice` via `GpuDevice::create_context() -> Result<GpuContext, GpuError>`.
- Methods:
  - `pub fn create_stream(&self) -> GpuStream;`
  - `pub fn create_buffer<T: GpuScalar>(&self, count: usize, usage: BufferUsage) -> GpuBuffer<T>;`
  - `pub fn create_kernel(&self, source: &str, entry: &str) -> Result<GpuKernel, GpuError>;`
  - `pub fn create_event(&self) -> GpuEvent;`
- The context manages per‑device state and kernel compilation.

### 2.3 `GpuBackend`

```
pub enum GpuBackend { Cuda, OpenCL, Vulkan, WebGPU }
```

- Exposed for informational purposes, not needed for typical usage.

---

## 3. `GpuBuffer<T>`

### 3.1 Type

```
pub struct GpuBuffer<T: GpuScalar> {
    ptr: *mut T,
    count: usize,
    context: GpuContext,
}
```

- Linear; `Dispose` frees the GPU memory.  `GpuScalar` trait is implemented for `u8`, `u16`, `u32`, `u64`, `i8`, `i16`, `i32`, `i64`, `f16`, `f32`, `f64`.

### 3.2 Methods

```
impl<T: GpuScalar> GpuBuffer<T> {
    pub fn len(&self) -> usize;
    pub fn as_ptr(&self) -> *const T;
    pub fn as_mut_ptr(&mut self) -> *mut T;
    pub fn copy_from_host(&mut self, data: &[T], stream: &GpuStream) -> Result<(), GpuError>;
    pub fn copy_to_host(&self, data: &mut [T], stream: &GpuStream) -> Result<(), GpuError>;
    pub fn copy_from_device(&mut self, src: &GpuBuffer<T>, stream: &GpuStream) -> Result<(), GpuError>;
    pub fn fill(&mut self, value: T, stream: &GpuStream) -> Result<(), GpuError>;
    pub fn to_host(&self) -> Result<Vec<T>, GpuError>;  // blocks until data is ready
}
```

- All `copy_*` methods return immediately (asynchronous) and insert the command into the given `GpuStream`.  The caller must ensure the host buffer lives until the operation completes, via event synchronisation or by using `to_host` which synchronises.
- `to_host` creates a new stream, submits the copy, and blocks until done.

---

## 4. `GpuKernel`

### 4.1 Type

```
pub struct GpuKernel {
    inner: Box<dyn GpuKernelImpl>,
    entry: Text,
    work_group_size: (u32, u32, u32),
    optimal_block_dim: (u32, u32, u32),
}
```

- Linear; created by `GpuContext::create_kernel(source, entry)`.  The source can be WGSL (default), SPIR‑V (if backend supports it), or PTX (if CUDA is the backend and `cuda` feature enabled).

### 4.2 Methods

```
impl GpuKernel {
    pub fn dispatch(&self, stream: &GpuStream, global_size: (u32, u32, u32), args: &[GpuKernelArg]) -> Result<(), GpuError>;
    pub fn set_work_group_size(&mut self, size: (u32, u32, u32));
    pub fn optimal_block_dim(&self) -> (u32, u32, u32);
}
```

- `dispatch` launches the kernel on the given stream.  The `global_size` is the total number of work‑items; the runtime automatically adjusts the local work‑group size to the optimal value for the device, unless overridden.
- `GpuKernelArg` is an enum:
  - `Buffer(&GpuBuffer<dyn GpuScalarTrait>)` – a pointer to any typed buffer.
  - `ScalarU32(u32)`, `ScalarI32(i32)`, `ScalarF32(f32)`, `ScalarF64(f64)` – small scalar values passed by value.

---

## 5. `GpuStream` and `GpuEvent`

### 5.1 `GpuStream`

```
pub struct GpuStream {
    inner: Box<dyn GpuStreamImpl>,
    context: GpuContext,
}
```

- Linear; created by `GpuContext::create_stream()`.  `Dispose` destroys the stream.
- Methods:
  - `pub fn synchronize(&self) -> Result<(), GpuError>;`
  - `pub fn record_event(&self) -> GpuEvent;`
  - `pub fn wait_event(&self, event: &GpuEvent);`
  - `pub fn enqueue_barrier(&self);`

### 5.2 `GpuEvent`

```
pub struct GpuEvent { inner: Box<dyn GpuEventImpl> }
```

- Linear; `Dispose` releases the event.
- Methods: `synchronize()`, `elapsed_time(&self, other: &GpuEvent) -> f32`.

---

## 6. Error Handling

```
pub enum GpuError {
    NoDevice,
    OutOfMemory,
    InvalidOperation,
    KernelCompilation(Text),
    KernelLaunchFailed(Text),
    StreamExecutionFailed(Text),
    CopyError(Text),
    UnsupportedFeature(Text),
    BackendError(Text),
}
```

- The crate maps all backend‑specific errors to this common set.

---

## 7. Automatic Backend Selection

At runtime, the first call to `GpuDevice::enumerate()` loads all available backends and discovers devices.  The ordering is: CUDA → OpenCL → Vulkan → WebGPU.  If multiple backends provide the same physical GPU, they are merged into a single `GpuDevice` entry with preferences for the native backend (CUDA if NVIDIA, OpenCL if AMD).  This logic is implemented in the crate and is transparent to the user.

---

## 8. Implementation Notes

- The crate uses dynamic dispatch internally (via `dyn GpuContextImpl` etc.) to switch between backends.  The overhead is one vtable lookup per operation; for compute‑intensive work, the bulk cost is in the kernel execution, so this is acceptable.  A higher‑level crate can monomorphise if needed.
- Kernel compilation is cached: the first compilation of a given source/entry pair writes a binary blob (or PTX) to a disk cache (in `~/.blaze/gpu_cache`) and subsequent runs load it directly, avoiding recompilation.
- The crate uses `blaze‑tensor`’s `Device` enum to allow transparent switching between CPU and GPU tensors.  When a `GpuBuffer` is taken from a tensor, the tensor is consumed and the buffer is used directly.

---

## 9. Testing

- **Enumeration:** List devices; at least on a machine with a GPU, verify a device is found with the expected backend.
- **Memory copy:** Allocate a buffer, copy from host, read back, verify data integrity.
- **Kernel execution:** Compile a trivial WGSL kernel that adds two numbers, create buffers for inputs/output, dispatch, copy output back, verify the sum.
- **Multiple streams:** Create two streams, submit overlapping copies, synchronise each, verify order is not required but data is consistent after synchronisation.
- **Error handling:** Use an invalid kernel source, expect `KernelCompilation` error.
- **Cache:** Run a kernel, check that the disk cache is created; re‑run, verify that the cached version is used (faster compilation time).
- All tests must pass on systems with at least one GPU backend.  A mock backend can be used for CI.
