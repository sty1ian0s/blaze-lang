# Phase 4 – Ecosystem Crate: `blaze‑sql`

> **Goal:** Specify the `blaze‑sql` crate, which provides a common, data‑oriented interface for SQL databases.  It defines core types, traits, and error handling that are shared by concrete database driver crates (e.g., `blaze‑sqlite`, `blaze‑postgres`, `blaze‑redis`).  The design is statically typed, linear, and fully asynchronous, leveraging Blaze’s effect system and actor model for connection management.

---

## 1. Core Concepts

The `blaze‑sql` crate defines:

- **`Database` trait** – an abstraction over a database backend (SQLite, PostgreSQL, etc.).
- **`Connection`** – a linear handle to a single database connection.
- **`Transaction`** – a linear RAII handle for database transactions.
- **`Row`** – a collection of values read from a result set.
- **`Value`** – a dynamic, type‑safe representation of a single database value.
- **`Query`** – a parsed, preparable SQL statement.
- **`Error`** – unified error type.

All database drivers implement the `Database` trait and provide a concrete `Connection` that can execute queries and return results.

---

## 2. `Database` Trait

```
pub trait Database: Sized {
    type Connection: Connection<Database = Self>;

    fn connect(uri: &str) -> Result<Self::Connection, Error>;
}
```

- Each implementing crate provides a `Database` type (e.g., `Sqlite`, `Postgres`) and its associated `Connection`.
- `connect` establishes a new connection, taking a connection string specific to the database.

---

## 3. `Connection` Trait

```
pub trait Connection: Send + Sync + 'static + Dispose {
    type Database: Database<Connection = Self>;

    async fn execute(&self, query: &Query) -> Result<ExecuteResult, Error>;
    async fn query(&self, query: &Query) -> Result<Rows, Error>;
    async fn transaction(&self) -> Result<Transaction<'_, Self>, Error>;
    fn is_autocommit(&self) -> bool;
    fn set_autocommit(&mut self, value: bool) -> Result<(), Error>;
    fn close(self) -> Result<(), Error>;
}
```

- **`execute`** runs a statement (INSERT, UPDATE, DELETE, DDL) and returns the number of affected rows or other output.
- **`query`** runs a SELECT and returns an owned `Rows` result set.
- **`transaction`** begins a new transaction and returns a `Transaction` handle; the transaction is committed on successful `Dispose` or rolled back on drop.
- **`is_autocommit`** / **`set_autocommit`** control the connection’s autocommit mode.
- **`close`** consumes the connection, performing an orderly shutdown.

---

## 4. `Transaction` Handle

```
pub struct Transaction<'a, C: Connection> {
    conn: &'a mut C,
    committed: bool,
}

impl<'a, C: Connection> Transaction<'a, C> {
    pub async fn commit(self) -> Result<(), Error>;     // commits explicitly
    pub async fn rollback(self) -> Result<(), Error>;   // rolls back explicitly
}

impl<'a, C: Connection> Dispose for Transaction<'a, C> {
    fn dispose(&mut self) {
        if !self.committed {
            // rollback automatically on cancel
        }
    }
}
```

- A transaction is created by `Connection::transaction()`.  While the transaction is alive, all queries on the underlying connection are executed within the transaction.
- If `commit` or `rollback` are not called explicitly, the transaction is rolled back on drop.

---

## 5. `Query` and Parameter Binding

### 5.1 `Query`

```
pub struct Query {
    sql: Text,
    params: Vec<Value>,
}
```

- A `Query` is a SQL statement with optional positional parameters (represented by `?` or `$1` depending on backend).
- Constructed via `Query::new(sql: &str)` and users add parameters using methods:
  - `pub fn bind<T: ToValue>(&mut self, value: T) -> &mut Self;`

### 5.2 `ToValue` Trait

```
pub trait ToValue {
    fn to_value(&self) -> Value;
}
```

Implemented for all primitive types, `Text`, `Option<T>`, `Vec<u8>`, etc.

---

## 6. `Rows` and `Row`

### 6.1 `Rows`

```
pub struct Rows {
    columns: Vec<ColumnInfo>,
    data: Vec<Row>,
    affected: u64,
}

impl Rows {
    pub fn columns(&self) -> &[ColumnInfo];
    pub fn iter(&self) -> impl Iterator<Item = &Row>;
    pub fn len(&self) -> usize;
    pub fn affected_rows(&self) -> u64;
}
```

- `columns` describes the schema of the result set.
- `iter` yields individual rows.
- For non‑SELECT statements, `affected_rows` is populated; `data` is empty.

### 6.2 `Row`

```
pub struct Row { values: Vec<Value> }

impl Row {
    pub fn get<T: FromValue>(&self, index: usize) -> Result<T, Error>;
    pub fn get_by_name<T: FromValue>(&self, name: &str) -> Result<T, Error>;
}
```

- `get` retrieves a value by 0‑based column index, performing type conversion.
- `get_by_name` uses column name; panics if name not found.

### 6.3 `FromValue` Trait

```
pub trait FromValue: Sized {
    fn from_value(value: &Value) -> Result<Self, Error>;
}
```

Implemented for primitive types, `Text`, `Vec<u8>`, `Option<T>`, `bool`, integers, floats, and date‑time types (if supported by backend).

---

## 7. `Value` (Dynamic Type)

```
pub enum Value {
    Null,
    Bool(bool),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    F32(f32),
    F64(f64),
    Text(Text),
    Bytes(Vec<u8>),
    Array(Vec<Value>),      // for arrays, not all backends support
    // … other types as needed
}
```

- All values are owned; `Value` is linear (requires explicit disposal).
- Implements `Debug`, `Display`, `PartialEq`, `Eq` (where applicable).
- The module also provides conversion functions between `Value` and Blaze’s standard types via `From`/`Into` traits.

---

## 8. Column Information

```
pub struct ColumnInfo {
    name: Text,
    type_info: TypeInfo,
    nullable: bool,
}

pub enum TypeInfo {
    Bool,
    TinyInt,
    SmallInt,
    Integer,
    BigInt,
    Float,
    Double,
    Text,
    Blob,
    Null,
    Other(Text),
}
```

- `ColumnInfo` is returned from `Rows::columns()` and can be used for dynamic schema inspection.

---

## 9. Error Handling

```
pub enum Error {
    ConnectionFailed(Text),
    QueryError(Text),
    DecodeError(Text),
    EncodeError(Text),
    TransactionError(Text),
    Protocol(Text),
    Io(std::io::Error),
}
```

Each backend maps its native error codes into this unified error type.

---

## 10. Implementation Notes

- The `blaze‑sql` crate itself does **not** contain any database engine; it only provides the generic API.  Concrete implementations are in `blaze‑sqlite`, `blaze‑postgres`, and `blaze‑redis` (for Redis as a key‑value store with SQL‑like query).
- Connection pooling is **not** part of this crate; it belongs to a higher‑level `blaze‑sql‑pool` crate, which manages a pool of `Connection` objects using actors.
- The crate uses `std::future` and `async`/`await` for all potentially blocking operations.  Drivers must integrate with the Blaze runtime’s async I/O.
- All resource management (connections, transactions, result sets) uses linear types with `Dispose` to ensure proper cleanup, even in the presence of cancellations.

---

## 11. Testing

- **Unit tests:** The crate provides a mock `Database` implementation that returns synthetic data.  This allows testing the core API without an actual database.
- **Integration tests for each driver:** Each concrete driver crate (`blaze‑sqlite`, etc.) includes integration tests that spin up a temporary database, execute schema creation, insert/update/delete, and query back, verifying the round‑trip of typed values.
- **Error handling:** Test that invalid queries return appropriate errors, and that connection failures are handled correctly.
- **Transaction semantics:** Verify that a `Transaction` rollback on drop undoes changes, and that explicit commit persists them.

All tests must pass before the next ecosystem crate’s specification is complete.
