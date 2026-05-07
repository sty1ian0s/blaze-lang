# Phase 5 – Ecosystem Crate: `blaze‑ml`

> **Goal:** Specify the `blaze‑ml` crate, which provides a data‑oriented, zero‑cost machine learning framework built entirely in Blaze.  It supports supervised and unsupervised learning algorithms, automatic differentiation, gradient‑based optimisation, and model serialisation.  The crate leverages Blaze’s effect system, compile‑time metaprogramming, and SIMD/GPU offloading (via `blaze‑wgpu` and `blaze‑cuda` if available) to deliver high performance without sacrificing safety.  All training and inference functions are pure, enabling automatic parallelisation and deterministic benchmarking.

---

## 1. Core Concepts

The crate is built around the following abstractions:

- **`Tensor`** – an n‑dimensional array with a static data type and shape.  Tensors are stored in a region‑aware, SoA‑optimised layout when possible, and can be allocated on CPU or GPU memory.  They implement `Dispose` for automatic memory management.
- **`Module`** – a trait for any differentiable component (similar to layers in a neural network).  A `Module` can have parameters, and supports `forward` (inference) and `backward` (gradient computation) methods that are automatically generated via `@derive(Module)` or custom implementations.
- **`Optimizer`** – a trait for optimisation algorithms (SGD, Adam, etc.) that update module parameters given gradients.
- **`Loss`** – a trait for loss functions that compute a scalar loss from predicted and target tensors, and automatically provide gradients.
- **`Dataset`** and **`DataLoader`** – for loading, shuffling, and batching training data in a data‑oriented pipeline, using Blaze’s sliding windows and parallel iterators.

---

## 2. Tensor

### 2.1 `Tensor<T, N>`

```
pub struct Tensor<T, const N: usize> {
    data: Owned<[T]>,
    shape: [usize; N],
    strides: [usize; N],
    device: Device,
}
```

- `T` is a numeric type (`f32`, `f64`, `i32`, `i64`, etc.).
- `N` is the rank (number of dimensions).  The tensor owns its data via an `Owned` pointer (allocated in a region or on a specific device).
- The `shape` and `strides` define the layout.  By default, the layout is row‑major (C contiguous) for optimal cache usage.
- `Device` is either `Cpu` or `Gpu(Device)` (from `blaze‑wgpu`).

### 2.2 Construction

```
impl<T: Numeric, const N: usize> Tensor<T, N> {
    pub fn new(shape: [usize; N], device: Device) -> Tensor<T, N>;
    pub fn from_slice(slice: &[T], shape: [usize; N], device: Device) -> Tensor<T, N>;
    pub fn zeros(shape: [usize; N], device: Device) -> Tensor<T, N>;
    pub fn ones(shape: [usize; N], device: Device) -> Tensor<T, N>;
    pub fn random(shape: [usize; N], device: Device) -> Tensor<T, N>;
    pub fn to_device(&self, device: Device) -> Tensor<T, N>;
}
```

- `new` creates an uninitialized tensor on the given device.
- `from_slice` copies data from a host slice to the tensor (or to GPU if `device` is GPU).
- `to_device` copies the tensor to another device.

### 2.3 Operations

Tensors support element‑wise arithmetic, matrix multiplication, convolutions, reductions, and slicing via standard operators and methods.  All operations are pure (unless they trigger a GPU kernel launch, which carries `gpu` effect) and are automatically parallelised across the available CPU cores or GPU threads.

```
impl<T: Numeric, const N: usize> Tensor<T, N> {
    pub fn transpose(&self) -> Tensor<T, N>;
    pub fn matmul(&self, other: &Tensor<T, N>) -> Tensor<T, N>;
    pub fn sum(&self) -> T;
    pub fn mean(&self) -> T;
    pub fn relu(&self) -> Tensor<T, N>;
    pub fn broadcast<const M: usize>(&self, target_shape: [usize; M]) -> Tensor<T, M>;
}
```

- The crate uses `@derive` to generate implementations for `Add`, `Mul`, etc. for all tensor types, enabling `a + b` syntax.

### 2.4 Slicing and Indexing

```
pub fn slice(&self, ranges: &[Range<usize>]) -> Tensor<T, N>;
pub fn gather(&self, indices: &Tensor<i32, 2>) -> Tensor<T, N>;
```

---

## 3. Automatic Differentiation

The crate provides reverse‑mode automatic differentiation via a `Grad` wrapper.  Any tensor operation performed inside a `Grad` context is recorded on a computation graph, and gradients can be computed with `backward`.

### 3.1 `Grad<T>`

```
pub struct Grad<T: Numeric> {
    data: Tensor<T, 1>,
    gradient: Option<Tensor<T, 1>>,
    tape: Option<Tape>,
}
```

- A `Grad` wraps a 1‑D tensor of trainable parameters.  Higher‑dimensional parameters are flattened.
- The `tape` records all operations used to compute the loss.
- `fn backward(&mut self) -> Vec<Tensor<T, 1>>` computes the gradient of the recorded loss with respect to every parameter that was wrapped in `Grad`.
- The tape is cleared after `backward`.

### 3.2 `Module` Trait

```
pub trait Module<T: Numeric> {
    fn forward(&self, input: &Tensor<T, 2>) -> Tensor<T, 2>;
    fn parameters(&self) -> Vec<Grad<T>>;
}
```

- `forward` defines the computation from input to output (batch × features).
- `parameters` returns all trainable weights as `Grad` objects so the optimizer can update them and the tape can record them.

### 3.3 Deriving Modules

The `#[derive(Module)]` attribute on a struct containing tensor fields generates a `Module` implementation where `forward` is a sequential application of the struct’s fields (layers).  Custom implementations can be written manually for complex architectures.

---

## 4. Optimizers

### 4.1 `Optimizer` Trait

```
pub trait Optimizer<T: Numeric> {
    fn step(&mut self, parameters: &mut [Grad<T>]);
    fn zero_grad(&mut self);
}
```

### 4.2 Built‑in Implementations

- `SGD { learning_rate: f64 }`
- `Adam { learning_rate: f64, beta1: f64, beta2: f64, epsilon: f64 }`
- `RMSprop { learning_rate: f64, alpha: f64, epsilon: f64 }`

Each optimizer stores state (e.g., momentum buffers) internally, which is allocated in the same region as the model and disposed automatically.

---

## 5. Loss Functions

```
pub trait Loss<T: Numeric> {
    fn forward(&self, predicted: &Tensor<T, 2>, target: &Tensor<T, 2>) -> T;
    fn backward(&self, predicted: &Tensor<T, 2>, target: &Tensor<T, 2>) -> Tensor<T, 2>;
}
```

- Predefined losses: `MseLoss`, `CrossEntropyLoss`, `BinaryCrossEntropyLoss`, etc.
- They are used in the training loop: call `forward` to get a scalar, then `backward` on the predicted tensor (or use the tape with `Grad`).

---

## 6. Data Handling

### 6.1 `Dataset`

```
pub trait Dataset<T: Numeric> {
    fn len(&self) -> usize;
    fn get(&self, index: usize) -> (Tensor<T, 2>, Tensor<T, 2>);  // (input, target)
}
```

- Implementations for in‑memory datasets (e.g., `Vec<(Vec<f32>, Vec<f32>)>`), and for streaming from files via `blaze‑csv` or `blaze‑sql`.

### 6.2 `DataLoader`

```
pub struct DataLoader<D: Dataset<T>, T: Numeric> {
    dataset: D,
    batch_size: usize,
    shuffle: bool,
}
impl<D: Dataset<T>, T: Numeric> Iterator for DataLoader<D, T> {
    type Item = (Tensor<T, 2>, Tensor<T, 2>);
    fn next(&mut self) -> Option<Self::Item> { … }
}
```

- The loader batches and optionally shuffles data; the iteration is parallelised if the dataset iterator is pure and the batches are independent.

---

## 7. Model Serialization

```
pub fn save<M: Module<f32>>(module: &M, path: &str) -> Result<(), MlError>;
pub fn load<M: Module<f32> + 'static>(path: &str) -> Result<M, MlError>;
```

- Serializes all parameters (as `Grad`) to a binary format or safetensors (via `blaze‑serde`).
- The module’s architecture must be known at compile time for deserialization.

---

## 8. Error Handling

```
pub enum MlError {
    ShapeMismatch(Text),
    DeviceMismatch,
    OutOfMemory,
    InvalidParameter(Text),
    LossNotReduced,
    LoadError(Text),
    SaveError(Text),
}
```

---

## 9. Implementation Notes

- The crate’s tensor operations are implemented using Blaze’s auto‑vectorisation and sliding‑window access to SoA layouts.  For CPU tensors, `Tensor<T, 2>` is stored as `[T; M*N]` with row‑major ordering.  For GPU tensors, the data lives in `wgpu::Buffer` objects, and operations launch compute shaders written in WGSL that are pre‑compiled and cached.
- `Grad` uses a linear tape that stores operation nodes as an enum (e.g., `Add`, `Mul`, `Sum`, `Broadcast`).  When `backward` is called, the tape is traversed in reverse, accumulating gradients into the parameter gradients.  This is entirely dynamic (no compile‑time graph building) and linear in the number of operations.
- The `Module` trait and `@derive(Module)` are implemented via `@comptime` macros that inspect the struct’s fields and generate the `forward` and `parameters` methods automatically, ensuring zero‑cost abstraction.
- For distributed training, a future crate (`blaze‑ml‑distributed`) will build on top of `blaze‑raft` (Raft consensus) or `blaze‑gpu‑compute` to synchronize gradients across nodes.

---

## 10. Testing

- **Tensor operations:** Create tensors, perform element‑wise ops, matrix multiply, reductions, and compare with expected results.
- **Autograd:** Define a simple linear function, compute gradient via `backward`, and verify the gradient matches the analytical derivative.
- **Optimizer:** Run a few steps of SGD on a quadratic function and verify convergence.
- **DataLoader:** Create a mock dataset, iterate with shuffling, check batching.
- **Model save/load:** Train a small linear model, save, load, and verify parameters match.
- **GPU (optional):** If a GPU is available, run the same tests on `Device::Gpu` and compare results with CPU (within epsilon).
- **Performance:** Benchmarks for matrix multiplication and convolution against known baselines.

All tests must pass on CPU and optionally on GPU.
