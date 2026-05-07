# Phase 4 – Ecosystem Crate: `blaze‑sqlite`

> **Goal:** Specify the `blaze‑sqlite` crate, which provides a concrete SQLite driver implementing the `blaze‑sql` traits.  It wraps the SQLite C library (`libsqlite3`) via unsafe FFI and exposes a safe, async, data‑oriented API that integrates with Blaze’s actor runtime and linear type system.  All I/O operations carry the `io` effect and are executed on a dedicated blocking thread pool to avoid blocking the async event loop.

---

## 1. Dependencies

- `blaze‑sql` – the generic SQL interface (provides `Database`, `Connection`, `Row`, `Value`, etc.)
- `extern "C"` bindings to `libsqlite3` – compiled as part of the crate or linked dynamically.

---

## 2. `Sqlite` – The Database Type

```
pub struct Sqlite;

impl Database for Sqlite {
    type Connection = SqliteConnection;
    fn connect(uri: &str) -> Result<SqliteConnection, Error>;
}
```

- `connect(uri)` opens an SQLite database file.  For in‑memory databases, the URI `":memory:"` is used.  It returns a linear `SqliteConnection`.

---

## 3. `SqliteConnection`

### 3.1 Struct

```
pub struct SqliteConnection {
    db: *mut sqlite3,           // raw pointer (owned)
    is_open: bool,
}
```

- This type is **linear**: it cannot be cloned or copied.  The `Dispose` impl calls `sqlite3_close` and sets `is_open = false`.

### 3.2 Constructors

```
impl SqliteConnection {
    pub fn new(db: *mut sqlite3) -> SqliteConnection;
}
```

- `new` takes ownership of the raw pointer.  It is called internally by `Sqlite::connect`.

### 3.3 `Connection` Implementation

```
impl Connection for SqliteConnection {
    type Database = Sqlite;

    async fn execute(&self, query: &Query) -> Result<ExecuteResult, Error>;
    async fn query(&self, query: &Query) -> Result<Rows, Error>;
    async fn transaction(&self) -> Result<Transaction<'_, Self>, Error>;
    fn is_autocommit(&self) -> bool;
    fn set_autocommit(&mut self, value: bool) -> Result<(), Error>;
    fn close(self) -> Result<(), Error>;
}
```

- **`execute`**: prepares and steps through the statement once (or multiple times for batched statements).  It returns `ExecuteResult` which contains the number of rows modified.
- **`query`**: prepares the statement, binds parameters, and collects all rows into a `Rows` structure.  It then finalizes the statement.
- **`transaction`**: begins a `BEGIN` transaction, and returns a `SqliteTransaction` that borrows the connection.
- **`is_autocommit` / `set_autocommit`**: by default SQLite operates in autocommit; `set_autocommit(false)` starts a deferred transaction.
- **`close`**: consumes the connection by calling `sqlite3_close` and disposing.

All I/O operations are performed synchronously on the SQLite C API, but the `async` methods dispatch the work to a global blocking thread pool using `blaze::async::spawn_blocking`.  This ensures the async runtime is not blocked.

---

## 4. `SqliteTransaction`

```
pub struct SqliteTransaction<'a> {
    conn: &'a mut SqliteConnection,
    committed: bool,
}

impl<'a> SqliteTransaction<'a> {
    pub async fn commit(self) -> Result<(), Error>;
    pub async fn rollback(self) -> Result<(), Error>;
}

impl<'a> Dispose for SqliteTransaction<'a> {
    fn dispose(&mut self) {
        if !self.committed {
            // send rollback command synchronously, ignore error
        }
    }
}
```

- Rust‑style RAII transaction: if explicitly committed, no rollback; otherwise rollback on dispose.

The `Transaction` type from `blaze‑sql` is a thin wrapper around this driver‑specific transaction, allowing the use of the trait method `Connection::transaction()`.

---

## 5. Parameter Binding and Result Extraction

### 5.1 Binding

The crate implements the low‑level binding of `Value` variants to SQLite statement parameters.  For each parameter in the query, `sqlite3_bind_*` is called with the appropriate type (e.g., `sqlite3_bind_int64` for `i64`, `sqlite3_bind_text` for `Text`, etc.).  Null values are bound with `sqlite3_bind_null`.

### 5.2 Extraction

When iterating over rows, the driver calls `sqlite3_column_*` functions and constructs `Value` variants.  Type information from `sqlite3_column_type` is used to ensure safe conversions.  If a conversion is not possible (e.g., retrieving an integer from a text column), an `Error::DecodeError` is returned.

The `FromValue` trait is implemented for all supported Blaze types by delegating to these extraction functions.

---

## 6. Error Handling

The error type is `blaze::sql::Error`, which this crate instantiates with the following mapping:

- `SQLITE_CONSTRAINT` → `Error::QueryError("constraint violation")`
- `SQLITE_BUSY` → `Error::Protocol("database is busy")`
- `SQLITE_IOERR` → `Error::Io(...)`
- `SQLITE_CORRUPT` → `Error::Protocol("database disk image is malformed")`
- etc.

The error message is taken from `sqlite3_errmsg(db)`.

---

## 7. Resource Management

All allocated resources (prepared statements, result sets) are correctly finalized and freed.  The `SqliteConnection` ensures that `sqlite3_close` is called exactly once, even if `Dispose` is called multiple times (idempotent).  The raw pointer is set to `null` after close.

---

## 8. Implementation Notes

- The FFI bindings are in a private module `ffi` that imports the necessary C functions from `sqlite3.h`.  They are wrapped with `unsafe` and exposed as safe methods where the invariants are maintained (e.g., non‑null db pointer).
- The blocking thread pool is provided by the Blaze runtime; `spawn_blocking` returns a `Future<Output = Result<T, Error>>` that runs the synchronous work and wakes the task when done.  This is the recommended pattern for CPU‑bound or third‑party I/O.
- The crate uses `@cfg` to optionally enable SQLite extensions (like FTS5, JSON1) via compile‑time features.

---

## 9. Testing

- **In‑memory database:** Connect to `:memory:`, create a table, insert rows, query them back and verify values.
- **Error handling:** Try inserting a duplicate primary key, verify `Error::QueryError`.
- **Transaction:** Start a transaction, insert a row, rollback, and confirm no row exists.  Commit another transaction and confirm the row persists.
- **Type round‑trip:** Insert an integer, a float, a text, and a blob, then retrieve them and verify the correct `Value` variants.
- **Null handling:** Insert `Value::Null` and read back as `Option<i32>` → `None`.
- **Connection lifecycle:** Ensure that dropping a `SqliteConnection` calls `sqlite3_close`, and that any subsequent use of the raw pointer would cause a panic (by checking `is_open` flag).
- All tests must pass on platforms where SQLite is available (most).
