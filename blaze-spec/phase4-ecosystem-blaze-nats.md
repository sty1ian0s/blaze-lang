# Phase 4 – Ecosystem Crate: `blaze‑nats`

> **Goal:** Specify the `blaze‑nats` crate, which provides an asynchronous NATS client built on Blaze’s async I/O, actor model, and `blaze‑tls`.  It implements the NATS wire protocol natively, supporting core NATS, JetStream, and KV store operations.  All I/O operations carry the `io` effect and are non‑blocking within the Blaze async runtime.  The design is data‑oriented, linear, and built for high‑throughput, low‑latency messaging.

---

## 1. Core Concepts

NATS is a lightweight, high‑performance messaging system with a simple text‑based protocol.  The crate provides:

- **`NatsClient`** – a connection to a NATS server (or cluster).
- **`Publisher`** – publishes messages to subjects.
- **`Subscriber`** – subscribes to subjects and receives messages.
- **`Requestor`** – sends request‑reply messages.
- **`JetStream`** – stream and consumer management.
- **`KeyValue`** – NATS KV store operations.
- **`Message`** – a NATS message with subject, payload, and reply subject.

---

## 2. `NatsClient`

### 2.1 Struct

```
pub struct NatsClient {
    stream: TcpStream,
    state: ClientState,
    subscriptions: Map<u64, Sender<Message>>,
    next_sid: u64,
}
```

- Linear type; `Dispose` flushes the buffer, sends `UNSUB` for all subscriptions, then closes the stream.

### 2.2 Methods

```
impl NatsClient {
    pub async fn connect(uri: &str, config: &NatsConfig) -> Result<NatsClient, NatsError>;
    pub async fn publish(&self, subject: &str, reply: Option<&str>, payload: &[u8]) -> Result<(), NatsError>;
    pub async fn subscribe(&self, subject: &str, queue: Option<&str>) -> Result<Subscriber, NatsError>;
    pub async fn request(&self, subject: &str, payload: &[u8], timeout: Duration) -> Result<Message, NatsError>;
    pub async fn flush(&self) -> Result<(), NatsError>;
    pub fn jetstream(&self) -> JetStream;
    pub fn key_value(&self, bucket: &str) -> KeyValue;
    pub async fn close(self) -> Result<(), NatsError>;
}
```

---

## 3. `NatsConfig`

```
pub struct NatsConfig {
    pub name: Option<Text>,
    pub verbose: bool,
    pub pedantic: bool,
    pub tls_required: bool,
    pub auth_token: Option<Text>,
    pub user: Option<Text>,
    pub password: Option<Text>,
    pub nkey: Option<Text>,
    pub jwt: Option<Text>,
    pub tls_config: Option<TlsConfig>,
    pub max_reconnect: usize,
    pub reconnect_time_wait: Duration,
    pub max_payload: usize,
    pub inbox_prefix: Text,
}
```

---

## 4. `Subscriber`

### 4.1 Struct

```
pub struct Subscriber {
    sid: u64,
    receiver: Receiver<Message>,
    client: NatsClient,
}

impl Subscriber {
    pub async fn recv(&self) -> Option<Message>;
    pub async fn unsubscribe(self) -> Result<(), NatsError>;
    pub async fn auto_unsubscribe(self, max: usize) -> Result<(), NatsError>;
}

impl Iterator for Subscriber {
    type Item = Message;
    fn next(&mut self) -> Option<Message> { self.recv() }
}
```

---

## 5. `Message`

```
pub struct Message {
    pub subject: Text,
    pub reply: Option<Text>,
    pub payload: Vec<u8>,
    pub headers: Option<Map<Text, Vec<Text>>>,
    pub sub: Option<Subscription>,
}
```

---

## 6. JetStream

### 6.1 `JetStream`

```
pub struct JetStream {
    client: NatsClient,
    domain: Option<Text>,
    api_prefix: Text,
}

impl JetStream {
    pub async fn stream_create(&self, config: &StreamConfig) -> Result<StreamInfo, NatsError>;
    pub async fn stream_update(&self, config: &StreamConfig) -> Result<StreamInfo, NatsError>;
    pub async fn stream_delete(&self, name: &str) -> Result<(), NatsError>;
    pub async fn stream_info(&self, name: &str) -> Result<StreamInfo, NatsError>;
    pub async fn consumer_create(&self, stream: &str, config: &ConsumerConfig) -> Result<ConsumerInfo, NatsError>;
    pub async fn consumer_delete(&self, stream: &str, consumer: &str) -> Result<(), NatsError>;
    pub async fn consumer_info(&self, stream: &str, consumer: &str) -> Result<ConsumerInfo, NatsError>;
    pub async fn consume(&self, stream: &str, consumer: &str, max_deliver: Option<usize>) -> Result<Subscriber, NatsError>;
    pub async fn publish(&self, subject: &str, payload: &[u8]) -> Result<PubAck, NatsError>;
    pub async fn publish_async(&self, subject: &str, payload: &[u8]) -> Result<PubAck, NatsError>;
}
```

---

## 7. KeyValue

### 7.1 `KeyValue`

```
pub struct KeyValue {
    js: JetStream,
    bucket: Text,
}

impl KeyValue {
    pub async fn create(&self, name: &str, replicas: usize) -> Result<(), NatsError>;
    pub async fn put(&self, key: &str, value: &[u8]) -> Result<u64, NatsError>;
    pub async fn get(&self, key: &str) -> Result<Option<Entry>, NatsError>;
    pub async fn delete(&self, key: &str) -> Result<(), NatsError>;
    pub async fn watch(&self, key: &str) -> Result<Subscriber, NatsError>;
    pub async fn history(&self, key: &str) -> Result<Vec<Entry>, NatsError>;
    pub async fn purge(&self, key: &str) -> Result<(), NatsError>;
}

pub struct Entry {
    pub key: Text,
    pub value: Vec<u8>,
    pub revision: u64,
    pub created: SystemTime,
    pub delta: u64,
    pub operation: KvOperation,
}
```

---

## 8. Protocol Implementation

NATS wire protocol is text‑based, with special framing for payloads.  Messages are delimited by `\r\n`, and payloads may include `\r\n` (requires length‑prefixed format for large payloads).  The crate implements a state‑machine parser that handles `INFO`, `MSG`, `+OK`, `-ERR`, `PING`, `PONG`, etc.

---

## 9. Error Handling

```
pub enum NatsError {
    Io(std::io::Error),
    Protocol(Text),
    NoResponders,
    Timeout,
    MaxPayload,
    AuthorizationViolation,
    ConnectionClosed,
    InvalidSubject,
    InvalidStream,
    InvalidConsumer,
    JetStreamApi(Text),
}
```

---

## 10. Testing

- **Connect:** Connect to a NATS server, verify that `INFO` is received and `PING`/`PONG` works.
- **Pub/Sub:** Publish a message on a subject, subscribe and receive it, verify payload integrity.
- **Request/Reply:** Start a subscriber that replies, send a request, verify response.
- **JetStream:** Create a stream, publish a message, create a consumer, consume the message, verify sequence ordering.
- **KV:** Create a bucket, put a key‑value pair, get it, delete it, verify history.
- **Reconnect:** Simulate a connection drop, verify automatic reconnect with subscriptions restored.
- **Dispose:** Ensure client and subscribers clean up correctly.
