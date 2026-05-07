# Phase 4 – Ecosystem Crate: `blaze‑rabbitmq`

> **Goal:** Specify the `blaze‑rabbitmq` crate, which provides an asynchronous RabbitMQ client built on Blaze’s async I/O, actor model, and `blaze‑tls`.  It implements the AMQP 0‑9‑1 wire protocol natively, supporting publishers, consumers, exchanges, queues, bindings, and message acknowledgements.  All I/O operations carry the `io` effect and are non‑blocking within the Blaze async runtime.  The design is data‑oriented, linear, and built for reliable message delivery.

---

## 1. Core Concepts

RabbitMQ is a message broker that stores and forwards messages between producers and consumers using the AMQP protocol.  The crate provides:

- **`Connection`** – a single TCP/TLS connection to a RabbitMQ server, multiplexing channels.
- **`Channel`** – a lightweight logical connection within a TCP connection; all AMQP operations occur on a channel.
- **`Publisher`** – sends messages to an exchange.
- **`Consumer`** – receives messages from a queue, handles acknowledgements.
- **`Exchange`** – declares and manages exchanges (topic, direct, fanout, headers).
- **`Queue`** – declares and manages queues, bindings.
- **`Message`** – an AMQP message with body, properties, and delivery tag.

All types are linear where appropriate (Connection, Channel).  The protocol implementation follows AMQP 0‑9‑1 specification.

---

## 2. AMQP 0‑9‑1 Wire Protocol

The crate implements the full AMQP 0‑9‑1 frame layer and content header/body serialization.  Key protocol methods:

- `connection.start`, `connection.open`, `connection.close`
- `channel.open`, `channel.close`
- `exchange.declare`, `exchange.delete`, `exchange.bind`, `exchange.unbind`
- `queue.declare`, `queue.delete`, `queue.bind`, `queue.unbind`, `queue.purge`
- `basic.publish`, `basic.consume`, `basic.ack`, `basic.nack`, `basic.reject`, `basic.qos`, `basic.get`
- `confirm.select` (publisher confirms)
- `tx.select`, `tx.commit`, `tx.rollback` (transactions)

All frames are encoded/decoded using the AMQP type system (short‑string, long‑string, table, timestamp, etc.) implemented with `blaze‑binary`.

---

## 3. `Connection`

### 3.1 Struct

```
pub struct Connection {
    stream: TcpStream,
    state: ConnectionState,
    channels: Map<u16, ChannelHandle>,
    next_channel_id: u16,
}
```

- Linear type; `Dispose` gracefully closes the connection (sends `connection.close` with reply code 200, reason "Goodbye") and closes the stream.

### 3.2 Methods

```
impl Connection {
    pub async fn connect(uri: &str, config: &ConnectionConfig) -> Result<Connection, RabbitError>;
    pub async fn open_channel(&mut self) -> Result<Channel, RabbitError>;
    pub async fn close(self) -> Result<(), RabbitError>;
}
```

- **`connect`**: opens a TCP/TLS stream, performs the AMQP handshake (start, tune, open).
- **`open_channel`**: opens a new channel, returning a `Channel` handle.
- **`close`**: graceful shutdown.

---

## 4. `Channel`

### 4.1 Struct

```
pub struct Channel {
    id: u16,
    sender: Sender<ChannelCommand>,
    receiver: Receiver<ChannelResponse>,
    state: ChannelState,
}
```

- Created by `Connection::open_channel`.  Linear; `Dispose` closes the channel gracefully.

### 4.2 Methods

```
impl Channel {
    pub async fn declare_exchange(&self, name: &str, kind: ExchangeKind, options: &ExchangeOptions) -> Result<(), RabbitError>;
    pub async fn declare_queue(&self, name: &str, options: &QueueOptions) -> Result<QueueInfo, RabbitError>;
    pub async fn bind_queue(&self, queue: &str, exchange: &str, routing_key: &str) -> Result<(), RabbitError>;
    pub async fn unbind_queue(&self, queue: &str, exchange: &str, routing_key: &str) -> Result<(), RabbitError>;
    pub async fn publish(&self, exchange: &str, routing_key: &str, message: &Message) -> Result<(), RabbitError>;
    pub async fn consume(&self, queue: &str, consumer_tag: &str, config: &ConsumerConfig) -> Result<Consumer, RabbitError>;
    pub async fn ack(&self, delivery_tag: u64) -> Result<(), RabbitError>;
    pub async fn nack(&self, delivery_tag: u64, requeue: bool) -> Result<(), RabbitError>;
    pub async fn qos(&self, prefetch_count: u16, prefetch_size: u32, global: bool) -> Result<(), RabbitError>;
    pub async fn confirm_select(&self) -> Result<(), RabbitError>;
    pub fn publisher_confirm(&self) -> Receiver<bool>;
    pub async fn close(self) -> Result<(), RabbitError>;
}
```

- **`publish`**: sends a `basic.publish` frame with the message body and properties.
- **`consume`**: starts a consumer on a queue; returns a `Consumer` object that yields `Delivery` messages.
- **`ack`/`nack`**: acknowledges or rejects a delivery.
- **`qos`**: sets quality of service (prefetch count).
- **`confirm_select`**: enables publisher confirms mode; subsequent publishes return acknowledgements via the `publisher_confirm` receiver.
- **`close`**: gracefully shuts down the channel.

---

## 5. `Exchange`, `Queue`, Bindings

### 5.1 `ExchangeKind`

```
pub enum ExchangeKind {
    Direct,
    Fanout,
    Topic,
    Headers,
    Custom(Text),
}
```

### 5.2 `ExchangeOptions`

```
pub struct ExchangeOptions {
    pub durable: bool,
    pub auto_delete: bool,
    pub internal: bool,
    pub arguments: Map<Text, AmqpValue>,
}
```

### 5.3 `QueueOptions`

```
pub struct QueueOptions {
    pub durable: bool,
    pub exclusive: bool,
    pub auto_delete: bool,
    pub arguments: Map<Text, AmqpValue>,
}
```

### 5.4 `QueueInfo`

```
pub struct QueueInfo {
    pub name: Text,
    pub message_count: u32,
    pub consumer_count: u32,
}
```

---

## 6. `Message` and `Delivery`

### 6.1 `Message`

```
pub struct Message {
    pub body: Vec<u8>,
    pub content_type: Option<Text>,
    pub content_encoding: Option<Text>,
    pub headers: Map<Text, AmqpValue>,
    pub delivery_mode: DeliveryMode,
    pub priority: u8,
    pub correlation_id: Option<Text>,
    pub reply_to: Option<Text>,
    pub expiration: Option<Text>,
    pub message_id: Option<Text>,
    pub timestamp: Option<SystemTime>,
    pub type_name: Option<Text>,
    pub user_id: Option<Text>,
    pub app_id: Option<Text>,
}

pub enum DeliveryMode {
    NonPersistent = 1,
    Persistent = 2,
}
```

### 6.2 `Delivery`

```
pub struct Delivery {
    pub delivery_tag: u64,
    pub exchange: Text,
    pub routing_key: Text,
    pub redelivered: bool,
    pub message: Message,
}
```

---

## 7. `Consumer`

```
pub struct Consumer {
    receiver: Receiver<Delivery>,
    channel: Channel,
    consumer_tag: Text,
}

impl Consumer {
    pub async fn recv(&self) -> Option<Delivery>;
    pub async fn cancel(self) -> Result<(), RabbitError>;
}

impl Iterator for Consumer {
    type Item = Delivery;
    fn next(&mut self) -> Option<Delivery> { self.recv() }
}
```

---

## 8. Error Handling

```
pub enum RabbitError {
    Io(std::io::Error),
    Protocol(Text),
    AuthenticationFailed,
    ConnectionRefused,
    ChannelClosed(u16, Text),
    ConsumerCancelled,
    Timeout,
    Serialization(Text),
}
```

---

## 9. Implementation Notes

- The crate uses a single TCP connection with multiple channels.  Each channel is an actor that serializes commands into an internal SPSC queue; a single writer actor multiplexes frame writes over the TCP stream.
- Heartbeats are sent automatically according to the negotiated heartbeat interval.
- All string values in the protocol (short‑string, long‑string) are validated UTF‑8; invalid strings cause a protocol error.
- Publisher confirms are implemented by sending `basic.ack`/`basic.nack` replies to a per‑channel channel.

---

## 10. Testing

- **Connection:** Connect to a local RabbitMQ instance, verify handshake, negotiate channel max.
- **Publish and consume:** Declare a queue, bind to exchange, publish a message, consume and verify delivery.
- **Acknowledgements:** Consume with manual ack, verify redelivery after consumer disconnect.
- **QoS:** Set prefetch count, publish multiple messages, verify only up to prefetch are delivered.
- **Error handling:** Close channel from server side (e.g., declare non‑existent exchange) and verify `ChannelClosed` error.
- **Dispose:** Ensure Connection and Channel close gracefully on drop.
