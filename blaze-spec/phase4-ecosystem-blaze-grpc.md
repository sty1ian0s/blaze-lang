# Phase 4 – Ecosystem Crate: `blaze‑grpc`

> **Goal:** Specify the `blaze‑grpc` crate, which provides a gRPC client and server implementation built on top of Blaze’s async runtime, HTTP/2 support (`blaze‑http2`), and the `blaze‑serde` serialization framework.  It uses Protocol Buffers as the default serialization format, but is designed to be modular, allowing pluggable codecs.  The crate fully embraces Blaze’s data‑oriented, linear, and actor‑based philosophy.

---

## 1. Core Concepts

gRPC is an RPC framework that uses HTTP/2 for transport and Protocol Buffers for message encoding.  Services are defined in `.proto` files, which are compiled to Blaze source code by a separate protoc plugin (`blaze‑protoc`).  This crate provides the runtime support for the generated code: client stubs, server handlers, and streaming.

The core types are:
- `MethodDescriptor` – describes a single RPC method (request/response types, streaming type).
- `ServiceDescriptor` – a collection of methods.
- `Client` – a channel to a remote gRPC server.
- `Server` – a gRPC server that routes requests to service implementations.

---

## 2. Protocol Buffers Integration

The crate depends on `blaze‑protobuf` (a low‑level protobuf encoding/decoding library) and re‑exports its core traits.  Every gRPC message type implements `protobuf::Message`:

```
pub trait Message: Serialize + Deserialize<'static> + Send + Sync + 'static {
    fn descriptor() -> MessageDescriptor;
}
```

`MessageDescriptor` provides field information for reflection (not needed at runtime, but used by codegen).

---

## 3. Method and Service Descriptors

### 3.1 `MethodDescriptor`

```
pub struct MethodDescriptor {
    pub name: &'static str,
    pub full_name: &'static str,   // e.g., "/package.Service/Method"
    pub streaming: StreamingType,
    pub request_type: std::meta::Type,
    pub response_type: std::meta::Type,
}

pub enum StreamingType {
    Unary,
    ClientStreaming,
    ServerStreaming,
    BidiStreaming,
}
```

- `name` is the simple method name.
- `full_name` is used as the HTTP/2 path.
- `streaming` indicates the type of RPC.

### 3.2 `ServiceDescriptor`

```
pub struct ServiceDescriptor {
    pub name: &'static str,
    pub methods: &'static [MethodDescriptor],
}
```

The generated code for each service creates a `ServiceDescriptor` constant.

---

## 4. Client API

### 4.1 `Channel`

A `Channel` represents a connection to a gRPC server (HTTP/2 connection with `blaze‑http2` client).

```
pub struct Channel {
    inner: Arc<Http2Client>,
    uri: Uri,
}
impl Channel {
    pub async fn connect(uri: &str) -> Result<Channel, GrpcError>;
    pub fn create_stub<S: Service>(&self) -> S::Stub;
}
```

- `connect` establishes the underlying connection.
- `create_stub` creates a typed client stub for the given service.

### 4.2 Generated Stub

The protoc plugin generates a trait for each service (e.g., `Greeter`) and a client stub struct that implements that trait.  The stub methods are `async fn` that take `Request<Req>` and return `Result<Response<Resp>, GrpcError>`.

```
// generated trait
pub trait Greeter: Send + Sync + 'static {
    async fn say_hello(&self, request: Request<HelloRequest>) -> Result<Response<HelloReply>, GrpcError>;
}
// generated stub
pub struct GreeterStub { channel: Channel }
impl Greeter for GreeterStub { … }
```

The stub serializes the request, constructs an HTTP/2 HEADERS frame with the gRPC‑specific headers, sends it on the channel, and deserializes the response.

### 4.3 `Request<T>` and `Response<T>`

```
pub struct Request<T> {
    pub message: T,
    pub metadata: MetadataMap,
    pub extensions: Extensions,
}
pub struct Response<T> {
    pub message: T,
    pub metadata: MetadataMap,
    pub extensions: Extensions,
}
```

- `metadata` contains custom HTTP/2 headers (e.g., authentication tokens).
- `extensions` is a type‑map for passing additional context (e.g., deadline).

---

## 5. Server API

### 5.1 `Server`

```
pub struct Server {
    router: Router,
}
impl Server {
    pub fn new() -> Self;
    pub fn add_service<S: Service>(&mut self, service: S) -> &mut Self;
    pub async fn serve(self, addr: &str) -> Result<(), GrpcError>;
}
```

- `add_service` registers a service implementation (a struct that implements the generated trait).
- `serve` binds to the address, starts an HTTP/2 server, and routes incoming gRPC requests to the appropriate method handler.

### 5.2 Service Implementation

The user writes a struct that implements the generated trait:

```
struct MyGreeter;
impl Greeter for MyGreeter {
    async fn say_hello(&self, request: Request<HelloRequest>) -> Result<Response<HelloReply>, GrpcError> {
        let reply = HelloReply { message: format!("Hello, {}!", request.message.name) };
        Ok(Response::new(reply))
    }
}
```

### 5.3 Streaming Methods

For streaming methods, the generated trait uses `Stream` types (from `blaze‑async‑std` or the core runtime) instead of `Request<T>` / `Response<T>`:

- Client streaming: `async fn client_stream(&self, stream: impl Stream<Item = Req>) -> Result<Response<Resp>, GrpcError>`
- Server streaming: `async fn server_stream(&self, request: Request<Req>) -> Result<impl Stream<Item = Resp>, GrpcError>`
- Bidi streaming: `async fn bidi_stream(&self, stream: impl Stream<Item = Req>) -> Result<impl Stream<Item = Resp>, GrpcError>`

The server handler can read from the request stream and write to the response stream concurrently.

---

## 6. Metadata and Error Handling

### 6.1 `MetadataMap`

```
pub struct MetadataMap(Vec<(Text, Text)>);
impl MetadataMap {
    pub fn new() -> Self;
    pub fn insert(&mut self, key: &str, value: &str);
    pub fn get(&self, key: &str) -> Option<&str>;
    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)>;
}
```

- Used for custom gRPC metadata (grpc‑metadata‑… headers).

### 6.2 `GrpcError`

```
pub enum GrpcError {
    Status(Status),
    Transport(HttpError),
    Codec(Text),       // serialization/deserialization error
}
pub struct Status {
    code: Code,
    message: Text,
    details: Vec<u8>,   // protobuf‑encoded details (optional)
}
pub enum Code {
    Ok = 0,
    Cancelled = 1,
    Unknown = 2,
    InvalidArgument = 3,
    DeadlineExceeded = 4,
    NotFound = 5,
    AlreadyExists = 6,
    PermissionDenied = 7,
    ResourceExhausted = 8,
    FailedPrecondition = 9,
    Aborted = 10,
    OutOfRange = 11,
    Unimplemented = 12,
    Internal = 13,
    Unavailable = 14,
    DataLoss = 15,
    Unauthenticated = 16,
}
```

- `Status` is the standard gRPC error representation (Code, message, optional detail).
- `GrpcError::Status` is used for all application‑level errors.
- `Transport` wraps HTTP/2 or network errors.
- `Codec` covers protobuf encoding/decoding failures.

---

## 7. Implementation Notes

- The crate re‑exports key types from `blaze‑http2` and `blaze‑protobuf`, gated by features.
- The generated code uses `blaze‑serde` for message serialization; the actual wire format is protobuf, so the `Serializer` and `Deserializer` implementations for protobuf are provided by `blaze‑protobuf`.
- The server uses an actor‑based design similar to `blaze‑http`: each incoming RPC is handled by a spawned actor, which holds the request and sends the response.  Streaming handlers are long‑lived actors that can handle multiple messages in a stream.
- All internal channels are bounded to provide backpressure under heavy load.

---

## 8. Testing

- **Unary RPC:** Start a server with a simple service, create a client channel, and call the method; verify the response matches.
- **Streaming RPCs:** Test client‑stream, server‑stream, and bidi‑streaming methods with multiple messages.
- **Error codes:** Implement a service method that returns a specific gRPC status code and verify the client receives that `GrpcError::Status`.
- **Metadata propagation:** Set custom metadata in the request, verify the server receives it, and set response metadata to verify the client receives it.
- **Invalid requests:** Send a request that cannot be deserialized; verify the server returns `Code::InvalidArgument` and the client gets the error.
- **Concurrency:** Spawn many concurrent RPCs and verify they are multiplexed correctly over a single HTTP/2 connection.
