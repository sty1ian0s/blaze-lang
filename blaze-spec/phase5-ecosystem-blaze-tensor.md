# Phase 5 – Ecosystem Crate: `blaze‑tensor`

> **Goal:** Specify the `blaze‑tensor` crate, which provides a generic, high‑performance n‑dimensional array library for Blaze.  It is the foundation for numerical computing, data science, and machine learning, offering a rich set of element‑wise operations, linear algebra, broadcasting, slicing, and device abstraction (CPU and GPU).  The crate is designed to be data‑oriented, with SoA‑aware storage for structured tensors, and to leverage Blaze’s vectorisation, parallelisation, and zero‑cost abstractions.  All operations are pure, enabling deterministic execution and automatic parallelism.

---

## 1. Core Concepts

A **tensor** is a multi‑dimensional array with a static data type and dynamic (or const‑generic) shape.  The crate provides:

- `Tensor<T, N>` – the fundamental type, owning its data and shape.
- `TensorView<'a, T, N>` – a borrowed, possibly strided view of a tensor or a slice of another tensor.
- `Device` – an enum representing the memory location: `Cpu` or `Gpu(GpuDevice)` (from `blaze‑wgpu`).
- `Layout` – describes the memory arrangement (row‑major, column‑major, SoA for structured elements).
- `Broadcast` – automatic shape expansion under arithmetic operations.

The crate uses Blaze’s region allocators for memory management and `Owned` for linear ownership.  Tensors are linear by default (move semantics), with explicit `Clone` where needed.

---

## 2. `Tensor<T, N>`

### 2.1 Type Definition

```
pub struct Tensor<T, const N: usize> {
    data: Owned<[T]>,
    shape: [usize; N],
    strides: [usize; N],
    device: Device,
}
```

- `T` is any `'static + Send + Sync + Numeric` (where `Numeric` is a trait implemented for all primitive numeric types, plus `Complex<f32>` etc.).
- `N` is the rank (number of dimensions).  A 0‑D tensor is represented as rank 1 with shape `[1]`.
- The tensor is linear; it cannot be copied implicitly (unless `T` implements `@copy` and the tensor is trivial, but tensors are not `@copy` because they own heap memory).  `Clone` is implemented.

### 2.2 Construction

```
impl<T: Numeric, const N: usize> Tensor<T, N> {
    pub fn new(shape: [usize; N], device: Device) -> Tensor<T, N>;
    pub fn from_slice(slice: &[T], shape: [usize; N], device: Device) -> Tensor<T, N>;
    pub fn zeros(shape: [usize; N], device: Device) -> Tensor<T, N>;
    pub fn ones(shape: [usize; N], device: Device) -> Tensor<T, N>;
    pub fn full(shape: [usize; N], value: T, device: Device) -> Tensor<T, N>;
    pub fn eye(dim: usize, device: Device) -> Tensor<T, 2>;
    pub fn random(shape: [usize; N], device: Device) -> Tensor<T, N>;  // uniform [0,1)
    pub fn from_fn(shape: [usize; N], f: impl Fn([usize; N]) -> T, device: Device) -> Tensor<T, N>;
    pub fn to_device(&self, device: Device) -> Tensor<T, N>;
    pub fn shape(&self) -> [usize; N];
    pub fn strides(&self) -> [usize; N];
    pub fn numel(&self) -> usize;
    pub fn as_slice(&self) -> &[T];          // CPU only, panics on GPU
    pub fn as_mut_slice(&mut self) -> &mut [T];
}
```

- On GPU, data is stored in a `wgpu::Buffer`; `as_slice` is unavailable.  The `to_device` method copies data between CPU and GPU.

### 2.3 Indexing and Slicing

Indexing returns a `TensorView` with reduced rank.  Slicing with ranges returns a view over the same data without copying.

```
impl<T: Numeric, const N: usize> Tensor<T, N> {
    pub fn get(&self, indices: [usize; N]) -> T;
    pub fn set(&mut self, indices: [usize; N], value: T);
    pub fn slice(&self, ranges: &[Range<usize>; N]) -> TensorView<T, N>;
}
```

- `slice` does not copy; the `TensorView` shares the original data.  The view is linear and cannot outlive the parent tensor (enforced by borrow checker).
- For GPU tensors, slicing creates a new buffer view without data transfer.

---

## 3. `TensorView<'a, T, N>`

A borrowed tensor that may have arbitrary strides and an offset.

```
pub struct TensorView<'a, T, const N: usize> {
    data: &'a [T],           // for CPU; for GPU, it stores a buffer slice reference
    shape: [usize; N],
    strides: [usize; N],
}
```

- Implements all the same arithmetic operations as `Tensor` (via operator traits) but works on borrowed data.  Operations that require ownership (like `to_device`) are not available; the user must copy into a new `Tensor` first.
- `Tensor` implements `Deref<Target = TensorView<T, N>>` so methods can be defined on `TensorView` and used by both.

---

## 4. Arithmetic and Linear Algebra

All arithmetic operators (`+`, `-`, `*`, `/`, etc.) are implemented for `Tensor<T, N>` and `TensorView<'_, T, N>` where both operands have the same shape, or are broadcastable (see below).  Operations are pure and automatically parallelised.

### 4.1 Element‑wise Operations

- `add`, `sub`, `mul`, `div`, `rem`, `neg`, `abs`, `sqrt`, `exp`, `log`, `pow`, `sin`, `cos`, `tan`, etc.
- `relu`, `sigmoid`, `tanh`, `softmax` (common ML activations).

Each operation is implemented via the corresponding operator trait and also as a method on `TensorView` (e.g., `a.relu()`).

### 4.2 Linear Algebra

```
pub fn matmul(a: &TensorView<T, 2>, b: &TensorView<T, 2>) -> Tensor<T, 2>;
pub fn dot(a: &TensorView<T, 1>, b: &TensorView<T, 1>) -> T;
pub fn transpose(&self) -> TensorView<T, N>;   // reverses dimensions
pub fn permute(&self, order: [usize; N]) -> TensorView<T, N>;
pub fn svd(&self) -> (Tensor<T, N>, Tensor<T, N>, Tensor<T, N>);  // for 2-D matrices
pub fn inverse(&self) -> Tensor<T, N>;  // 2-D square matrix
pub fn solve(&self, b: &Tensor<T, N>) -> Tensor<T, N>;
```

- These are implemented using BLAS/LAPACK when targeting CPU (via `blaze‑blas` feature), and custom WGSL compute shaders when targeting GPU.  The pure interface remains the same.

### 4.3 Reductions

```
pub fn sum(&self, axes: Option<&[usize]>) -> Tensor<T, N>;
pub fn mean(&self, axes: Option<&[usize]>) -> Tensor<T, N>;
pub fn max(&self, axes: Option<&[usize]>) -> Tensor<T, N>;
pub fn min(&self, axes: Option<&[usize]>) -> Tensor<T, N>;
pub fn argmax(&self, axis: usize) -> Tensor<i64, N>;
```

- These return a tensor with the specified axes removed (or a scalar if no axes given).  Reductions are automatically parallelised over the non‑reduced dimensions.

---

## 5. Broadcasting

When two tensors of different shapes are used in a binary operation, they are automatically broadcast to a common shape following standard broadcasting rules (Numpy‑style).  The crate implements:

```
pub fn broadcast<const M: usize>(&self, target_shape: [usize; M]) -> TensorView<T, M>;
```

- Broadcasting is done lazily (strides are set to 0 for expanded dimensions) so that no additional memory is allocated.  The compiler can then elide the zero‑stride accesses during optimisation.

---

## 6. Layouts and SoA for Structured Elements

For tensors whose element type is a struct with `@layout(soa)`, the tensor storage can be transformed to SoA internally.  The crate provides a `StructuredTensor<S, const N: usize>` type for this case:

```
pub struct StructuredTensor<S: Struct + Component, const N: usize> {
    fields: Map<String, Tensor<???>>,   // one tensor per field
}
```

- This is used by `blaze‑ecs` for component storage.  For the general `blaze‑tensor` crate, we provide conversion functions between AoS and SoA tensors for struct types annotated with `@layout(soa)`.  The user calls `tensor.to_soa()` to get a `StructuredTensor` and `structured.to_aos()` to convert back.  All operations that don't require cross‑field access run on SoA directly for optimal cache performance.

---

## 7. Random Number Generation

Tensors can be initialised with random values drawn from various distributions using `blaze‑rand`.  The crate exposes:

```
pub fn uniform(shape: [usize; N], low: T, high: T, device: Device) -> Tensor<T, N>;
pub fn normal(shape: [usize; N], mean: T, std: T, device: Device) -> Tensor<T, N>;
```

- These use the thread‑local RNG and are pure (the RNG state is passed as an immutable reference and mutation is done via linear types, or we rely on a deterministic seed for reproducibility).  For `blaze test`, the `--reproducible` flag ensures identical random sequences.

---

## 8. Serialization

```
pub fn save(path: &str, tensor: &Tensor<T, N>) -> Result<(), TensorError>;
pub fn load(path: &str) -> Result<Tensor<T, N>, TensorError>;
```

- Supports raw binary format (header with shape, dtype, then flat data) and NPZ via `blaze‑npy` feature.  The format is simple and portable.

---

## 9. Error Handling

```
pub enum TensorError {
    ShapeMismatch(Text),
    OutOfBounds,
    DeviceMismatch,
    NotSupported(Text),
    LayoutError(Text),
    Io(std::io::Error),
}
```

---

## 10. Testing

- **Basic creation:** Create tensors of various ranks, verify shapes and element values.
- **Arithmetic:** Add tensors, broadcast, check results against manual loops.
- **Matrix multiply:** Multiply two random matrices and compare against a reference implementation.
- **Slicing:** Slice a tensor, modify the original, verify the view sees changes.
- **Broadcasting:** Add a scalar to a matrix, verify it's added to all elements.
- **SoA conversion:** Create a struct with two fields, build a tensor of those structs, convert to SoA, verify fields are contiguous.
- **GPU (if available):** Copy to GPU, perform an operation, copy back, compare with CPU result.
- **Determinism:** Run two identical random generations with `--reproducible`, verify identical tensors.

All tests must pass on CPU and optionally GPU.
