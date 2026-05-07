# Phase 4 – Ecosystem Crate: `blaze‑kafka`

> **Goal:** Specify the `blaze‑kafka` crate, which provides an asynchronous Kafka client built on Blaze’s async I/O, actor model, and `blaze‑tls`.  It implements the Kafka wire protocol natively, supporting producers, consumers, consumer groups, and administrative operations.  All I/O operations carry the `io` effect and are non‑blocking within the Blaze async runtime.  The design is data‑oriented, linear, and built for high throughput.

---

## 1. Core Concepts

Apache Kafka is a distributed event streaming platform with a binary TCP protocol.  The crate provides:

- **`KafkaClient`** – a connection to a Kafka broker (or bootstrap server).
- **`Producer`** – a typed message sender with partitioning and acknowledgment control.
- **`Consumer`** – a typed message receiver with group coordination and offset management.
- **`AdminClient`** – for managing topics, partitions, and configuration.
- **`KafkaRecord<K,V>`** – a message with optional key, value, headers, and timestamp.
- **`TopicPartition`** – identifies a topic and partition.
- **`Offset`** – a position within a partition.

All types are linear where appropriate (e.g., connections, consumers, producers).  The protocol implementation follows the Kafka wire protocol specification (versions 0–14 for selected keys, focusing on produce, fetch, metadata, offset fetch/commit, group management, and find coordinator).

---

## 2. `KafkaClient`

### 2.1 Struct

```
pub struct KafkaClient {
    stream: TcpStream,
    node_id: i32,
    correlation_id: i32,
    api_versions: Map<i16, VersionRange>,
    state: ClientState,
}
```

- Linear type; `Dispose` sends a proper disconnect and closes the stream.
- Maintains a monotonically increasing correlation ID for request‑response matching.

### 2.2 Connection

```
impl KafkaClient {
    pub async fn connect(brokers: &str, config: &KafkaConfig) -> Result<KafkaClient, KafkaError>;
    pub fn create_producer<K: Serialize, V: Serialize>(&self, topic: &str) -> Producer<K, V>;
    pub fn create_consumer<K: Deserialize<'static>, V: Deserialize<'static>>(&self, topic: &str, group: &str) -> Consumer<K, V>;
    pub fn admin(&self) -> AdminClient;
}
```

- **`connect`**: resolves the bootstrap brokers, negotiates the highest supported API version, and returns a client connected to one broker.
- **`create_producer`**: creates a `Producer` that will send records to the given topic.
- **`create_consumer`**: creates a `Consumer` that subscribes to a topic within a consumer group, handling partition assignment and offset commits.
- **`admin`**: returns an `AdminClient` that can perform administrative actions.

---

## 3. `KafkaConfig`

```
pub struct KafkaConfig {
    pub client_id: Text,
    pub max_request_size: usize,
    pub max_response_size: usize,
    pub request_timeout: std::time::Duration,
    pub retries: u32,
    pub retry_backoff: std::time::Duration,
    pub security_protocol: SecurityProtocol,
    pub sasl_config: Option<SaslConfig>,
    pub tls_config: Option<TlsConfig>,
}

pub enum SecurityProtocol {
    Plaintext,
    Ssl,
    SaslPlaintext,
    SaslSsl,
}
```

- Provides configuration for connections, authentication, and TLS.  `TlsConfig` from `blaze‑tls` is reused.

---

## 4. `Producer<K,V>`

### 4.1 Struct

```
pub struct Producer<K, V> {
    sink: Sender<ProducerRequest<K, V>>,
    // actor handle for the producer actor
}
```

- `@copy` handle to an internal actor that batches and sends records to Kafka brokers.

### 4.2 Methods

```
impl<K: Serialize, V: Serialize> Producer<K, V> {
    pub async fn send(&self, record: KafkaRecord<K, V>) -> Result<(), KafkaError>;
    pub fn send_batch(&self, records: Vec<KafkaRecord<K, V>>) -> Result<(), KafkaError>;
    pub async fn flush(&self);
    pub async fn close(self);
}
```

- **`send`**: asynchronously enqueues a single record.  The producer actor batches records for the same batch and sends them to the appropriate partition leader.
- **`send_batch`**: enqueues multiple records atomically.
- **`flush`**: waits until all previously sent records have been acknowledged by the broker (depending on `acks` configuration).
- **`close`**: shuts down the producer gracefully, flushing pending records.

### 4.3 Partitioning

The producer actor uses the provided key to determine the target partition (via `murmur2` hash over the serialized key bytes).  If no key is provided, records are distributed round‑robin or based on a configured partitioner callback.

### 4.4 Acknowledgements

The `acks` setting is specified in the `ProducerConfig`:

```
pub enum Acks {
    None,
    Leader,
    All,
}
```

Default is `Leader`.

---

## 5. `Consumer<K,V>`

### 5.1 Struct

```
pub struct Consumer<K, V> {
    receiver: Receiver<KafkaRecord<K, V>>,
    // actor handle for consumer group actor
}
```

- `@copy` handle to an internal actor that fetches from assigned partitions and commits offsets.

### 5.2 Methods

```
impl<K: Deserialize<'static>, V: Deserialize<'static>> Consumer<K, V> {
    pub async fn recv(&self) -> Option<KafkaRecord<K, V>>;
    pub async fn commit(&self);
    pub async fn seek(&self, tp: TopicPartition, offset: Offset);
    pub async fn close(self);
}
```

- **`recv`**: returns the next record from any assigned partition, suspending if none are available.
- **`commit`**: commits the current offsets (all assigned partitions) to the group coordinator.
- **`seek`**: moves the consumer to a specific offset on a partition.
- **`close`**: leaves the consumer group, commits offsets (if configured), and shuts down.

### 5.3 Group Coordination

The consumer actor handles the full group protocol (find coordinator, join group, sync group, heartbeat, offset fetch/commit) transparently.  It is configured via `ConsumerConfig`:

```
pub struct ConsumerConfig {
    pub group_id: Text,
    pub session_timeout: Duration,
    pub max_poll_records: usize,
    pub auto_offset_reset: AutoOffsetReset,
    pub enable_auto_commit: bool,
}

pub enum AutoOffsetReset {
    Latest,
    Earliest,
    None,
}
```

---

## 6. `KafkaRecord<K,V>`

```
pub struct KafkaRecord<K, V> {
    pub topic: Text,
    pub partition: i32,
    pub offset: Offset,
    pub timestamp: i64,
    pub key: Option<K>,
    pub value: V,
    pub headers: Vec<(Text, Vec<u8>)>,
}
```

- Key and value types are generic; serialization is performed by `blaze‑serde` (or manually if using the low‑level `RawKafkaRecord`).
- `Offset` is a newtype over `i64`.

---

## 7. Admin Operations

### 7.1 `AdminClient`

```
pub struct AdminClient {
    // actor handle
}
impl AdminClient {
    pub async fn create_topic(&self, name: &str, partitions: i32, replication_factor: i16) -> Result<(), KafkaError>;
    pub async fn delete_topic(&self, name: &str) -> Result<(), KafkaError>;
    pub async fn list_topics(&self) -> Result<Vec<TopicInfo>, KafkaError>;
    pub async fn describe_topic(&self, name: &str) -> Result<TopicDescription, KafkaError>;
}
```

- Administrative requests are sent directly to the controller broker (via the client’s metadata).

---

## 8. Wire Protocol

The crate implements a subset of the Kafka protocol APIs:

- **Metadata** (API key 3) – used to discover broker layout and partition leaders.
- **Produce** (API key 0) – send records to a partitions.
- **Fetch** (API key 1) – read records from a partitions.
- **OffsetFetch** (API key 9) – get consumer group offsets.
- **OffsetCommit** (API key 8) – commit consumer group offsets.
- **GroupCoordinator** (API key 10) – find coordinator for a group.
- **JoinGroup** (API key 11) – join a consumer group.
- **SyncGroup** (API key 14) – sync group assignments.
- **Heartbeat** (API key 12) – heartbeat to group coordinator.
- **LeaveGroup** (API key 13) – leave a consumer group.
- **ApiVersions** (API key 18) – negotiate supported versions.
- **CreateTopics** (API key 19) – create new topics.
- **DeleteTopics** (API key 20) – delete topics.

Each request and response is serialized/deserialized using the Kafka binary protocol specified for each API key.  The implementation uses `blaze‑binary` and `blaze‑endian` for primitive serialization.

---

## 9. Error Handling

```
pub enum KafkaError {
    Io(std::io::Error),
    Protocol(Text),
    ApiVersionNotSupported,
    CoordinatorNotAvailable,
    NotCoordinator,
    OffsetOutOfRange,
    UnknownTopicOrPartition,
    LeaderNotAvailable,
    NotLeaderForPartition,
    RequestTimedOut,
    ConsumerRebalanceInProgress,
    Serialization(Text),
    Deserialization(Text),
    AuthenticationFailed,
    AuthorizationFailed,
}
```

- Errors from the broker are mapped to the appropriate enum variant.  Unknown server errors are represented as `Protocol` with the server’s error message.

---

## 10. Implementation Notes

- The crate uses a connection pool internally for efficient broker communication.  Each broker connection is an actor that multiplexes requests.
- The `Producer` actor batches records per partition and flushes them either on a timer or when a batch limit is reached.  Backpressure is handled by bounding the internal channel.
- All serialization of keys and values defaults to `blaze‑serde` with a configurable serializer (e.g., JSON, protobuf, binary).  The user can also pass raw bytes if they want full control.
- Consumer group rebalancing is cooperative: when a new consumer joins, a `ConsumerRebalanceRequest` is sent to the group coordinator and all consumers in the group are notified.  The Blaze actor model makes the rebalance state machine straightforward: each consumer actor reacts to join/sync messages while continuing to fetch on its currently assigned partitions.

---

## 11. Testing

- **Connect:** Bootstrap to a real Kafka cluster (or embedded Kafka for testing), verify that metadata is fetched.
- **Produce and consume:** Create a producer, send records, then create a consumer, subscribe, and verify that all records are received in order.
- **Consumer group:** Start two consumers in the same group; send records to two partitions; observe that each consumer receives records from one partition.
- **Offset commit:** Commit offsets after consuming, restart consumer, and verify it resumes from the committed offset.
- **Error handling:** Disconnect the broker while producing; verify that the producer retries and eventually returns an `Io` error.
- **Admin:** Create a topic, list topics, verify its presence, then delete and confirm removal.
- All tests must pass with a running Kafka instance (integration tests) and offline unit tests for protocol encoding/decoding.
