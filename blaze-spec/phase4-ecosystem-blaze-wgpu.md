# Phase 4 – Ecosystem Crate: `blaze‑wgpu`

> **Goal:** Specify the `blaze‑wgpu` crate, which provides a high‑performance, data‑oriented graphics and compute abstraction layer built on top of the WebGPU API (and native equivalents via wgpu‑native).  It integrates with the Blaze actor model and memory management to allow applications to leverage GPU resources for rendering, compute tasks, and data processing.  All GPU operations carry the `gpu` effect (which implies `parallel`).  The crate is designed to be the primary graphics backend for the `blaze‑gui` crate and other visualization libraries.

---

## 1. Core Concepts

The crate provides a safe, linear‑typed interface to the GPU, mapping the WebGPU API into Blaze’s ownership model.  Key types mirror the WebGPU specification:

- **`Instance`** – the entry point to the API, representing a connection to the GPU subsystem.
- **`Adapter`** – a physical GPU device.
- **`Device`** – a logical device with its own command queues, resources, and settings.
- **`Queue`** – a submission queue for command buffers.
- **`Buffer`** – a linear GPU memory region (uniform, storage, index, vertex, etc.).
- **`Texture`** and **`TextureView`** – image data for sampling or rendering.
- **`Sampler`** – texture sampling configuration.
- **`BindGroup`** and **`BindGroupLayout`** – resource binding for shaders.
- **`Pipeline`** (RenderPipeline and ComputePipeline) – compiled shader pipelines.
- **`ShaderModule`** – compiled SPIR‑V or WGSL shader code.
- **`CommandEncoder`** – records GPU commands into command buffers.
- **`Surface`** and **`SwapChain`** – platform‑specific rendering targets.

All GPU resources are linear (must be explicitly disposed) and implement `Dispose`.  Resource creation is asynchronous for large operations (e.g., buffer mapping) but immediate for small metadata.

---

## 2. Instance and Adapter

### 2.1 `Instance`

```
pub struct Instance {
    inner: *mut WGPUInstance,
    backends: Backends,
}
```

- Created by `Instance::new(backends: Backends)`.  `Backends` is a flags type selecting which GPU backends to enable (e.g., `Backends::VULKAN | Backends::DX12 | Backends::METAL`).
- Methods:
  - `pub fn request_adapter(&self, options: &RequestAdapterOptions) -> Result<Adapter, RequestAdapterError>;` – enumerates adapters and returns one matching the options.

### 2.2 `Adapter`

```
pub struct Adapter { /* … */ }
```

- Provides methods to query features, limits, and to request a logical device:
  - `pub fn request_device(&self, desc: &DeviceDescriptor) -> Result<Device, RequestDeviceError>;`
  - `pub fn get_info(&self) -> AdapterInfo;`

### 2.3 `AdapterInfo`

```
pub struct AdapterInfo {
    pub name: Text,
    pub vendor: usize,
    pub device: usize,
    pub device_type: DeviceType,
    pub backend: Backend,
}

pub enum DeviceType { DiscreteGpu, IntegratedGpu, Cpu, Other }
pub enum Backend { Vulkan, Dx12, Metal, Gl }
```

---

## 3. Device and Queue

### 3.1 `Device`

```
pub struct Device { /* … */ }
```

- Linear; created by `Adapter::request_device`.  Owns all GPU memory allocated through it.
- Provides methods to create all resource types: buffers, textures, bind groups, pipelines, shader modules, command encoders, and also for submitting commands.
- `pub fn create_buffer(&self, desc: &BufferDescriptor) -> Buffer;`
- `pub fn create_texture(&self, desc: &TextureDescriptor) -> Texture;`
- `pub fn create_sampler(&self, desc: &SamplerDescriptor) -> Sampler;`
- `pub fn create_bind_group_layout(&self, desc: &BindGroupLayoutDescriptor) -> BindGroupLayout;`
- `pub fn create_bind_group(&self, desc: &BindGroupDescriptor) -> BindGroup;`
- `pub fn create_pipeline_layout(&self, desc: &PipelineLayoutDescriptor) -> PipelineLayout;`
- `pub fn create_render_pipeline(&self, desc: &RenderPipelineDescriptor) -> RenderPipeline;`
- `pub fn create_compute_pipeline(&self, desc: &ComputePipelineDescriptor) -> ComputePipeline;`
- `pub fn create_shader_module(&self, desc: &ShaderModuleDescriptor) -> ShaderModule;`
- `pub fn create_command_encoder(&self, desc: &CommandEncoderDescriptor) -> CommandEncoder;`
- `pub fn default_queue(&self) -> Queue;`
- `pub fn poll(&self, wait: bool);`

### 3.2 `Queue`

```
pub struct Queue { /* … */ }
```

- Methods:
  - `pub fn submit(&self, command_buffers: &[CommandBuffer]) -> SubmissionIndex;`
  - `pub fn write_buffer(&self, buffer: &Buffer, offset: u64, data: &[u8]);`
  - `pub fn write_texture(&self, texture: &Texture, data: &[u8], data_layout: &ImageDataLayout, size: Extent3d);`
  - `pub fn on_submitted_work_done(&self, f: impl FnOnce());`

---

## 4. GPU Resources

### 4.1 `Buffer`

```
pub struct Buffer { /* … */ }
```

- Linear; created by `Device::create_buffer` with a `BufferDescriptor` containing size, usage flags, and mapped‑at‑creation flag.
- Methods:
  - `pub fn map_async(&self, mode: MapMode, callback: impl FnOnce(Result<(), BufferMapError>));`
  - `pub fn get_mapped_range(&self) -> &[u8];`
  - `pub fn get_mapped_range_mut(&self) -> &mut [u8];`
  - `pub fn unmap(&self);`
  - `pub fn size(&self) -> u64;`
  - `pub fn usage(&self) -> BufferUsages;`

### 4.2 `Texture`

```
pub struct Texture { /* … */ }
```

- Types: `TextureDimension` (1D, 2D, 3D), `TextureFormat`, `TextureUsages`.
- Methods: `pub fn create_view(&self, desc: &TextureViewDescriptor) -> TextureView;`

### 4.3 `TextureView` and `Sampler` analogous, with descriptors.

---

## 5. Bind Groups and Layouts

Bind groups connect shader resources to pipeline slots.  Created via `Device` methods with corresponding descriptors:

```
pub struct BindGroupLayout { /* … */ }
pub struct BindGroup { /* … */ }
pub struct PipelineLayout { /* … */ }
```

- `BindGroupLayoutDescriptor` contains a list of `BindGroupLayoutEntry` each specifying binding index, shader stage visibility, and buffer/texture/sampler type.
- `BindGroupDescriptor` references a `BindGroupLayout` and a list of `BindGroupEntry` (which contain the actual buffers/textures).

---

## 6. Shaders and Pipelines

### 6.1 `ShaderModule`

```
pub struct ShaderModule { /* … */ }
```

- Created from SPIR‑V or WGSL source (descriptor with code string).
- `pub fn compilation_info(&self) -> CompilationInfo;` – returns any warnings/errors.

### 6.2 Render Pipeline

```
pub struct RenderPipeline { /* … */ }
```

- `RenderPipelineDescriptor` specifies vertex buffers, primitive topology, depth/stencil state, target color formats, and the `ShaderModule` with entry points.

### 6.3 Compute Pipeline

```
pub struct ComputePipeline { /* … */ }
```

- `ComputePipelineDescriptor` specifies the compute shader entry point.

---

## 7. Command Encoding and Submission

### 7.1 `CommandEncoder`

```
pub struct CommandEncoder { /* … */ }
```

- Created by `Device::create_command_encoder`.  Records commands:
  - `pub fn begin_render_pass(&self, desc: &RenderPassDescriptor) -> RenderPass;`
  - `pub fn begin_compute_pass(&self, desc: &ComputePassDescriptor) -> ComputePass;`
  - `pub fn copy_buffer_to_buffer(&self, src: &Buffer, src_offset: u64, dst: &Buffer, dst_offset: u64, size: u64);`
  - `pub fn copy_texture_to_texture(&self, …);`
  - `pub fn finish(&self) -> CommandBuffer;`

### 7.2 `RenderPass`, `ComputePass`

- Provide methods to set pipeline, bind groups, vertex/index buffers, draw calls (`draw`, `draw_indexed`, `dispatch`), and end the pass.

### 7.3 `CommandBuffer`

- Opaque; submitted via `Queue::submit`.  Not directly usable on its own.

---

## 8. Error Handling

```
pub enum WgpuError {
    RequestAdapter(Text),
    RequestDevice(Text),
    BufferMap(Text),
    Layout(Text),
    Pipeline(Text),
    ShaderCompilation(CompilationInfo),
    Io(std::io::Error),
}
```

- Many operations return `Result<_, WgpuError>`.  Asynchronous operations (like buffer mapping) use callbacks that receive a `Result`.

---

## 9. Implementation Notes

- The crate links to the `wgpu‑native` C library (or uses a pure‑Blaze WebGPU implementation).  The public API is designed to be safe: raw pointers are never exposed; all resources are tracked by linear types and disposed automatically.
- Resource descriptors use `&str` for labels (debugging) and are passed by value.
- The `gpu` effect is automatically applied to all functions that call into this crate, ensuring that they cannot be accidentally called from pure contexts.
- Multi‑threading: `Device` and `Queue` are `Send + Sync`; resources can be shared across actors via `Arc` (when explicitly annotated with `@copy`? Actually, `Arc` is not yet specified in the standard library, but we can rely on `blaze‑async‑std` or a similar crate.  Alternatively, the crate provides its own reference‑counted `GpuRc<T>` for sharing buffers/textures.)

---

## 10. Testing

- **Adapter enumeration:** Request an adapter and verify that `get_info` returns non‑empty name and a valid device type.
- **Device creation:** Request a device with default features, verify it can create a buffer.
- **Buffer copy:** Create two buffers, write data to one, copy to the other, map and read back, compare.
- **Compute shader:** Compile a small WGSL compute shader that adds two numbers, dispatch one workgroup, verify output buffer.
- **Render pipeline:** (Optional, requires a window) Create a surface, render a simple triangle, capture a screenshot, compare CRC.
- **Resource disposal:** Ensure that dropping a `Device` properly releases all GPU memory (tested by observing memory usage after multiple create/drop cycles).
- All tests must pass on a system with a compatible GPU and the `wgpu‑native` library installed.
