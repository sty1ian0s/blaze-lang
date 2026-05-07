# Phase 4 – Ecosystem Crate: `blaze‑vulkan`

> **Goal:** Specify the `blaze‑vulkan` crate, which provides low‑level, unsafe, data‑oriented bindings to the Vulkan graphics and compute API (version 1.3).  This crate is intended as a foundation for higher‑level GPU abstractions such as `blaze‑wgpu` and for applications that require direct Vulkan control.  All Vulkan objects are wrapped in linear types and must be explicitly managed.  The crate is inherently unsafe and should be used only inside `unsafe` blocks or by higher‑level safe wrappers.

---

## 1. Design Philosophy

The bindings closely follow the Vulkan API but with Blaze‑idiomatic naming and with all resource management based on linear types.  Each Vulkan handle (instance, device, queue, command buffer, etc.) is wrapped in a struct that implements `Dispose` to call the appropriate `vkDestroy*` or `vkFree*` function.  Raw C function pointers are exposed as `extern "C"` imports and wrapped with unsafe functions.  The crate does not attempt to hide the complexity of Vulkan; rather, it provides a foundation for safe abstractions.

---

## 2. Core Types

### 2.1 `Instance`

```
pub struct Instance {
    handle: vk::Instance,
    entry: Entry,
}
```

- Created by `Instance::new(create_info: &InstanceCreateInfo) -> Result<Instance, VkError>`.  Loads function pointers (via a Vulkan loader).  `Dispose` calls `vkDestroyInstance`.

### 2.2 `PhysicalDevice`

```
pub struct PhysicalDevice {
    handle: vk::PhysicalDevice,
}
```

- Not a linear resource (can be copied).  Represents a GPU.  Enumerated via `Instance::enumerate_physical_devices() -> Vec<PhysicalDevice>`.
- Provides methods to query properties, features, queue families, surface support, etc.

### 2.3 `Device`

```
pub struct Device {
    handle: vk::Device,
    queues: Map<u32, Queue>,
}
```

- Linear; created by `PhysicalDevice::create_device(create_info: &DeviceCreateInfo) -> Result<Device, VkError>`.  
- `Dispose` calls `vkDestroyDevice`.

### 2.4 `Queue`

```
pub struct Queue {
    handle: vk::Queue,
    family: u32,
}
```

- Not a linear resource (can be copied).  Obtained from `Device`.  Used to submit command buffers.

---

## 3. Memory and Resources

### 3.1 `DeviceMemory`

```
pub struct DeviceMemory {
    handle: vk::DeviceMemory,
    size: vk::DeviceSize,
}
```

- Linear; allocated via `Device::allocate_memory`.  `Dispose` calls `vkFreeMemory`.

### 3.2 `Buffer`

```
pub struct Buffer {
    handle: vk::Buffer,
    memory: DeviceMemory,
    size: vk::DeviceSize,
}
```

- Linear; created by `Device::create_buffer`.

### 3.3 `Image`

```
pub struct Image {
    handle: vk::Image,
    memory: DeviceMemory,
    format: vk::Format,
    extent: vk::Extent3D,
}
```

- Linear; created by `Device::create_image`.

### 3.4 `ImageView`, `Sampler`, `Framebuffer`, `RenderPass`, `Pipeline`, `ShaderModule`, `DescriptorSetLayout`, `DescriptorPool`, `DescriptorSet`, `CommandPool`, `CommandBuffer`, `Fence`, `Semaphore`, `Event`, `QueryPool` — each is a struct with a Vulkan handle and a `Dispose` implementation that calls the appropriate destroy function.

---

## 4. Command Recording

### 4.1 `CommandPool`

```
pub struct CommandPool {
    handle: vk::CommandPool,
}
```

- Created by `Device::create_command_pool`.  All command buffers are allocated from a pool.

### 4.2 `CommandBuffer`

```
pub struct CommandBuffer {
    handle: vk::CommandBuffer,
    level: vk::CommandBufferLevel,
}
```

- Methods:
  - `pub fn begin(&mut self, info: &CommandBufferBeginInfo);`
  - `pub fn end(&mut self);`
  - `pub fn cmd_begin_render_pass(&mut self, info: &RenderPassBeginInfo, contents: SubpassContents);`
  - `pub fn cmd_end_render_pass(&mut self);`
  - `pub fn cmd_bind_pipeline(&mut self, bind_point: PipelineBindPoint, pipeline: &Pipeline);`
  - `pub fn cmd_bind_vertex_buffers(&mut self, first_binding: u32, buffers: &[&Buffer], offsets: &[vk::DeviceSize]);`
  - `pub fn cmd_bind_index_buffer(&mut self, buffer: &Buffer, offset: vk::DeviceSize, index_type: IndexType);`
  - `pub fn cmd_bind_descriptor_sets(&mut self, bind_point: PipelineBindPoint, layout: &PipelineLayout, first_set: u32, descriptor_sets: &[&DescriptorSet], dynamic_offsets: &[u32]);`
  - `pub fn cmd_draw(&mut self, vertex_count: u32, instance_count: u32, first_vertex: u32, first_instance: u32);`
  - `pub fn cmd_draw_indexed(&mut self, index_count: u32, instance_count: u32, first_index: u32, vertex_offset: i32, first_instance: u32);`
  - `pub fn cmd_dispatch(&mut self, group_count_x: u32, group_count_y: u32, group_count_z: u32);`
  - `pub fn cmd_copy_buffer(&mut self, src: &Buffer, dst: &Buffer, regions: &[BufferCopy]);`
  - `pub fn cmd_pipeline_barrier(&mut self, src_stage: PipelineStageFlags, dst_stage: PipelineStageFlags, dependency_flags: DependencyFlags, memory_barriers: &[MemoryBarrier], buffer_memory_barriers: &[BufferMemoryBarrier], image_memory_barriers: &[ImageMemoryBarrier]);`
  - `pub fn cmd_push_constants(&mut self, layout: &PipelineLayout, stages: ShaderStageFlags, offset: u32, constants: &[u8]);`

---

## 5. Synchronization

- `Fence`, `Semaphore`, `Event` — all have `signal`, `wait`, `reset`, `get_status` methods.

---

## 6. Error Handling

```
pub enum VkError {
    OutOfHostMemory,
    OutOfDeviceMemory,
    InitializationFailed,
    DeviceLost,
    MemoryMapFailed,
    LayerNotPresent,
    ExtensionNotPresent,
    FeatureNotPresent,
    IncompatibleDriver,
    TooManyObjects,
    FormatNotSupported,
    FragmentedPool,
    Unknown(i32),
    Loader(Text),
}
```

- All functions that can fail return `Result<_, VkError>`.  The error mapping from Vulkan `VkResult` to `VkError` is exhaustive.

---

## 7. Extensions and Layers

- `InstanceCreateInfo` and `DeviceCreateInfo` allow specifying enabled layers and extensions.  The crate does not hardcode any layer or extension; the user must request them explicitly.

---

## 8. Implementation Notes

- The crate uses dynamic loading (via `vkGetInstanceProcAddr` and `vkGetDeviceProcAddr`) for all functions.  Function pointers are stored in the `Instance` and `Device` structs, allowing multiple versions of Vulkan to be present on the system.
- All handle types are `#[repr(transparent)]` wrappers around raw pointers, reducing ABI incompatibility.
- Memory management for buffers/images is explicit: the user must allocate `DeviceMemory` and bind it to the resource.  The `Buffer` and `Image` types own the memory; on `Dispose`, both the resource and its memory are freed in the correct order.

---

## 9. Testing

- **Instance creation:** Create an instance with the default application info, verify it succeeds.
- **Physical device enumeration:** List physical devices, verify at least one is found (on systems with a GPU).
- **Device creation:** Create a logical device with a graphics queue, verify queue count.
- **Buffer creation:** Allocate a buffer, map memory, write data, unmap, destroy.
- **Command recording:** Allocate a command buffer, begin, end, then reset.
- **Error handling:** Attempt to create a device with an unsupported extension, expect `ExtensionNotPresent`.
- **Resource cleanup:** Ensure that dropping a `Device` and all associated resources does not leak (use Valgrind or similar).

All tests must be run on a system with Vulkan drivers.  For CI environments without a GPU, a mock driver (like SwiftShader) can be used.
