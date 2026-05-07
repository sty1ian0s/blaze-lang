# Phase 4 – Ecosystem Crate: `blaze‑tls`

> **Goal:** Specify the `blaze‑tls` crate, which provides TLS (Transport Layer Security) client and server functionality built on top of Blaze’s async I/O and actor model.  It wraps a native TLS library (e.g., OpenSSL, BoringSSL, or a pure‑Blaze implementation) and exposes a simple, safe stream interface.  The crate integrates with `blaze‑http`, `blaze‑http2`, and any other protocol that requires encrypted transport.

---

## 1. Core Concepts

TLS provides encrypted, authenticated communication over a raw byte stream (typically TCP).  After a TLS handshake, the application reads and writes plaintext data through a `TlsStream`, which internally encrypts/decrypts and manages the underlying socket.

The crate exposes two main types:
- `TlsConnector` – for establishing a client TLS connection.
- `TlsAcceptor` – for accepting a server TLS connection.

Both are configurable with certificates, private keys, and trust anchors.

---

## 2. Configuration

### 2.1 `TlsConfig`

```
pub struct TlsConfig {
    pub cert_file: Option<Text>,         // path to certificate chain (PEM or DER)
    pub key_file: Option<Text>,          // path to private key file
    pub ca_file: Option<Text>,           // path to CA certificate for verifying peer
    pub verify_hostname: bool,           // require hostname match (client only)
    pub min_protocol_version: ProtocolVersion,
    pub max_protocol_version: ProtocolVersion,
    pub cipher_list: Option<Text>,       // custom cipher string
    pub alpn_protocols: Vec<Text>,       // ALPN protocols (e.g., "h2", "http/1.1")
    pub enable_sni: bool,                // enable Server Name Indication (client)
}
```

- `TlsConfig` is `@copy` (it's a small struct with no pointers, all fields are `Clone`).
- Defaults: TLS 1.2 minimum, no certificates, no verification, empty ALPN.

### 2.2 `ProtocolVersion`

```
pub enum ProtocolVersion {
    TlsV1_0,
    TlsV1_1,
    TlsV1_2,
    TlsV1_3,
}
```

---

## 3. Client API

### 3.1 `TlsConnector`

```
pub struct TlsConnector {
    inner: ConnectorImpl,   // platform‑specific
}

impl TlsConnector {
    pub fn new(config: TlsConfig) -> Result<Self, TlsError>;
    pub async fn connect<S: Read + Write + Send + 'static>(&self, domain: &str, stream: S) -> Result<TlsStream<S>, TlsError>;
}
```

- `new(config)` builds the connector from the given configuration.  Returns an error if the certificate/private key files cannot be read or parsed.
- `connect(domain, stream)` performs the TLS handshake on the given raw stream (typically a TCP stream).  The `domain` parameter is used for SNI and hostname verification (if enabled).  The returned `TlsStream` is in the `Open` state.

### 3.2 `TlsStream<S>`

```
pub struct TlsStream<S> {
    inner: S,
    ssl: SslContext,        // platform‑specific
    state: State,
}
```

- Wraps the underlying stream `S` (must implement `Read` and `Write`) and provides TLS encryption/decryption.
- Implements `Read`, `Write`, and `Dispose`.

**Methods:**
```
impl<S: Read + Write> TlsStream<S> {
    pub fn get_ref(&self) -> &S;
    pub fn get_mut(&mut self) -> &mut S;
    pub fn alpn_protocol(&self) -> Option<&str>;
    pub fn peer_certificate(&self) -> Option<Certificate>;
}
```

- `alpn_protocol()` returns the negotiated ALPN protocol (e.g., `"h2"`), if any.
- `peer_certificate()` returns the peer's X.509 certificate, if available.

### 3.3 `Certificate`

```
pub struct Certificate {
    der: Vec<u8>,
}
impl Certificate {
    pub fn to_der(&self) -> &[u8];
    pub fn subject_name(&self) -> Option<Text>;
    pub fn issuer_name(&self) -> Option<Text>;
    pub fn not_before(&self) -> Option<SystemTime>;
    pub fn not_after(&self) -> Option<SystemTime>;
}
```

---

## 4. Server API

### 4.1 `TlsAcceptor`

```
pub struct TlsAcceptor {
    inner: AcceptorImpl,
}

impl TlsAcceptor {
    pub fn new(config: TlsConfig) -> Result<Self, TlsError>;
    pub async fn accept<S: Read + Write + Send + 'static>(&self, stream: S) -> Result<TlsStream<S>, TlsError>;
}
```

- `new(config)` builds an acceptor; the configuration must include a certificate and private key.
- `accept(stream)` performs the TLS server handshake.  The returned `TlsStream` is ready for encrypted communication.

---

## 5. Error Handling

```
pub enum TlsError {
    Io(std::io::Error),
    Config(Text),             // certificate parse error, missing file
    Handshake(HandshakeError),
    Certificate(Text),        // verification error
    Protocol(Text),           // protocol violation
    Timeout,
}

pub struct HandshakeError {
    pub kind: HandshakeErrorKind,
    pub message: Text,
}

pub enum HandshakeErrorKind {
    NoCertificates,
    InvalidCertificate,
    UnknownCa,
    HandshakeFailure,
    ProtocolVersion,
    BadRecordMac,
    DecodeError,
    UnexpectedMessage,
}
```

---

## 6. Implementation Notes

- The crate builds on a native TLS library.  At Phase 4, the default backend is **OpenSSL** (or BoringSSL) linked via `extern "C"`.  A future `blaze‑rustls` (pure‑Blaze TLS) could be substituted.
- The implementation wraps the C API with Blaze’s unsafe FFI; all `TlsStream` operations are safe because the raw pointers are managed by the wrapper and lifetime‑checked.
- The `TlsStream` implements `Read` and `Write` by calling `SSL_read`/`SSL_write` on the underlying SSL context.  It respects the blocking mode of the stream: if the raw stream is non‑blocking, the TLS operations will return `WouldBlock` when they cannot proceed, allowing the async runtime to wake the task.
- For async usage, the `TlsStream` is polled by the runtime; the crate provides an `AsyncRead`/`AsyncWrite` implementation (or simply relies on the runtime's ability to poll `Read`/`Write` traits with `WouldBlock` detection).

---

## 7. Testing

- **Client handshake:** Create a self‑signed certificate, start a test server, connect with a client, verify the handshake succeeds and `alpn_protocol` returns the correct value.
- **Server handshake:** Accept a connection, verify the peer certificate (optional), and exchange data.
- **Encryption round‑trip:** Write data on one side, read on the other, verify the plaintext matches.
- **Invalid certificate:** Configure a client with a CA that doesn't sign the server cert; verify that `connect` returns a `Certificate` verification error.
- **ALPN negotiation:** Connect with ALPN `["h2"]` and verify the server selects it; absent from server side, verify the handshake still succeeds without ALPN.
- **Error injection:** Corrupt a TLS record, verify the stream returns a `Protocol` error.
- **Resource cleanup:** Ensure that dropping a `TlsStream` properly calls `SSL_shutdown` and closes the underlying socket.
