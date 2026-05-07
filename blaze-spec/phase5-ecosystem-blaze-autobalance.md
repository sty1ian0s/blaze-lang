# Phase 5 – Ecosystem Crate: `blaze‑autobalance`

> **Goal:** Extend Blaze’s automatic parallelism to heterogeneous hardware (multi‑CPU, GPU, NUMA, and distributed nodes).  The crate provides a `#![no_std]`‑compatible scheduler that treats every compute resource—CPU core, GPU stream, remote actor, or custom accelerator—as a `ComputeDevice`.  Pure loops, actor workloads, and futures are automatically partitioned and load‑balanced across devices without manual intervention.  The system honours the `@energy_aware` hint and the existing effect model (`io`, `gpu`, `hal`, `parallel`).  All decisions remain deterministic and reproducible under the `--reproducible` flag.

---

## 1. Core Concepts

- **ComputeDevice** – an abstract handle to any computation resource: a CPU core, a GPU stream, a NUMA node, a remote Node, or a custom accelerator.  Devices expose their capabilities (throughput, memory, latency, energy profile).
- **Scheduler** – a global, work‑stealing scheduler that owns a pool of devices.  It accepts `Task` objects, partitions them automatically, and executes them on the best available device.
- **Task** – a unit of work: a pure function plus its input data (structured as a `Workload`).  Tasks are produced by the compiler when a pure loop is annotated with `@balance` or when the runtime detects an actor that could be migrated.
- **Workload** – a description of parallel work: a pure closure, an array of input chunks, and an optional reduction.  The scheduler can split workloads across devices and reassemble results.
- **Placement Hints** – optional attributes `@device`, `@numa`, `@remote` that suggest where a task should run; the scheduler respects these but may override for load balancing.

All types are linear where they own resources, and `@copy` when they are lightweight handles.

---

## 2. `ComputeDevice`

### 2.1 Enum

```
pub enum ComputeDevice {
    Cpu(CpuCore),
    Gpu(GpuDevice),
    NumaNode(NumaNode),
    RemoteNode(RemoteNode),
    Custom(Box<dyn Compute>),
}
```

- `CpuCore` represents a specific CPU core (logical or physical).  This is obtained from the runtime’s thread pool.
- `GpuDevice` wraps a `wgpu::Device` or `cuda::Device` (from `blaze‑wgpu`, `blaze‑cuda`).  The scheduler can allocate multiple streams per device.
- `NumaNode` represents a NUMA node on multi‑socket systems; it is a collection of CPU cores and local memory.
- `RemoteNode` is a network‑reachable Blaze runtime instance (another process or machine) identified by a `NodeId`.  Communication uses the actor protocol.
- `Custom` allows integration of FPGA, DSPs, etc., via the `Compute` trait.

### 2.2 Capabilities

Every `ComputeDevice` provides a `DeviceInfo` struct:

```
pub struct DeviceInfo {
    pub name: Text,
    pub kind: DeviceKind,
    pub compute_units: u32,
    pub memory_bytes: u64,
    pub max_work_group_size: u32,
    pub is_low_power: bool,       // e.g., efficiency cores
    pub performance_relative: f64, // relative to a baseline CPU core
    pub numa_node: Option<u32>,
}
```

- `DeviceKind` is `Cpu`, `Gpu`, `Accelerator`, `Remote`.

The scheduler uses this information plus runtime load metrics to make placement decisions.

---

## 3. Scheduler

### 3.1 Global Scheduler

The crate installs a global `Scheduler` singleton that replaces the default CPU‑only work‑stealing pool.  It is created at application startup via a configuration:

```
pub struct SchedulerConfig {
    pub use_gpus: bool,
    pub use_distributed: bool,
    pub max_gpu_streams: u32,
    pub remote_nodes: Vec<RemoteNode>,
    pub balancing_policy: BalancingPolicy,
    pub profiling_window: Duration,
}
```

- If no explicit configuration is provided, the scheduler uses all CPU cores and any GPU discovered via `blaze‑wgpu` (if the `gpu` feature is enabled).  Distributed nodes must be explicitly added.

### 3.2 Task Submission

The compiler emits calls to the scheduler for loops annotated with `@balance`, or when the `--auto-balance` flag is passed to `blaze build` (which treats all pure loops as candidates for heterogeneous balancing).  The runtime also submits actor mailboxes as tasks when an actor is idle and could be migrated.

```
impl Scheduler {
    pub fn submit(&self, task: Task) -> JoinHandle<()>;
    pub fn submit_chunked(&self, workload: Workload) -> JoinHandle<()>;
}
```

- `Task` contains a pure closure (`fn() / pure`) that can be sent between threads and devices.
- `Workload` bundles an array of input chunks, a pure mapping closure, and an optional reduction closure; the scheduler partitions the chunks across devices and automatically reassembles the result.

### 3.3 Load Balancing Policy

The scheduler uses a **work‑stealing + cost‑model** approach.  Each device maintains a task queue.  When a device becomes idle, it steals from the queued workload of another device, prioritizing tasks whose `DeviceInfo` matches the device’s capabilities.  The cost model uses the relative performance and memory requirements of the task (extracted from the closure’s known size and the input data size) to decide whether to move the task to a GPU or keep it on CPU.

- The policy can be overridden: `BalancingPolicy::CpuOnly`, `GpuFirst`, `DistributedFirst`, `Adaptive` (default).

---

## 4. Integration with the Compiler

### 4.1 `@balance` Attribute

When a loop or function is annotated with `@balance`, the compiler generates code that packages the loop body as a `Workload` and submits it to the global scheduler instead of using the CPU‑only parallel pool.

Example:

```
#[balance]
fn compute_all(data: &[f64]) -> f64 {
    let mut sum = 0.0;
    for &x in data {
        sum += x.sqrt();     // pure loop, auto‑balanced
    }
    sum
}
```

The compiler splits `data` into chunks respected by device memory, creates a `Workload` with a mapping closure (the loop body) and a reduction closure (the summation), then submits it.  The scheduler may execute parts on GPU, parts on CPU, and even on remote nodes.

### 4.2 `--auto-balance` Flag

When compiling with `blaze build --auto-balance`, the compiler automatically treats all pure loops (empty effect set) as candidates for heterogeneous balancing.  This is **opt‑in** because it may introduce overhead for very small loops; the scheduler’s profiling window helps decide at runtime whether to use the full heterogeneous path.

### 4.3 Effect System Changes

None required.  Tasks that are sent to a GPU will automatically acquire the `gpu` effect, but the scheduler ensures they are only submitted to devices that support that effect.  The compiler infers the required effect set for each task from the closure’s annotated environment.

---

## 5. Remote Execution

When `RemoteNode`s are configured, the scheduler can serialize a `Workload` (input data + closure) and send it to a remote runtime via the actor protocol.  The remote runtime executes the workload on its own local scheduler and returns the result.  This is transparent: the local `JoinHandle` becomes a future that is fulfilled when the remote work completes.

- The transport uses the same `blaze‑serde` serialization as other actor messages; closures are serialized as compiled WASM or LLVM bitcode, which is then JIT‑compiled on the remote if the architecture matches.
- Security: the remote node must be explicitly trusted; the feature is only enabled with `remote` feature flag.

---

## 6. NUMA‑Aware Balancing

By default, the scheduler queries the operating system for NUMA topology and groups CPU cores by NUMA node.  Workloads that are memory‑intensive are preferentially scheduled on the NUMA node where their input data resides.  The user can also pin data structures to a NUMA node via `@numa` attribute on type definitions, which causes allocations to be placed in that node’s memory.

---

## 7. Energy‑Aware Balancing

When the `@energy_aware` context is active (or global flag), the scheduler prefers low‑power devices (efficiency cores, integrated GPU) for tasks whose estimated energy‑to‑completion is lower.  It can also throttle high‑performance cores to reduce power spikes.

---

## 8. Determinism and Reproducibility

Even with heterogeneous balancing, the system remains deterministic under the `--reproducible` flag.  The scheduler seeds its PRNG from the same global seed, and all device selections are then deterministic based on the fixed seed.  GPU kernel executions are presumed deterministic (the Blaze GPU crate enforces deterministic kernels when `--reproducible` is passed).  Remote execution requires the remote nodes to also operate under `--reproducible` with the same seed.

---

## 9. Error Handling

```
pub enum BalanceError {
    NoDevice,
    OutOfMemory,
    RemoteNodeUnavailable,
    RemoteExecutionFailed(Text),
    SerializationFailed(Text),
    DeserializationFailed(Text),
    IncompatibleDevice,
}
```

---

## 10. Implementation Notes

- The crate uses dynamic dispatch for `ComputeDevice` (a small vtable) but only during scheduling decisions.  The hot path—task execution—remains static and monomorphised because the closure is generic and inlined on each device.
- The global scheduler is initialised lazily in a `OnceLock`.  If the crate is not linked, Blaze falls back to the default CPU‑only scheduler with no overhead.
- The crate depends on `blaze‑wgpu`, `blaze‑cuda`, `blaze‑opencl`, `blaze‑gpu‑compute` (for GPU backends), `blaze‑serde` (for remote serialization), and the core runtime.  All GPU backend dependencies are optional via feature flags.
- For remote execution, the crate uses the `spawn_on` underlying actor infrastructure; the scheduler spawns a temporary actor on the remote node that runs the workload and sends back the result.

---

## 11. Testing

- **CPU‑only balancing:** Set `BalancingPolicy::CpuOnly`, submit a workload with many chunks, verify all CPU cores are utilised.
- **GPU balancing:** Enable GPUs, submit a vector‑add workload; verify data is transferred to GPU and result is correct.
- **NUMA awareness:** (Platform‑specific) Allocate large arrays, verify that prefetching to a local NUMA node reduces access latency.
- **Remote balancing:** Set up two local runtimes, configure remote nodes, submit a distributed sum; verify result equals local sum.
- **Determinism:** Run the same balanced task twice with `--reproducible`; verify identical outputs.
- **Error cases:** Unplug a remote node mid‑execution; verify scheduler retries on another node and eventually returns a `RemoteNodeUnavailable` error.

All tests must pass on supported platforms and with the appropriate optional features enabled.

---

*End of `blaze‑autobalance` specification.*
