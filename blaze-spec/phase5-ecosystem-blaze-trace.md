# Phase 5 – Ecosystem Crate: `blaze‑trace`

> **Goal:** Specify the `blaze‑trace` crate, which provides a distributed tracing framework adhering to the OpenTelemetry standard.  It enables instrumentation of Blaze applications to produce spans that capture the flow of requests across services, actors, and async tasks.  The crate offers a zero‑cost, data‑oriented API where no‑op tracing adds negligible overhead, and span creation automatically captures the current execution context from the Blaze async runtime.  Export to various backends (OTLP, Jaeger, Zipkin, a local file) is supported via pluggable exporters.  All span operations are pure when no exporter is active, and carry the `io` effect only when exporting over the network.

---

## 1. Core Concepts

A **trace** represents the entire journey of a request, composed of multiple **spans**.  Each span has an operation name, start and end timestamps, a set of attributes (key‑value pairs), events (time‑stamped log‑like entries), and a status.  Spans are linked via a parent‑child relationship, forming a tree within a trace.  Context propagation (trace ID, span ID, trace flags) can be passed across process boundaries (e.g., via HTTP headers), and the crate provides inject/extract functions for standard formats (W3C TraceContext, B3).

The crate is designed for low overhead: when no tracer provider is configured, span creation returns a no‑op span that is completely elided at compile‑time (via `@comptime` when possible).  Real spans are allocated in the current async task’s region and automatically ended when the span object is dropped (linear `Dispose`).

---

## 2. `Tracer` and `TracerProvider`

### 2.1 `TracerProvider`

```
pub trait TracerProvider: Send + Sync + 'static {
    fn create_tracer(&self, name: &str, version: &str) -> Box<dyn Tracer>;
}
```

- The `TracerProvider` is a factory for `Tracer` instances.  A global provider is set at application startup.

### 2.2 `Tracer`

```
pub trait Tracer: Send + Sync {
    fn start(&self, name: &str, parent: Option<&SpanContext>) -> Span;
    fn span_context(&self) -> SpanContext;   // for the current active span
}
```

- `start` creates a new span.  If `parent` is `None`, it inherits the current active span from the async context.
- The returned `Span` is linear (must be ended).  On `Dispose`, the span is ended automatically and exported (if a recorder is active).

---

## 3. `Span`

### 3.1 Struct

```
pub struct Span {
    inner: Box<dyn SpanImpl>,
}

impl Span {
    pub fn set_attribute(&mut self, key: &str, value: impl Into<AttributeValue>);
    pub fn add_event(&mut self, name: &str, attributes: Vec<KeyValue>);
    pub fn record_error(&mut self, err: &dyn Error);
    pub fn set_status(&mut self, code: StatusCode, description: &str);
    pub fn end(self);   // explicitly ends the span (also done on drop)
}

impl Dispose for Span {
    fn dispose(&mut self) { self.end(); }
}
```

- The `Span` is a linear resource.  Dropping it without calling `end` will automatically end the span, which might capture a backtrace (if configured) and export the span.

### 3.2 `SpanContext`

```
pub struct SpanContext {
    pub trace_id: TraceId,
    pub span_id: SpanId,
    pub trace_flags: TraceFlags,
    pub is_remote: bool,
    pub trace_state: Vec<(Text, Text)>,
}
```

- `TraceId`, `SpanId` are 128‑bit and 64‑bit binary values (represented as `u128` and `u64`).
- `TraceFlags` contains a sampled flag.

---

## 4. Attributes and Events

### 4.1 `KeyValue`

```
pub struct KeyValue {
    pub key: Text,
    pub value: AttributeValue,
}

pub enum AttributeValue {
    String(Text),
    Bool(bool),
    Int(i64),
    Float(f64),
    Array(Vec<AttributeValue>),
    StringArray(Vec<Text>),
    BoolArray(Vec<bool>),
    IntArray(Vec<i64>),
    FloatArray(Vec<f64>),
}
```

- Attributes are stored as a linear array in the span.

---

## 5. Context Propagation

The crate provides functions to inject and extract span context from carrier objects (e.g., HTTP headers, message metadata).

```
pub trait Injector {
    fn set(&mut self, key: &str, value: &str);
}
pub trait Extractor {
    fn get(&self, key: &str) -> Option<&str>;
}

pub fn inject(context: &SpanContext, format: PropagationFormat, carrier: &mut dyn Injector);
pub fn extract(format: PropagationFormat, carrier: &dyn Extractor) -> Option<SpanContext>;
```

- `PropagationFormat` enum: `W3CTraceContext`, `B3`, `B3Multi`.

---

## 6. Exporters

### 6.1 `SpanExporter`

```
pub trait SpanExporter: Send + Sync {
    fn export(&self, spans: &[SpanData]) -> Result<(), TraceError>;
    fn flush(&self);
}
```

- `SpanData` is a snapshot of a finished span, containing all its fields (name, IDs, timestamps, attributes, events, status).

### 6.2 Built‑in Exporters

- **`OtlpExporter`** – sends spans via gRPC (using `blaze‑grpc`) or HTTP (using `blaze‑http`) to an OpenTelemetry collector.
- **`JaegerExporter`** – sends spans in Jaeger Thrift format over UDP or HTTP.
- **`ZipkinExporter`** – sends spans in Zipkin JSON format over HTTP.
- **`FileExporter`** – writes spans to a file (e.g., for offline analysis).
- **`LogExporter`** – forwards spans as structured log records via `blaze‑log`.
- **`StdoutExporter`** – prints spans as JSON to stdout.

Exporters are configured via a builder pattern and can be chained.

---

## 7. Sampling

To reduce overhead in high‑throughput systems, the crate supports sampling:

```
pub trait Sampler {
    fn should_sample(&self, trace_id: TraceId, operation: &str, parent: Option<&SpanContext>) -> bool;
}
```

- Built‑in samplers: `AlwaysOn`, `AlwaysOff`, `Probability(f64)`, `RateLimiting(u32)`.

---

## 8. Integration with Async and Actors

Blaze’s async runtime automatically maintains a per‑task `SpanContext`.  When a new async task is spawned (e.g., via `spawn` or actor message handler), the current span context is propagated to the new task.  The crate provides a `current_span()` function that returns the active `SpanContext` for the current execution context.

For actors, the tracer automatically creates a span for each message handler invocation, naming it after the actor and message type.

---

## 9. Error Handling

```
pub enum TraceError {
    Io(std::io::Error),
    InvalidTraceId,
    InvalidSpanId,
    ExportFailed(Text),
    SerializationError(Text),
}
```

- Errors from exporters are logged but do not affect the application.

---

## 10. Implementation Notes

- The global tracer provider is stored in a static `OnceLock`.  When no provider is set, all `start` calls return a `NoopSpan` that has empty methods and is optimized away.
- Span data is allocated in a linear region per trace (or per span batch) to reduce memory fragmentation.  Exporters receive batches of `SpanData` and must process them synchronously (or copy them for async processing).
- The `Span` type uses dynamic dispatch (`dyn SpanImpl`) to allow different exporters without static dependencies.  The overhead is one vtable lookup per span method, acceptable for tracing.
- The crate uses `blaze‑time` for monotonic timestamps and `blaze‑log` for internal logging when an exporter fails.

---

## 11. Testing

- **Span lifecycle:** Create a tracer, start a span, add attributes, end, verify the exporter receives the span data.
- **Noop when no provider:** Without setting a provider, start spans; verify no allocation and no exporter call.
- **Context propagation:** Inject into a mock carrier, extract, verify the extracted context matches.
- **Sampling:** Set a probability sampler, generate many traces, verify the sampled fraction matches.
- **Async propagation:** Spawn an async task that creates a span; verify the parent span context is correctly inherited.
- **Export:** Use a `FileExporter`, write a span, read the file, parse back.
- All tests must pass on all platforms.
