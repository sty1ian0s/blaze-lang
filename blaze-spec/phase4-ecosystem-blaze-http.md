# Phase 4 – Ecosystem Crate: `blaze‑http`

> **Goal:** Specify the `blaze‑http` crate, which provides an HTTP client and a lightweight, actor‑based HTTP server.  It is built entirely on Blaze’s core I/O, async, and actor primitives, adhering to the language’s data‑oriented, zero‑cost abstraction philosophy.  This crate is optional but serves as the standard HTTP implementation for the ecosystem.

---

## 1. Core Types

### 1.1 `HttpError`

```
pub enum HttpError {
    Io(std::io::Error),
    Parse(ParseError),
    Protocol(Text),
    Timeout,
}
```

- Wraps all errors that can occur during HTTP operations.  Implements `Debug` and `Display`.
- `ParseError` covers malformed request/response lines, headers, or body framing.

### 1.2 `Method`

```
pub enum Method {
    Get,
    Post,
    Put,
    Delete,
    Head,
    Options,
    Patch,
    Connect,
    Trace,
}
```

- Standard HTTP methods.  Additional methods can be represented via a custom string with `Method::Other(Text)`.

### 1.3 `StatusCode`

```
pub struct StatusCode(u16);
```

- Represents an HTTP status code (e.g., 200, 404).  Provides associated constants like `StatusCode::OK`, `StatusCode::NOT_FOUND`.

### 1.4 `Headers`

```
pub struct Headers {
    // small‑vector optimisation: inline storage for up to 8 headers
}
```

- A collection of HTTP headers.  Implemented as a linear data structure; duplicate header names are allowed (for `Set‑Cookie`, `Accept`, etc.).
- **Methods:**
  - `pub fn new() -> Headers`
  - `pub fn insert(&mut self, name: &str, value: &str)`
  - `pub fn get(&self, name: &str) -> Option<&str>`
  - `pub fn remove(&mut self, name: &str) -> Option<Text>`
  - `pub fn iter(&self) -> impl Iterator<Item = (&str, &str)>`
  - `pub fn len(&self) -> usize`

### 1.5 `Uri`

```
pub struct Uri {
    scheme: Text,
    host: Text,
    port: u16,
    path: Text,
    query: Option<Text>,
}
```

- Parses and holds a URI.  Constructed via `Uri::parse(s: &str) -> Result<Uri, ParseError>`.

---

## 2. Client API

All client operations are asynchronous and carry the `io` effect (and thus require an async runtime).

### 2.1 `Client`

```
pub struct Client {
    // connection pool, configuration
}
```

- An HTTP client that manages connection reuse, timeouts, and redirects.

- **Constructors:**
  - `pub fn new() -> Client;`
  - `pub fn with_config(config: ClientConfig) -> Client;`

### 2.2 `ClientConfig`

```
pub struct ClientConfig {
    pub timeout: std::time::Duration,
    pub max_redirects: usize,
    pub user_agent: Option<Text>,
}
```

### 2.3 `Request`

```
pub struct Request {
    method: Method,
    uri: Uri,
    headers: Headers,
    body: Option<Body>,
}
```

- Builders are generated via `@builder` attribute, but the raw struct is accessible.

### 2.4 `Response`

```
pub struct Response {
    status: StatusCode,
    headers: Headers,
    body: Body,
}
```

### 2.5 `Body`

```
pub enum Body {
    Empty,
    Bytes(Vec<u8>),
    Text(Text),
    Stream(Box<dyn BodyStream>),
}
```

- Stream variant allows sending/receiving chunked or large payloads without buffering.

### 2.6 Sending a Request

```
impl Client {
    pub async fn send(&self, request: Request) -> Result<Response, HttpError>;
}
```

- Executes the HTTP request.  Handles redirects according to `ClientConfig::max_redirects`.
- If the response body is not fully read, its `Dispose` ensures the connection is released.

### 2.7 Convenience Functions

For simple cases, the crate provides per‑method functions on `Client`:

```
impl Client {
    pub async fn get(&self, uri: &str) -> Result<Response, HttpError>;
    pub async fn post(&self, uri: &str, body: Body) -> Result<Response, HttpError>;
    pub async fn put(&self, uri: &str, body: Body) -> Result<Response, HttpError>;
    pub async fn delete(&self, uri: &str) -> Result<Response, HttpError>;
}
```

---

## 3. Server API

The server side uses Blaze’s actor model for concurrency: each incoming connection is handled by a spawned actor, keeping all connection state isolated and linear.

### 3.1 `HttpServer`

```
pub struct HttpServer {
    // listens on a socket
}
```

- **Constructors:**
  - `pub fn bind(addr: &str) -> Result<HttpServer, HttpError>;`

- **Methods:**
  - `pub async fn serve(self, handler: impl HttpHandler + Send + 'static) -> Result<(), HttpError>;`
    - Accepts a handler that implements the `HttpHandler` trait.  For each incoming connection, the server spawns a new actor, passes it a `Request`, and expects a `Response` or an error.  The server gracefully shuts down when all outstanding actors have finished and the socket is closed.

### 3.2 `HttpHandler` Trait

```
pub trait HttpHandler: Send + 'static {
    fn handle(&self, request: Request) -> Response;
}
```

- Synchronous handler: the request is fully buffered before calling the handler, which must produce a `Response` immediately.
- For streaming handlers, a separate `AsyncHttpHandler` trait is available (defined later).

### 3.3 Actor‑Based Routing

The crate includes a simple `Router` that implements `HttpHandler` and routes based on method and path to registered handler functions.

```
pub struct Router {
    routes: Vec<Route>,
}

impl Router {
    pub fn new() -> Self;
    pub fn route(mut self, method: Method, path: &str, handler: fn(Request) -> Response) -> Self;
}

impl HttpHandler for Router { … }
```

Routes are matched in order of registration; the first match wins.

---

## 4. Integration with `blaze‑serde`

`Request` and `Response` do not automatically implement `Serialize`/`Deserialize`.  However, the crate provides helper functions to read/write JSON bodies using `blaze‑serde` (if present).  These are feature‑gated on `serde`:

```
#[cfg(feature = "serde")]
impl Request {
    pub fn json<T: Deserialize<'static>>(&self) -> Result<T, Error>;
}

#[cfg(feature = "serde")]
impl Response {
    pub fn json<T: Serialize>(body: &T) -> Response;
}
```

---

## 5. Implementation Notes

- The client uses `std::io` and `std::net` (a future `blaze‑net` crate that provides TCP/UDP).  Since `blaze‑net` is not yet specified, we rely on raw socket wrappers from `std::os::unix`/`std::os::windows` or define a minimal `TcpStream` inside the crate.  For Phase 4, we can bundle a basic `TcpStream` implementation that implements `Read` and `Write`.
- The server runs on top of the actor runtime; each connection handler is an actor with a single message: the incoming socket.  The handler reads the request, processes it, and writes the response.
- All header and URI operations are data‑oriented: no virtual calls, no inheritance, just plain structs and enums.
- The `Body::Stream` variant uses a trait object (`dyn BodyStream`) for dynamic dispatch when the body content is not known in advance; this is the only place where dynamic dispatch is used, and it is explicitly justified.

---

## 6. Testing

- **Client:** Mock a simple HTTP server (or use a test‑only loopback server) and test GET/POST requests, redirects, timeouts.
- **Server:** Bind to a random ephemeral port, register a handler, send a request from the client, and verify the response.
- **Router:** Register multiple routes, send requests, check that the correct handler is called.
- **Error handling:** Simulate connection failures, malformed responses, and verify that appropriate `HttpError` variants are returned.
- **Actor isolation:** Ensure that multiple concurrent requests are handled independently; test with a handler that blocks (sleeps) and verify it does not affect other requests.
