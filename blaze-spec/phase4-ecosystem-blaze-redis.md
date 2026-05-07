# Phase 4 – Ecosystem Crate: `blaze‑redis`

> **Goal:** Specify the `blaze‑redis` crate, which provides an asynchronous Redis client and connection pool built on Blaze’s async I/O and actor model.  It implements the Redis Serialization Protocol (RESP2 and RESP3) natively, supports pipelining, transactions, Pub/Sub, and cluster‑aware routing.  All I/O operations carry the `io` effect and are non‑blocking within the Blaze async runtime.

---

## 1. Core Concepts

Redis is a fast, in‑memory data structure store accessed via a simple request‑response protocol over TCP.  The crate provides:

- **`RedisClient`** – a owned, linear connection to a single Redis instance (or a pool of connections).
- **`RedisPool`** – an actor‑based connection pool that multiplexes commands across multiple connections to the same Redis node or cluster.
- **`Cmd`** – a builder for constructing Redis commands with arbitrary arguments.
- **`RedisValue`** – an enum representing any value returned by Redis (strings, integers, arrays, nil, errors).
- **`PubSub`** – a dedicated subscriber actor for receiving messages from Redis channels/patterns.

The entire crate is data‑oriented: all types are enums or structs with no virtual dispatch, and the protocol parser is hand‑written as a state machine over a byte buffer.

---

## 2. `RedisClient`

### 2.1 Struct

```
pub struct RedisClient {
    stream: TcpStream,
    buf: Vec<u8>,
    state: ClientState,
}
```

- Linear type.  `Dispose` sends a `QUIT` command and closes the stream.

### 2.2 Connection

```
impl RedisClient {
    pub async fn connect(uri: &str) -> Result<RedisClient, RedisError>;
    pub async fn execute(&mut self, cmd: &Cmd) -> Result<RedisValue, RedisError>;
    pub async fn pipeline(&mut self, cmds: &[Cmd]) -> Result<Vec<RedisValue>, RedisError>;
    pub async fn subscribe(self, channels: &[Text]) -> Result<PubSub, RedisError>;
}
```

- **`connect`**: parses the URI (e.g., `redis://host:port/db`), opens a TCP stream, optionally authenticates with password, and selects the database number.
- **`execute`**: sends a single command, waits for the response, and returns it.  The command is serialized into the RESP protocol, sent, and the response is parsed.
- **`pipeline`**: sends multiple commands without waiting for individual replies, then reads all replies in order, reducing round‑trips.
- **`subscribe`**: converts the client into a `PubSub` actor for receiving messages (the client cannot be used for other commands after this).  It sends `SUBSCRIBE` or `PSUBSCRIBE` commands and begins listening for `message` / `pmessage` push events.

---

## 3. `Cmd` Builder

### 3.1 Struct

```
pub struct Cmd {
    args: Vec<RedisValue>,
    is_inline: bool,
}
```

- `args` contains the command tokens; the first element is the command name (uppercase, e.g., `"SET"`).

### 3.2 Construction

```
impl Cmd {
    pub fn new(command: &str) -> Cmd;
    pub fn arg<T: ToRedisValue>(&mut self, value: T) -> &mut Cmd;
    pub fn arg_if<T: ToRedisValue>(&mut self, condition: bool, value: T) -> &mut Cmd;
}
```

- `Cmd::new("GET")` creates a builder with the command name.
- `arg` appends a single argument (string, integer, float, bytes, nil).
- Convenience macros `cmd!` are provided for ergonomic command creation:

```
let cmd = cmd!("SET", "mykey", "myvalue");
```

The macro expands to `Cmd::new("SET").arg("mykey").arg("myvalue")`.

---

## 4. `RedisValue`

### 4.1 Enum

```
pub enum RedisValue {
    Nil,
    SimpleString(Text),        // simple string (OK, etc.)
    BulkString(Vec<u8>),       // binary‑safe string
    Integer(i64),
    Array(Vec<RedisValue>),
    Double(f64),
    Boolean(bool),
    BigNumber(Vec<u8>),        // arbitrary precision integer in string form (RESP3)
    Error(RedisError),
    // … VerbatimString, Map, Set, Push are RESP3‑only and may be added later
}
```

- All variants are owned; `RedisValue` is linear.  Cloning requires explicit `.clone()`.

### 4.2 Conversion Traits

```
pub trait ToRedisValue {
    fn to_redis_value(self) -> RedisValue;
}

pub trait FromRedisValue: Sized {
    fn from_redis_value(value: &RedisValue) -> Result<Self, RedisError>;
}
```

- Implemented for primitive types, strings, `Vec<u8>`, `Option<T>`, `Vec<T>`, etc.
- For example, `"hello"` becomes `RedisValue::BulkString(b"hello")`; converting back via `from_redis_value` extracts the bytes and validates UTF‑8 for `Text`.

---

## 5. RESP Protocol Parser

The crate contains a zero‑copy parser for RESP2 (and optional RESP3) based on a state machine that reads from a `&[u8]` buffer.  It handles:

- **Simple strings** (`+OK\r\n`)
- **Errors** (`-ERR unknown command\r\n`)
- **Integers** (`:1000\r\n`)
- **Bulk strings** (`$5\r\nhello\r\n`)
- **Arrays** (`*2\r\n$3\r\nfoo\r\n$3\r\nbar\r\n`)
- **Null bulk strings** (`$-1\r\n`)

The parser is called internally by `RedisClient` and is not exposed to the user.  It produces `RedisValue` variants directly without intermediate tokens.

---

## 6. `RedisPool`

### 6.1 Struct

```
pub struct RedisPool {
    // actor handle to the pool manager
}
```

- The pool maintains a set of idle connections.  `RedisPool` is `@copy` (it is a lightweight handle to an actor).  When asked for a connection, the pool checks if an idle connection exists; if not, it creates a new one (up to a max limit) or suspends the caller until one becomes available.

### 6.2 Methods

```
impl RedisPool {
    pub fn new(config: PoolConfig) -> RedisPool;
    pub async fn get(&self) -> Result<PooledConnection, RedisError>;
}

pub struct PooledConnection {
    inner: RedisClient,
    // on drop, returns the connection to the pool
}
```

- `PooledConnection` implements `Deref` to `RedisClient`, so users can call `execute`/`pipeline` directly on it.  On `Dispose`, the connection is checked for errors; if healthy, it is returned to the pool; if broken, it is closed.

### 6.3 `PoolConfig`

```
pub struct PoolConfig {
    pub max_connections: usize,
    pub idle_timeout: std::time::Duration,
    pub connection_timeout: std::time::Duration,
}
```

---

## 7. Pub/Sub

### 7.1 `PubSub`

```
pub struct PubSub {
    receiver: Receiver<PubSubMessage>,
    // actor handle for control
}

pub enum PubSubMessage {
    Message {
        channel: Text,
        pattern: Option<Text>,
        payload: Vec<u8>,
    },
    Subscribe { channel: Text, count: usize },
    Unsubscribe { channel: Text, count: usize },
}
```

- Created by `RedisClient::subscribe`.  The client is consumed and turned into a listening actor.  The user receives messages via the `receiver` channel.
- Additional channels/patterns can be subscribed/unsubscribed using `subscribe`/`unsubscribe` methods on `PubSub`, which send commands to the internal actor.

---

## 8. Error Handling

```
pub enum RedisError {
    Io(std::io::Error),
    Protocol(Text),                  // invalid RESP
    Server(Text),                    // Redis error reply
    AuthenticationFailed,
    ConnectionClosed,
    Timeout,
    PoolExhausted,
    TypeMismatch(Text),              // cannot convert RedisValue to requested type
}
```

- `Server(Text)` carries the Redis error message (e.g., `"WRONGTYPE"`, `"MOVED"`).
- `MOVED` and `ASK` errors for cluster redirection are handled internally by the cluster client (if cluster mode is enabled via feature), but in the base client they appear as `Server("MOVED ...")`.

---

## 9. Cluster Support (Optional Feature)

When the `cluster` feature is enabled, a `RedisCluster` type is provided.  It holds a map of slot‑to‑node assignments and a pool of connections per node.  The `RedisCluster` implements the same interface as `RedisClient`, automatically redirecting commands to the correct node based on the key hash.  `MOVED` and `ASK` errors are handled transparently.

---

## 10. Testing

- **Connection:** Connect to a local Redis instance (or a mock) and verify the `PING` command returns `PONG`.
- **Basic commands:** `SET` a key, `GET` it, verify the value; `DEL` and verify it’s gone.
- **Integer and float:** `INCR` a key, verify the integer; `SET` a float and get it back.
- **Pipelining:** Send a batch of `SET` commands, read all replies, and verify order.
- **Pub/Sub:** Subscribe to a channel, publish a message from another client, and verify the message is received via the `PubSub` receiver.
- **Error handling:** Execute an unknown command, expect `Server("ERR unknown command")`.  Send a command with wrong number of arguments, expect a protocol error.
- **Pool:** Create a pool with max 2 connections, acquire 2, try a third; it should suspend until one is released, then succeed.
- **Dispose:** Ensure dropping a `RedisClient` sends `QUIT` and closes the TCP stream.
