# Phase 4 – Ecosystem Crate: `blaze‑http2`

> **Goal:** Specify the `blaze‑http2` crate, which provides an HTTP/2 client and server implementation built on top of the Blaze async runtime and the `blaze‑http` core types.  It supports binary framing, multiplexing of concurrent streams, header compression using HPACK, server push, and flow control.  The design is data‑oriented, linear, and actor‑based.

---

## 1. Dependencies and Integration

- Uses the same core types as `blaze‑http`: `Method`, `StatusCode`, `Headers`, `Uri`, `Request`, `Response`, `Body`.
- Re‑exports those types so that users can depend on `blaze‑http2` without importing `blaze‑http` separately.
- Operates over a TLS stream (provided by `blaze‑tls`) or a raw TCP stream (upgraded to ALPN‑negotiated HTTP/2 via `h2` protocol).

---

## 2. HTTP/2 Frame Layer

### 2.1 `Frame`

HTTP/2 communication is entirely frame‑based.  The crate defines an enum for all frame types:

```
pub enum Frame {
    Data {
        stream_id: u32,
        flags: DataFlags,
        payload: Vec<u8>,
    },
    Headers {
        stream_id: u32,
        flags: HeadersFlags,
        header_block_fragment: Vec<u8>,
    },
    Priority {
        stream_id: u32,
        exclusive: bool,
        stream_dep: u32,
        weight: u8,
    },
    RstStream {
        stream_id: u32,
        error_code: ErrorCode,
    },
    Settings {
        flags: SettingsFlags,
        settings: Vec<(SettingId, u32)>,
    },
    PushPromise {
        stream_id: u32,
        promised_stream_id: u32,
        header_block_fragment: Vec<u8>,
    },
    Ping {
        flags: PingFlags,
        opaque_data: [u8; 8],
    },
    GoAway {
        last_stream_id: u32,
        error_code: ErrorCode,
        debug_data: Vec<u8>,
    },
    WindowUpdate {
        stream_id: u32,
        window_size_increment: u32,
    },
    Continuation {
        stream_id: u32,
        header_block_fragment: Vec<u8>,
    },
}
```

- `DataFlags`, `HeadersFlags`, `SettingsFlags`, `PingFlags` are bit‑flag types.
- `SettingId` and `ErrorCode` are enums with the standard HTTP/2 values.

### 2.2 `ErrorCode`

```
pub enum ErrorCode {
    NoError = 0,
    ProtocolError = 1,
    InternalError = 2,
    FlowControlError = 3,
    SettingsTimeout = 4,
    StreamClosed = 5,
    FrameSizeError = 6,
    RefusedStream = 7,
    Cancel = 8,
    CompressionError = 9,
    ConnectError = 10,
    EnhanceYourCalm = 11,
    InadequateSecurity = 12,
    Http11Required = 13,
}
```

---

## 3. HPACK Header Compression

The crate includes a pure, zero‑allocation (where possible) implementation of the HPACK compression algorithm (RFC 7541).  It is exposed via two types:

### 3.1 `HpackEncoder`

```
pub struct HpackEncoder {
    dynamic_table: DynamicTable,
}
impl HpackEncoder {
    pub fn new() -> Self;
    pub fn encode<'a>(&mut self, headers: &Headers, buf: &'a mut Vec<u8>) -> Result<&'a [u8], Error>;
}
```

- Encodes a set of headers into the HPACK literal representation, updating the dynamic table.
- The output is a byte slice that must be sent as a HEADERS frame’s `header_block_fragment`.

### 3.2 `HpackDecoder`

```
pub struct HpackDecoder {
    dynamic_table: DynamicTable,
}
impl HpackDecoder {
    pub fn new() -> Self;
    pub fn decode(&mut self, fragment: &[u8]) -> Result<Headers, Error>;
}
```

- Decodes a header block fragment, updating the dynamic table, and returns the resulting `Headers`.

---

## 4. Stream Abstraction

Each HTTP/2 stream is an independent, linear object that implements `Read` and `Write` for the stream’s data frames.  The library provides `Stream`:

```
pub struct Stream {
    stream_id: u32,
    rx: Receiver<Vec<u8>>,
    tx: Sender<Vec<u8>>,
    // flow‑control windows
}
```

- Created by the connection handler when a new stream is opened.
- `read` and `write` operate on the stream’s data flow.
- `Dispose` for `Stream` sends an RST_STREAM frame with `Cancel` if the stream is still open.

---

## 5. Client API

### 5.1 `Client`

```
pub struct Client {
    config: ClientConfig,
}
```

- **Constructors:**
  - `pub fn new() -> Client;`
  - `pub fn with_config(config: ClientConfig) -> Client;`

- **Connections:**
  - `pub async fn connect(&self, uri: &Uri) -> Result<Connection, HttpError>;`

`Connection` represents a single HTTP/2 connection (multiplexed).  It spawns an actor internally to manage frames.

### 5.2 `Connection`

```
pub struct Connection {
    sender: Sender<ClientCommand>,
}
impl Connection {
    pub async fn send(&self, request: Request) -> Result<Response, HttpError>;
    pub async fn close(self);
}
```

- Sending a request allocates a new stream ID and sends a HEADERS frame.
- The returned `Response` includes the body as a stream (the response data is read from the stream).

### 5.3 `ClientConfig`

```
pub struct ClientConfig {
    pub max_concurrent_streams: u32,
    pub initial_window_size: u32,
    pub enable_push: bool,
    pub timeout: std::time::Duration,
}
```

---

## 6. Server API

The server is actor‑based like `blaze‑http`, but with the ability to handle multiple concurrent streams on a single connection.

### 6.1 `Http2Server`

```
pub struct Http2Server {
    // listens on TLS socket
}
impl Http2Server {
    pub fn bind(addr: &str, tls_config: &TlsConfig) -> Result<Self, HttpError>;
    pub async fn serve(self, handler: impl HttpHandler + Send + 'static) -> Result<(), HttpError>;
}
```

- Each incoming connection spawns a `ConnectionActor` that handles the HTTP/2 handshake and frame multiplexing.
- The `HttpHandler` trait is the same as in `blaze‑http`; the server adapts HTTP/2 streams to `Request`/`Response`.

### 6.2 `HttpHandler`

```
pub trait HttpHandler: Send + 'static {
    fn handle(&self, request: Request) -> Response;
}
```

- For HTTP/2, the handler is called with the complete request headers (and optionally, a streaming body if the request includes data).  The response is sent as HEADERS + DATA frames.

### 6.3 Server Push

The server can push additional resources by sending PUSH_PROMISE frames.  This is exposed via a `PushPromise` struct returned by the handler, or via a method on a `Responder` object.

```
pub struct PushPromise {
    path: Text,
    response: Response,
}
```

The handler can return a `(Response, Vec<PushPromise>)` tuple if it wants to push.  The server will send the PUSH_PROMISE before the main response.

---

## 7. Flow Control

The crate implements stream‑level and connection‑level flow control as per RFC 7540.  Both are configurable via settings frames.  The internal `Stream` objects track window sizes and buffer data accordingly.  If a stream’s receive window is exhausted, the peer is not sent WINDOW_UPDATE until the application reads data.  The sender will block (or return `WouldBlock`) if the remote window is full, using backpressure from the actor’s async channels.

---

## 8. Error Handling

Errors are mapped to the appropriate HTTP/2 error codes and RST_STREAM frames.  The crate’s `HttpError` type (from `blaze‑http`) is extended with HTTP/2 specific variants:

```
pub enum HttpError {
    Io(std::io::Error),
    Parse(ParseError),
    Protocol(Text),
    Timeout,
    Http2(ErrorCode),
    GoAway(Option<ErrorCode>),
}
```

---

## 9. Implementation Notes

- The entire crate is built on top of `blaze‑tls` for TLS, `blaze‑async-std` or the core runtime for async, and `blaze‑http` for common types.
- Frame serialization and deserialization are hand‑written for performance, avoiding any heap allocation for small frames (headers are allocated inline).
- The `ConnectionActor` is the central piece: it reads frames from the socket, dispatches them to the appropriate stream actors, and handles streams lifecycle.
- All internal channels are bounded to enforce backpressure.

---

## 10. Testing

- **Frame encoding/decoding:** Test serialization and deserialization of each frame type, ensuring round‑tripping.
- **HPACK:** Encode a set of headers, decode with an empty dynamic table, verify the result matches the original.
- **Multiplexing:** Create a client and server, open multiple concurrent streams, send requests and receive responses simultaneously, verifying order is maintained per stream but interleaved across streams.
- **Flow control:** Set small window sizes, send large payloads, verify that the sender respects window limits and resumes after WINDOW_UPDATE.
- **Server push:** Register a handler that pushes an extra resource; on the client side, observe a PUSH_PROMISE frame and subsequent response.
- **Error handling:** Trigger a protocol error (e.g., invalid frame) and verify that the connection is torn down with a GOAWAY frame.
