# Phase 4 – Ecosystem Crate: `blaze‑log`

> **Goal:** Provide a structured, performant, data‑oriented logging framework for Blaze applications.  It supports multiple log levels, structured key‑value pairs, compile‑time filtering, multiple output backends (console, file, syslog, network), and integration with the actor model.  All logging is designed to have minimal overhead when disabled and to be zero‑cost for message formatting when the log level is not met.

---

## 1. Core Concepts

The crate provides:

- **`log!` macro** – the primary entry point for logging, accepting a level and a message with key‑value pairs.
- **`Logger`** – a trait implemented by log backends (console, file, syslog, network, etc.).
- **`Level`** – an enum representing standard log levels.
- **`LogRecord`** – a data‑oriented struct that represents a single log event, carrying timestamp, target, module, line, level, message, and structured fields.
- **`LoggerConfig`** – compile‑time and runtime configuration for filters, format, and output.

All logging operations are pure unless the backend itself performs I/O (which is captured by the backend's effect).  When no logger is installed, logs are silently discarded.

---

## 2. Log Levels

```
pub enum Level {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
    Fatal,
}
```

- Log levels are ordered: `Trace < Debug < Info < Warn < Error < Fatal`.
- At compile time, the crate respects a per‑module `max_level` attribute, which elides entire call sites when the level is too low:
  - `@log_level(max = Level::Info)` on a module or crate root.

---

## 3. The `log!` Macro

```
log!(level: Level, msg: &str, key1: val1, key2: val2, …);
```

- Example: `log!(Info, "connection established", port: 443, protocol: "HTTPS");`
- The macro compiles to a check against the current max log level; if the level is sufficient, it evaluates the message and key‑value pairs and dispatches to the installed logger.
- Message formatting is deferred: the `LogRecord` stores the raw message string and a slice of key‑value pairs; the backend decides how to format them.  This avoids allocation during formatting.

---

## 4. `LogRecord`

```
pub struct LogRecord<'a> {
    pub timestamp: Instant,
    pub level: Level,
    pub target: &'a str,          // module path, e.g. "blaze::net::tcp"
    pub module: &'a str,          // file!()
    pub line: u32,
    pub message: &'a str,
    pub fields: &'a [LogField<'a>],
}

pub struct LogField<'a> {
    pub key: &'a str,
    pub value: LogValue<'a>,
}

pub enum LogValue<'a> {
    Str(&'a str),
    Int(i64),
    Float(f64),
    Bool(bool),
    Text(&'a Text),
    Bytes(&'a [u8]),
    Error(&'a dyn Error),
    Debug(&'a dyn Debug),
    Display(&'a dyn Display),
}
```

- `LogRecord` is `@copy` (contains only references and small values).  It is the data‑oriented representation of a log event, enabling backends to serialise, filter, or route without heap allocation.

---

## 5. Logger Trait

```
pub trait Logger: Send + Sync + 'static {
    fn log(&self, record: &LogRecord);
    fn flush(&self);  // ensure all buffered records are written
}
```

- Loggers are registered globally or per‑actor via `set_logger` and `set_global_logger`.
- The global logger is stored in a static `OnceLock<Box<dyn Logger>>` initialised once.

### 5.1 Built‑in Loggers

- **`ConsoleLogger`** – writes coloured, human‑readable output to `stderr`.  Respects a `format` option: `compact`, `pretty`, `json`.
- **`JsonLogger`** – writes one JSON object per line (NDJSON) to `stdout` or a file.  Suitable for structured log analysis.
- **`FileLogger`** – appends plain‑text or JSON to a file, with optional rotation (by size or time).  Created with `FileLogger::new(path, rotation)`.
- **`SyslogLogger`** – sends log events to the system syslog daemon (Unix only, via `std::os::unix`).
- **`NullLogger`** – discards all messages; used as default until a logger is installed.
- **`CompositeLogger`** – holds a `Vec<Box<dyn Logger>>` and forwards to all sub‑loggers.

---

## 6. Log Configuration

```
pub struct LogConfig {
    pub level: Level,
    pub format: LogFormat,
    pub filters: Vec<LogFilter>,
    pub max_level_per_module: Map<Text, Level>,
}

pub enum LogFormat { Compact, Pretty, Json }
pub struct LogFilter { pub key: Text, pub value: Text }
```

- `LogConfig` can be set at startup via `blaze‑config` or `blaze.toml`.
- Filters allow conditional logging: e.g., only log events from target `"blaze::net"` with level >= `Debug`.

---

## 7. Actor Integration

The crate provides an `ActorLogger` wrapper that forwards log records from within an actor to a channel, allowing a dedicated logging actor to avoid I/O on worker threads.

```
pub struct ActorLogger {
    sender: Sender<LogRecord>,
}
impl Logger for ActorLogger { … }
```

- The `ActorLogger` owns a `Sender`; a dedicated logger actor receives records, formats them, and writes to the actual backend.  This decouples the hot path from slow I/O.

---

## 8. Compile‑time Filtering

The `@log_level(max = Level::Info)` attribute uses compile‑time reflection to automatically strip `log!` calls below the specified level.  This is separate from the runtime filter and ensures zero cost for disabled traces.

---

## 9. Error Handling

The crate itself produces no errors (logging failures are silently ignored by default).  The `FileLogger` may panic if it cannot open the log file (this is intentional: startup errors should be fatal).  There is an option to set a fallback logger in case of failure.

---

## 10. Testing

- **All levels:** Use `log!` at each level; verify the message is passed to a mock logger.
- **Compile‑time filter:** Set `@log_level(max = Info)` and verify that `log!(Debug, …)` does not call the logger at all (by checking the mock is not invoked).
- **JSON output:** Use `JsonLogger` and capture output; parse back and verify the fields are present.
- **File logging:** Log to a temp file, read back, verify content.
- **Actor logging:** Spawn a logger actor, send log records, verify they are received and formatted.
- **Performance:** Measure throughput with and without logging enabled; ensure the no‑op path is negligible (<1 ns per call).

All tests must pass on all platforms; syslog tests are Unix‑only.
