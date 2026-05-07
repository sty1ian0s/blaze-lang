# Phase 5 – Ecosystem Crate: `blaze‑opencl`

> **Goal:** Specify the `blaze‑opencl` crate, which provides low‑level, unsafe, data‑oriented bindings to the OpenCL heterogeneous computing API (version 3.0).  It is intended as a foundation for higher‑level GPU abstractions such as `blaze‑tensor` (when targeting OpenCL‑compatible devices) and for applications that require direct control over OpenCL kernels.  All OpenCL resources are wrapped in linear types and carry the `gpu` effect.  This crate is inherently unsafe and should be used only inside `unsafe` blocks or by higher‑level safe wrappers.

---

## 1. Design Philosophy

The OpenCL bindings provide direct access to the OpenCL platform and runtime API.  Each OpenCL object (platform, device, context, command queue, buffer, image, program, kernel, event) is wrapped in a Blaze struct that implements `Dispose` to call the corresponding `clRelease*` function.  All functions that modify the OpenCL state machine are marked `unsafe`.  The bindings are designed to be lightweight and data‑oriented, exposing the minimal set of calls required for compute, while allowing the user to manage contexts, queues, and synchronisation explicitly.  Higher‑level crates will build on top of these bindings.

---

## 2. Core Types

### 2.1 `Platform`

```
pub struct Platform { id: cl_platform_id }
```

- Represents an OpenCL platform (e.g., a vendor’s OpenCL implementation).  Obtained via `Platform::enumerate() -> Vec<Platform>` (safe).
- Methods:
  - `pub fn name(&self) -> Result<Text, OpenCLError>;`
  - `pub fn vendor(&self) -> Result<Text, OpenCLError>;`
  - `pub fn version(&self) -> Result<Text, OpenCLError>;`
  - `pub fn devices(&self, device_type: DeviceType) -> Result<Vec<Device>, OpenCLError>;`

### 2.2 `Device`

```
pub struct Device { id: cl_device_id }
```

- Represents an OpenCL device (GPU, CPU, accelerator, etc.).  Not linear; can be copied.  Obtained via `Platform::devices`.
- Query methods for device properties: `name`, `vendor`, `type`, `max_compute_units`, `max_work_group_size`, `global_mem_size`, `local_mem_size`, `max_clock_frequency`, `extensions`.

### 2.3 `Context`

```
pub struct Context { inner: cl_context }
```

- Linear; created by `Context::new(devices: &[&Device]) -> Result<Context, OpenCLError>`.  `Dispose` calls `clReleaseContext`.
- The context manages a set of devices; all memory and command queues are created within a context.

### 2.4 `CommandQueue`

```
pub struct CommandQueue { inner: cl_command_queue }
```

- Linear; created by `Context::create_queue(device: &Device, properties: QueueProperties) -> Result<CommandQueue, OpenCLError>`.  `Dispose` calls `clReleaseCommandQueue`.
- `QueueProperties` includes bits for out‑of‑order execution and profiling.
- Methods: `finish()`, `flush()`.

### 2.5 `Event`

```
pub struct Event { inner: cl_event }
```

- Linear; created by various enqueue operations (may also be returned).  `Dispose` calls `clReleaseEvent`.
- Methods: `wait()`, `get_command_execution_status()`, `get_profiling_info()`.

---

## 3. Memory Objects

### 3.1 `Buffer`

```
pub struct Buffer<T> {
    inner: cl_mem,
    size: usize,
    phantom: PhantomData<T>,
}
```

- Linear; created by `Context::create_buffer(flags: MemFlags, size: usize, host_ptr: Option<*const u8>) -> Result<Buffer<T>, OpenCLError>`.  `Dispose` calls `clReleaseMemObject`.
- `MemFlags` include `READ_WRITE`, `WRITE_ONLY`, `READ_ONLY`, `USE_HOST_PTR`, `ALLOC_HOST_PTR`, `COPY_HOST_PTR`.
- Methods:
  - `pub fn size(&self) -> usize;`
  - `pub unsafe fn enqueue_write(&self, queue: &CommandQueue, blocking: bool, offset: usize, data: &[T], events_wait: &[&Event]) -> Result<Event, OpenCLError>;`
  - `pub unsafe fn enqueue_read(&self, queue: &CommandQueue, blocking: bool, offset: usize, data: &mut [T], events_wait: &[&Event]) -> Result<Event, OpenCLError>;`
  - `pub unsafe fn enqueue_fill(&self, queue: &CommandQueue, pattern: &T, offset: usize, size: usize, events_wait: &[&Event]) -> Result<Event, OpenCLError>;`

### 3.2 `Image`

```
pub struct Image {
    inner: cl_mem,
    format: ImageFormat,
    width: usize,
    height: usize,
    depth: usize,
}
```

- Linear; created by `Context::create_image2d`/`create_image3d`.  `Dispose` calls `clReleaseMemObject`.
- Enables texture‑like access in kernels.  Methods for enqueueing read/write to/from host.

### 3.3 `Sampler`

```
pub struct Sampler { inner: cl_sampler }
```

- Linear; `Dispose` calls `clReleaseSampler`.  Used by kernels to sample images.

---

## 4. Programs and Kernels

### 4.1 `Program`

```
pub struct Program { inner: cl_program }
```

- Linear; created by `Context::create_program_with_source(source: &str) -> Result<Program, OpenCLError>` or `create_program_with_binary(… )`.  `Dispose` calls `clReleaseProgram`.
- Methods:
  - `pub fn build(&self, devices: &[&Device], options: Option<&str>) -> Result<(), OpenCLError>;`
  - `pub fn get_build_log(&self, device: &Device) -> Result<Text, OpenCLError>;`
  - `pub fn create_kernel(&self, name: &str) -> Result<Kernel, OpenCLError>;`

### 4.2 `Kernel`

```
pub struct Kernel { inner: cl_kernel }
```

- Linear; created by `Program::create_kernel`.  `Dispose` calls `clReleaseKernel`.
- Methods to set kernel arguments (type‑safe wrappers for `clSetKernelArg`):
  - `pub unsafe fn set_arg<T: Sized>(&self, index: u32, arg: &T) -> Result<(), OpenCLError>;`
  - `pub unsafe fn set_arg_buffer<T>(&self, index: u32, buffer: &Buffer<T>) -> Result<(), OpenCLError>;`
  - `pub unsafe fn set_arg_image(&self, index: u32, image: &Image) -> Result<(), OpenCLError>;`
  - `pub unsafe fn set_arg_sampler(&self, index: u32, sampler: &Sampler) -> Result<(), OpenCLError>;`
- Launch:
  - `pub unsafe fn enqueue_nd_range(&self, queue: &CommandQueue, global_work_size: &[usize], local_work_size: Option<&[usize]>, events_wait: &[&Event]) -> Result<Event, OpenCLError>;`

---

## 5. Synchronization

- `CommandQueue::finish()` blocks until all previously enqueued commands have completed.
- `Event::wait()` blocks on a single event.
- `Event::wait_for_events(events: &[&Event])` waits for all specified events.
- `CommandQueue::enqueue_barrier()` inserts a barrier (waits for all previous commands in the queue).
- `CommandQueue::enqueue_marker()` returns an event that can be used to signal completion.

---

## 6. Error Handling

```
pub enum OpenCLError {
    DeviceNotFound,
    DeviceNotAvailable,
    CompilerNotAvailable,
    OutOfHostMemory,
    OutOfResources,
    OutOfMemory,
    InvalidValue,
    InvalidDevice,
    InvalidBufferSize,
    InvalidGlobalWorkSize,
    InvalidWorkGroupSize,
    InvalidWorkItemSize,
    InvalidProgram,
    InvalidProgramExecutable,
    InvalidKernel,
    InvalidKernelArgs,
    InvalidCommandQueue,
    InvalidContext,
    InvalidEvent,
    InvalidOperation,
    BuildProgramFailure,
    MapFailure,
    UnmapFailure,
    ProfilingInfoNotAvailable,
    Unknown(i32),
    Loader(Text),
}
```

- All functions that can fail return `Result<_, OpenCLError>`.  The error mapping from OpenCL `cl_int` to `OpenCLError` is exhaustive.

---

## 7. Extensions and Feature Queries

The crate provides helper functions to query available extensions and to load function pointers for extension functions (e.g., `clCreateCommandQueueWithProperties`).  Users can request specific extensions via `Context::create_with_properties` and query device `extensions`.

---

## 8. Implementation Notes

- The crate dynamically loads the OpenCL library at runtime (via `dlopen`/`LoadLibrary`) and resolves all function pointers.  This allows the same binary to run on systems with or without an OpenCL ICD, failing gracefully at runtime.
- All handle types are `#[repr(transparent)]` wrappers around `cl_*` pointers.  `Dispose` calls the appropriate `clRelease*` only if the handle is non‑null.
- The type parameter `T` on `Buffer<T>` is purely for documentation and type‑safe kernel argument setting.  The size is in bytes, and no type checking is performed by OpenCL.  The `set_arg<T>` method uses `sizeof::<T>()` to pass the size.
- The crate does not provide a safe abstraction for memory management; the user must ensure that kernels do not access freed buffers.  The linear type system ensures that the buffer is freed only once, but does not prevent use‑after‑free across queues (this is the responsibility of the user or a higher‑level crate that tracks queue ordering).

---

## 9. Testing

- **Platform and device enumeration:** Call `Platform::enumerate()` and `Platform::devices()`, verify that at least one platform and device is found on a system with OpenCL support.
- **Context and queue creation:** Create a context and queue, verify they can be created and released without error.
- **Program compilation:** Create a simple OpenCL kernel (e.g., a vector add), build it, verify build log.
- **Kernel launch:** Create buffers, set arguments, enqueue a kernel, read back results, verify correctness.
- **Events and synchronization:** Enqueue a kernel with an event, wait for event, verify command has executed.
- **Error handling:** Provide an invalid kernel source, expect `BuildProgramFailure`.
- All tests must run on a system with an OpenCL‑capable device (or a mock OpenCL library for CI).  A software OpenCL implementation like POCL can be used.
