# Phase 4 – Ecosystem Crate: `blaze‑opengl`

> **Goal:** Specify the `blaze‑opengl` crate, which provides low‑level, unsafe, data‑oriented bindings to the OpenGL 4.6 Core Profile.  This crate is intended as a foundation for higher‑level rendering libraries (e.g., a future `blaze‑renderer` or internal use by `blaze‑gui` when targeting OpenGL).  All OpenGL objects are wrapped in linear types and must be explicitly managed.  The crate is inherently unsafe because OpenGL operations rely on a global state machine; the bindings are designed to be called only inside `unsafe` blocks or wrapped by safe higher‑level code.

---

## 1. Design Philosophy

OpenGL is an imperative, state‑based graphics API.  The bindings closely mirror the C API but with Blaze‑idiomatic naming, using enums for constants and bit‑flags for mask parameters.  Every OpenGL object (texture, buffer, shader, program, etc.) is represented as a linear struct that, on `Dispose`, calls the corresponding `glDelete*` function.  Functions that query or modify global state require an active context, which must be managed externally (e.g., via `blaze‑glfw` or platform‑specific code).  The crate does not provide context creation; it assumes a valid context is current in the calling thread.

---

## 2. Core Types

### 2.1 Object Handles

Each OpenGL object type is a thin wrapper around a `GLuint` name.  All implement `Dispose` to delete the object and are not `@copy`.

```
pub struct Buffer { name: GLuint }
pub struct Texture { name: GLuint, target: TextureTarget }
pub struct VertexArray { name: GLuint }
pub struct Shader { name: GLuint, shader_type: ShaderType }
pub struct Program { name: GLuint }
pub struct Renderbuffer { name: GLuint }
pub struct Framebuffer { name: GLuint }
pub struct Query { name: GLuint, query_type: QueryType }
pub struct Sampler { name: GLuint }
pub struct Sync { ptr: *mut c_void }   // GLsync is a pointer
```

All are linear and moveable.  `Dispose` calls `glDelete*` for the given name, which is idempotent (ignored if name is 0).

### 2.2 `GLuint` and Basic Types

```
pub type GLuint = u32;
pub type GLint = i32;
pub type GLsizei = i32;
pub type GLfloat = f32;
pub type GLdouble = f64;
pub type GLboolean = bool;
pub type GLenum = u32;
pub type GLbitfield = u32;
pub type GLchar = u8;
```

---

## 3. Object Creation and Management

Each object type provides a static constructor and methods.  Example for buffers:

### 3.1 `Buffer`

```
impl Buffer {
    pub fn new() -> Result<Buffer, GLError>;
    pub fn bind(&self, target: BufferTarget);
    pub fn buffer_data(target: BufferTarget, size: GLsizeiptr, data: *const c_void, usage: Usage);
    pub fn set_sub_data(target: BufferTarget, offset: GLintptr, size: GLsizeiptr, data: *const c_void);
    pub fn map(target: BufferTarget, access: Access) -> *mut c_void;
    pub fn unmap(target: BufferTarget) -> bool;
}
```

- `bind` sets the active buffer for the given target.
- Other functions are static because they operate on the currently bound buffer (OpenGL style).  A higher‑level wrapper may provide methods that bind and then operate.

### 3.2 `Texture`

```
pub enum TextureTarget { Texture1D, Texture2D, Texture3D, TextureCubeMap, … }

impl Texture {
    pub fn new(target: TextureTarget) -> Result<Texture, GLError>;
    pub fn bind(&self);
    pub fn tex_image2d(target: TextureTarget, level: GLint, internal_format: InternalFormat, width: GLsizei, height: GLsizei, border: GLint, format: Format, pixel_type: PixelType, pixels: *const c_void);
    pub fn generate_mipmap(target: TextureTarget);
    // … other texImage* variants
}
```

### 3.3 `Shader` and `Program`

```
pub enum ShaderType { Vertex, Fragment, Geometry, Compute, … }

impl Shader {
    pub fn new(shader_type: ShaderType) -> Result<Shader, GLError>;
    pub fn source(&self, source: &str);
    pub fn compile(&self);
    pub fn get_compile_status(&self) -> bool;
    pub fn get_info_log(&self) -> Text;
}

impl Program {
    pub fn new() -> Result<Program, GLError>;
    pub fn attach_shader(&self, shader: &Shader);
    pub fn link(&self);
    pub fn get_link_status(&self) -> bool;
    pub fn get_info_log(&self) -> Text;
    pub fn use_program(&self);
    pub fn get_uniform_location(&self, name: &str) -> Option<UniformLocation>;
    pub fn uniform_1i(loc: UniformLocation, v: i32);
    pub fn uniform_1f(loc: UniformLocation, v: f32);
    // … many uniform setters
}
pub struct UniformLocation(GLint);
```

### 3.4 `VertexArray`

```
impl VertexArray {
    pub fn new() -> Result<VertexArray, GLError>;
    pub fn bind(&self);
    pub fn vertex_attrib_pointer(index: u32, size: i32, attr_type: Type, normalized: bool, stride: GLsizei, pointer: usize);
    pub fn enable_attrib(index: u32);
}
```

### 3.5 `Framebuffer` and `Renderbuffer`

```
impl Framebuffer {
    pub fn new() -> Result<Framebuffer, GLError>;
    pub fn bind(target: FramebufferTarget, fbo: Option<&Framebuffer>);
    pub fn framebuffer_texture2d(target: FramebufferTarget, attachment: Attachment, textarget: TextureTarget, texture: &Texture, level: GLint);
    pub fn check_status(target: FramebufferTarget) -> FramebufferStatus;
}

impl Renderbuffer {
    pub fn new() -> Result<Renderbuffer, GLError>;
    pub fn bind(target: RenderbufferTarget);
    pub fn renderbuffer_storage(target: RenderbufferTarget, internal_format: InternalFormat, width: GLsizei, height: GLsizei);
}
```

### 3.6 `Query`

```
pub enum QueryType {
    SamplesPassed,
    Timestamp,
    // …
}

impl Query {
    pub fn new(query_type: QueryType) -> Result<Query, GLError>;
    pub fn begin(&self);
    pub fn end(&self);
    pub fn is_result_available(&self) -> bool;
    pub fn get_result_u64(&self) -> u64;
}
```

### 3.7 `Sync`

```
impl Sync {
    pub fn fence_sync(condition: SyncCondition, flags: SyncBehavior) -> Sync;
    pub fn wait(&self, timeout_ns: u64) -> WaitResult;
}
```

---

## 4. Global State and Context

The crate does not manage OpenGL contexts.  Functions that require a current context are marked `unsafe` because the caller must guarantee that a valid context is current.  The following utility is provided to assist external context management:

```
pub unsafe fn set_context_getter(getter: extern "C" fn() -> bool);
```

This allows a windowing crate to register a callback that the crate will call before any GL operation to verify the context is current.  If the callback returns `false`, the operation panics with a clear error message.

---

## 5. Error Handling

```
pub enum GLError {
    InvalidEnum,
    InvalidValue,
    InvalidOperation,
    StackOverflow,
    StackUnderflow,
    OutOfMemory,
    InvalidFramebufferOperation,
    Unknown(GLenum),
}

pub fn get_error() -> Option<GLError>;
```

- Many functions return `Result<_, GLError>` by internally calling `get_error` after the GL call.  Functions that cannot fail return `()` but may set an error that can be queried.

---

## 6. Constants

All GL constants are defined as `pub const` values of the appropriate type (`GLenum`, `GLbitfield`, `GLboolean`).  For example:

```
pub const TEXTURE_2D: GLenum = 0x0DE1;
pub const ARRAY_BUFFER: GLenum = 0x8892;
pub const FRAGMENT_SHADER: GLenum = 0x8B30;
// … (hundreds of constants)
```

A separate code‑generated module `constants.blz` will be provided by the crate maintainers.

---

## 7. Implementation Notes

- The crate uses `extern "C"` to import the OpenGL functions.  Function pointers are loaded via a dynamic loader (e.g., `glad`, `glbinding`) at startup, or linked against a specific OpenGL library.  The default backend is **glow** (a cross‑platform loader), but the bindings can also be linked directly to a system OpenGL library.
- All object creation functions call `glGen*`, and `Dispose` calls `glDelete*`.  If an object is already deleted (name == 0), `Dispose` is a no‑op.
- The `Shader::source` method must be called before `compile`, and the source string must be valid UTF‑8 (the crate converts to a null‑terminated C string internally).
- For modern OpenGL (core profile), some functions like `glBegin`/`glEnd` are not provided; this crate targets the core profile only.

---

## 8. Testing

- **Object lifecycle:** Create and immediately drop a `Buffer`; verify no GL error is generated.  Use a mock GL library that records calls.
- **Shader compilation:** Create a shader, compile a minimal source, verify compile status is true; then delete.
- **Program linking:** Attach vertex/fragment shaders, link, verify link status.
- **Errors:** Attempt to compile an invalid shader; expect `get_error()` to return `InvalidEnum` or similar, and `get_compile_status()` false.
- All tests must run with a software OpenGL implementation (e.g., Mesa’s llvmpipe) or a headless GPU in CI.  A mock GL loader can be used for offline testing.
