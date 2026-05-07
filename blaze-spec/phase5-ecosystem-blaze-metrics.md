# Phase 5 – Ecosystem Crate: `blaze‑metrics`

> **Goal:** Specify the `blaze‑metrics` crate, which provides a data‑oriented, low‑overhead metrics collection and export framework for Blaze applications.  It supports counters, gauges, histograms, and summaries, with push and pull export to various backends (Prometheus, InfluxDB, Graphite, and a simple text logger).  All metric operations are pure and designed to be extremely fast (`O(1)` recording), with no allocation in the hot path.  The crate integrates with the Blaze actor model for application‑wide metrics aggregation and with `blaze‑log` for structured logging of metric events.

---

## 1. Core Concepts

The crate provides:

- **`Metric`** – a trait for all metric types.
- **`Counter`**, **`Gauge`**, **`Histogram`**, **`Summary`** – concrete metric types.
- **`MetricsRegistry`** – a global registry of named metrics, with thread‑safe operations.
- **`MetricsRecorder`** – a trait implemented by export backends.
- **`MetricsConfig`** – compile‑time and runtime configuration for labels, buckets, and export.

All metric updates are `O(1)` and non‑blocking.  Histograms use logarithmic bucket sizes; summaries use streaming quantile estimation (e.g., T‑Digest).

---

## 2. Metric Types

### 2.1 `Counter`

```
pub struct Counter { value: AtomicU64 }
impl Counter {
    pub fn new() -> Counter;
    pub fn inc(&self, delta: u64);
    pub fn get(&self) -> u64;
    pub fn reset(&self);
}
```

- Monotonically increasing counter (or resettable).  `inc` is atomically incrementing, no locks.

### 2.2 `Gauge`

```
pub struct Gauge { value: AtomicI64 }
impl Gauge {
    pub fn new() -> Gauge;
    pub fn set(&self, value: i64);
    pub fn inc(&self, delta: i64);
    pub fn dec(&self, delta: i64);
    pub fn get(&self) -> i64;
}
```

- Represents a single value that can go up or down.

### 2.3 `Histogram`

```
pub struct Histogram {
    buckets: Vec<AtomicU64>,       // cumulative bucket counts
    sum: AtomicF64,
    count: AtomicU64,
    config: HistogramConfig,
}
impl Histogram {
    pub fn new(config: HistogramConfig) -> Histogram;
    pub fn observe(&self, value: f64);
    pub fn buckets(&self) -> &[u64];   // snapshot (not purely instantaneous, but good enough)
    pub fn sum(&self) -> f64;
    pub fn count(&self) -> u64;
}
```

- `HistogramConfig` contains `bucket_limits: Vec<f64>` (upper bounds of each bucket).  Common config: `linear(0.0, 1.0, 10)`, `exponential(1.0, 2.0, 20)`.

### 2.4 `Summary`

```
pub struct Summary {
    quantiles: Vec<(f64, f64)>,    // (quantile, value) computed by algorithm
    sum: AtomicF64,
    count: AtomicU64,
    algorithm: Box<dyn SummaryAlgorithm>,
}
impl Summary {
    pub fn new(config: SummaryConfig) -> Summary;
    pub fn observe(&self, value: f64);
    pub fn quantiles(&self) -> &[(f64, f64)]; // snapshot
}
```

- `SummaryConfig` specifies the quantiles to track (e.g., `[0.5, 0.9, 0.99]`) and the algorithm parameters (e.g., T‑Digest compression factor).  The default algorithm is T‑Digest.

---

## 3. Metrics Registry

### 3.1 `MetricsRegistry`

```
pub struct MetricsRegistry {
    global: OnceLock<HashMap<Text, Box<dyn Metric>>>,
}
```

- The global registry is initialised lazily and is thread‑safe.  Access is via `MetricsRegistry::global()`.
- Methods:
  - `pub fn register<M: Metric + 'static>(name: &str, metric: M) -> Result<(), MetricsError>;` – registers a metric with the given name; fails if already registered.
  - `pub fn counter(name: &str) -> Option<&'static Counter>;` – retrieves a registered counter by name.
  - `pub fn gauge(name: &str) -> Option<&'static Gauge>;`
  - `pub fn histogram(name: &str) -> Option<&'static Histogram>;`
  - `pub fn summary(name: &str) -> Option<&'static Summary>;`
  - `pub fn collect(&self) -> Vec<MetricSnapshot>;` – collects a snapshot of all metrics for export.

### 3.2 `MetricSnapshot`

```
pub enum MetricSnapshot {
    Counter { name: Text, value: u64, labels: Map<Text, Text> },
    Gauge { name: Text, value: f64, labels: Map<Text, Text> },
    Histogram { name: Text, buckets: Vec<u64>, sum: f64, count: u64, labels: Map<Text, Text> },
    Summary { name: Text, quantiles: Vec<(f64, f64)>, sum: f64, count: u64, labels: Map<Text, Text> },
}
```

- Each snapshot carries labels that identify the metric and its dimensions (e.g., `path="/api/users"`, `method="GET"`).

---

## 4. Labels and Cardinality

The crate supports labeled metrics: a single metric name with multiple dimensions defined by key‑value pairs.  Labels are specified at registration time and are stored in a `Vec<Text>` per metric instance.

For example, a counter with label `path` can have multiple instances: `counter("http_requests")` with label `path="/"` and another with `path="/login"`.  The registry maintains a separate counter for each label combination; labels are interned to avoid string allocation in the hot path (they are stored as `&'static Text` after first creation).

Creation of labeled metrics is done via `registry.labeled_counter(name, labels)` which returns a handle to the specific instance, creating it if needed.

---

## 5. Export Backends

### 5.1 `MetricsRecorder` Trait

```
pub trait MetricsRecorder: Send + Sync + 'static {
    fn record(&self, snapshot: &[MetricSnapshot]);
}
```

- Backends implement this trait to export metrics.

### 5.2 Built‑in Backends

- **`PrometheusRecorder`** – starts an HTTP server on a configurable port (e.g., 9090) that exposes `/metrics` in Prometheus text format.  This is implemented using `blaze‑http` and runs as an actor.
- **`InfluxDbRecorder`** – pushes metrics to an InfluxDB instance via its HTTP API.
- **`GraphiteRecorder`** – connects to a Graphite/Carbon plaintext protocol.
- **`LogRecorder`** – logs each snapshot as a structured log line using `blaze‑log`.
- **`CsvRecorder`** – appends snapshots to a CSV file, suitable for offline analysis.

Backend selection is done at startup, and multiple backends can be chained via `CompositeRecorder`.

---

## 6. Actor Model Integration

The crate provides a `MetricsActor` that aggregates metrics across an application's actors and exposes them to exporters.  Each actor can create its own local metrics (counters, gauges) and then send them to the central metrics actor via a channel, ensuring low contention and no global locks.

```
pub actor MetricsActor {
    registry: MetricsRegistry,
    recorders: Vec<Box<dyn MetricsRecorder>>,
    push_interval: Duration,
}
```

- The actor runs a periodic timer that calls `registry.collect()` and pushes snapshots to all recorders.  It also listens for external pull requests (e.g., Prometheus scrape) and serves the latest snapshot.

---

## 7. Error Handling

```
pub enum MetricsError {
    DuplicateMetric,
    InvalidLabel,
    ExporterError(Text),
    Io(std::io::Error),
}
```

- Registration errors are returned immediately.  Export errors are logged and do not affect the application.

---

## 8. Implementation Notes

- All metric types use atomics for the hot path, ensuring lock‑free updates.  Histogram bucket arrays are allocated once and never resized, enabling safe concurrent reads without a mutex.
- The global registry uses a `RwLock` for registration, but metric recording after registration is lock‑free.
- Snapshot collection iterates over all registered metrics and creates `MetricSnapshot` values; this is done by the metrics actor or explicitly by the user, not in the hot path.
- Labels are interned using a global string table that maps label keys/values to unique `Text` handles, so that label comparison is just pointer equality in the registry.

---

## 9. Testing

- **Counter/Gauge:** Increment and check value; test concurrent increments from multiple threads (if Blaze's thread pool is available) and verify final value.
- **Histogram:** Observe known values, snapshot buckets, verify counts in correct buckets.
- **Summary:** Observe a stream of values, check that quantiles approximate the expected distribution.
- **Registry:** Register multiple metrics, retrieve by name, verify snapshot collection.
- **Labeling:** Create labeled counters with different label values, verify they are independent.
- **Export:** Set up a `LogRecorder`, collect snapshot, verify the log output contains the metric name and value.
- **Performance:** Measure that recording a counter takes a few CPU cycles (via benchmark) and does not allocate.

All tests must pass on all platforms.
