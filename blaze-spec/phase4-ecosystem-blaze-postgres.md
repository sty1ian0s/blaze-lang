# Phase 4 – Ecosystem Crate: `blaze‑postgres`

> **Goal:** Specify the `blaze‑postgres` crate, which provides a concrete PostgreSQL driver implementing the `blaze‑sql` traits.  It uses the PostgreSQL wire protocol (version 3) natively over a TCP/TLS connection, without any external C library.  All I/O operations are asynchronous and carry the `io` effect, and the driver integrates seamlessly with Blaze’s actor model, linear type system, and async runtime.

---

## 1. Dependencies and Design

- `blaze‑sql` – core SQL traits (`Database`, `Connection`, `Row`, `Value`, etc.).
- `blaze‑tls` – optional TLS support for encrypted connections (gated by feature `tls`).
- `blaze‑binary` and `blaze‑endian` – for encoding/decoding wire protocol messages.
- `std::io` and `std::net` (or `blaze‑net`) – for TCP stream handling.

The crate implements the PostgreSQL front‑end protocol directly, avoiding any C library dependency.  This aligns with Blaze’s philosophy of writing critical infrastructure in Blaze itself for safety, performance, and observability.

---

## 2. `Postgres` – The Database Type

```
pub struct Postgres;

impl Database for Postgres {
    type Connection = PostgresConnection;
    fn connect(uri: &str) -> Result<PostgresConnection, Error>;
}
```

- `connect(uri)` parses the connection URI (e.g., `postgres://user:pass@host:port/dbname?sslmode=require`), performs the PostgreSQL startup handshake, authenticates (using password, SCRAM‑SHA‑256, or client certificates), and returns a `PostgresConnection`.

---

## 3. `PostgresConnection`

### 3.1 Struct

```
pub struct PostgresConnection {
    stream: Box<dyn AsyncReadWrite>,    // TCP or TLS stream
    parameters: Map<Text, Text>,         // server parameters from startup
    state: ConnectionState,
}

enum ConnectionState {
    Idle,
    Active,
    Transaction(u32),                    // depth of nested transaction
    Broken,
}
```

- The connection is **linear**; cloning is not possible.  `Dispose` sends a `Terminate` message and closes the stream.

### 3.2 Constructors

```
impl PostgresConnection {
    pub fn new(stream: Box<dyn AsyncReadWrite>, params: Map<Text, Text>) -> Self;
}
```

- Called internally after the handshake is complete.

### 3.3 `Connection` Implementation

```
impl Connection for PostgresConnection {
    type Database = Postgres;

    async fn execute(&self, query: &Query) -> Result<ExecuteResult, Error>;
    async fn query(&self, query: &Query) -> Result<Rows, Error>;
    async fn transaction(&self) -> Result<Transaction<'_, Self>, Error>;
    fn is_autocommit(&self) -> bool;
    fn set_autocommit(&mut self, value: bool) -> Result<(), Error>;
    fn close(self) -> Result<(), Error>;
}
```

- **`execute`**: binds parameters to a prepared statement, sends `Bind`→`Execute`→`Sync`, and reads the `CommandComplete` response.  For batched statements, reads multiple command completions.
- **`query`**: same but reads `RowDescription`, then reads data rows (text or binary format depending on negotiation) until `ReadyForQuery`.
- **`transaction`**: sends `BEGIN` (or `SAVEPOINT` if nested), returns a `PostgresTransaction`.
- **`is_autocommit` / `set_autocommit`**: tracks whether the session is in autocommit (the default for PostgreSQL) or has an open transaction block.  Setting autocommit to `false` sends a `BEGIN`; setting to `true` commits any pending transaction.

---

## 4. `PostgresTransaction`

```
pub struct PostgresTransaction<'a> {
    conn: &'a mut PostgresConnection,
    depth: u32,
    committed: bool,
}

impl<'a> PostgresTransaction<'a> {
    pub async fn commit(self) -> Result<(), Error>;
    pub async fn rollback(self) -> Result<(), Error>;
}

impl<'a> Dispose for PostgresTransaction<'a> {
    fn dispose(&mut self) {
        if !self.committed {
            // send ROLLBACK or ROLLBACK TO SAVEPOINT automatically
        }
    }
}
```

- Nested transactions are supported via savepoints.  The `depth` field tracks the nesting level, so that a commit/release only affects the current savepoint.

---

## 5. Wire Protocol Encoding/Decoding

The crate implements a complete encoder/decoder for PostgreSQL wire protocol messages.  Key message types:

- **Startup:** `StartupMessage` (version, parameters), `SSLRequest`, `PasswordMessage`, `SASLInitialResponse`, `SASLResponse`.
- **Query:** `Parse`, `Bind`, `Describe`, `Execute`, `Sync`, `Close` (prepare statement management).
- **Response:** `ErrorResponse`, `NoticeResponse`, `Authentication*`, `ParameterStatus`, `BackendKeyData`, `ReadyForQuery`, `RowDescription`, `DataRow`, `CommandComplete`, `EmptyQueryResponse`, `ParseComplete`, `BindComplete`, `CloseComplete`, `NotificationResponse`, `CopyInResponse`, `CopyOutResponse`.

All numbers are transmitted in network byte order (big‑endian).  The crate uses `std::endian` to swap as needed.

---

## 6. Type Mapping

The driver negotiates **binary** format for all queries by default (except for unknown types where text format is used).  This avoids conversion overhead and ensures precision for types like `NUMERIC`, `TIMESTAMPTZ`, `UUID`, etc.

### 6.1 Type Registry

The crate maintains an internal type registry that maps PostgreSQL type OIDs to Blaze types:

| PostgreSQL type | PG OID (built‑in) | Blaze type |
|-----------------|-------------------|-------------|
| `int2` | 21 | `i16` |
| `int4` | 23 | `i32` |
| `int8` | 20 | `i64` |
| `float4` | 700 | `f32` |
| `float8` | 701 | `f64` |
| `bool` | 16 | `bool` |
| `varchar`/`text` | 1043, 25 | `Text` |
| `bytea` | 17 | `Vec<u8>` |
| `timestamp` / `timestamptz` | 1114, 1184 | `std::time::SystemTime` (or a dedicated `PgTimestamp` struct) |
| `date` | 1082 | `PgDate` (newtype) |
| `uuid` | 2950 | `Uuid` (from `blaze‑uuid`) |
| `json` / `jsonb` | 114, 3802 | `JsonValue` (from `blaze‑json`) |
| `numeric` | 1700 | `PgNumeric` (arbitrary precision) |
| `int2[]`, `int4[]`, etc. | array OIDs | `Vec<T>` |

For types not in the registry, the driver falls back to text format and returns `Value::Text`.

### 6.2 `FromValue` and `ToValue`

The crate implements `FromValue` and `ToValue` for all supported Blaze types, using the binary format send/recv functions.  For example, an `i32` is encoded as 4 bytes in network order, and decoded by reading 4 bytes and swapping endianness.

---

## 7. Error Handling

```
pub enum PgError {
    Io(std::io::Error),
    WireFormat(Text),         // malformed message
    AuthFailed(Text),
    QueryError(PgErrorResponse),
    ConnectionClosed,
    Timeout,
}
pub struct PgErrorResponse {
    pub severity: Text,
    pub code: Text,           // SQLSTATE
    pub message: Text,
    pub detail: Option<Text>,
    pub hint: Option<Text>,
    pub position: Option<u32>,
}
impl From<PgError> for sql::Error { … }
```

- Wire protocol errors (e.g., invalid message type, length mismatch) → `WireFormat`.
- Authentication failures → `AuthFailed`.
- SQL errors (returned by server) → `QueryError`, carrying the full `ErrorResponse` fields for rich diagnostics.

---

## 8. Implementation Notes

- The crate uses a `Box<dyn AsyncReadWrite>` for the underlying stream, allowing TLS to be optionally enabled via a feature flag without changing the connection code.
- The type registry is initialised with built‑in OIDs and can be extended by the user for custom types via a `register_type` method on `PostgresConnection`.
- Connection pooling is **not** part of this crate; it belongs to `blaze‑sql‑pool`.
- All I/O is asynchronous: when a message is being sent, the crate writes the full message buffer using `write_all`.  When receiving, it reads the message type byte and length, then reads the body.  The runtime’s async I/O ensures non‑blocking behavior.
- PostgreSQL NOTIFY messages are delivered to a channel provided by the user via `set_notification_handler`, allowing reactive patterns.

---

## 9. Testing

- **Connection and handshake:** Start a test PostgreSQL instance (e.g., via Docker), connect with a valid URI, verify that the connection parameters (server version, client encoding) are received.
- **Execute:** Create a table, insert rows, execute updates and deletes, verify affected row counts.
- **Query:** Insert sample rows, query them back with `query`, verify column names, types, and row values.
- **Binary format:** Insert and retrieve various types (int, float, text, bytea, json) and ensure round‑trip accuracy.
- **Error handling:** Run a malformed query, verify `Error::QueryError` with a valid SQLSTATE and message.
- **Transactions:** Start a transaction, insert, rollback, verify absence; repeat with commit and verify persistence.
- **TLS:** Connect with `sslmode=require` to a test server that requires TLS (e.g., via a self‑signed certificate).  Verify the handshake succeeds.
- **Dispose:** Ensure that dropping a `PostgresConnection` sends a `Terminate` message and closes the stream.

All tests must pass on platforms with access to a running PostgreSQL server (integration tests) and offline unit tests for wire protocol encoding/decoding.
