# Phase 5 – Ecosystem Crate: `blaze‑dataframe`

> **Goal:** Specify the `blaze‑dataframe` crate, which provides a high‑performance, data‑oriented, columnar data frame library for Blaze.  It is inspired by frameworks like Pandas and Polars but is built entirely on Blaze’s own foundations: `blaze‑tensor`, `blaze‑serde`, and the ECS storage model.  Data frames store columns in SoA layout for optimal cache usage, support zero‑copy slicing, lazy evaluation, and automatic parallelisation of operations.  I/O operations carry the `io` effect; all data transformations are pure.

---

## 1. Core Concepts

A **data frame** is a collection of named columns, each of which is a typed array (a `Series<T>`).  Columns can be numeric, string, categorical, datetime, or nested (list, struct).  The data frame is columnar: operations on a single column are vectorised and parallelised.  The crate provides:

- `DataFrame` – the central type, owning a set of columns.
- `Series<T>` – a generic, type‑erased column backed by a `Tensor<T, 1>`.
- `Column` – an enum over all supported `Series<T>` types for dynamic access.
- `GroupBy` – a builder for split‑apply‑combine operations.
- `Join` – several join algorithms (hash, sort‑merge) for combining data frames.
- `CsvReader`, `JsonReader`, `ParquetReader` (optional), `SqlReader` – for loading data.
- `DataFrameWriter` – for saving data.

All types are linear where they own memory.  Slicing produces `DataFrameView` or `SeriesView` that borrow data and are zero‑copy.

---

## 2. `Series<T>` and `Column`

### 2.1 `Series<T>`

```
pub struct Series<T: Numeric + 'static> {
    data: Tensor<T, 1>,
    name: Text,
}
```

- `T` can be any primitive numeric type, `bool`, `Text`, `DateTime`, or `Categorical<T>`.  (Text and Categorical are not `Numeric`, so we relax the trait to `SeriesElement` which is implemented for all supported types.)
- The underlying tensor is stored in row‑major order as a single contiguous array.  For string columns, the data is stored as a list of `Text` objects with a SoA layout for the text data (see below).
- `Series<T>` implements `Index<usize>`, `Iterator`, and arithmetic operations (for numeric types).  Methods: `len()`, `name()`, `rename()`, `slice()`, `cast<U>()`, `sort()`, `unique()`, `value_counts()`.

### 2.2 `Column` Enum

```
pub enum Column {
    Int8(Series<i8>),
    Int16(Series<i16>),
    Int32(Series<i32>),
    Int64(Series<i64>),
    UInt8(Series<u8>),
    UInt16(Series<u16>),
    UInt32(Series<u32>),
    UInt64(Series<u64>),
    Float32(Series<f32>),
    Float64(Series<f64>),
    Bool(Series<bool>),
    String(Series<Text>),
    DateTime(Series<DateTime>),
    Categorical(Series<Categorical>),
    List(Box<Column>),   // nested column
    Struct(Map<Text, Box<Column>>),
}
```

- `Column` provides dynamic dispatch for operations that work across types.  For performance‑critical paths, users should work with `Series<T>` directly.

---

## 3. `DataFrame`

### 3.1 Struct

```
pub struct DataFrame {
    columns: Map<Text, Column>,
    row_count: usize,
}
```

- Linear; `Dispose` drops all columns.
- `columns` is a B‑tree map for ordered column names.

### 3.2 Construction

```
impl DataFrame {
    pub fn new() -> DataFrame;
    pub fn from_columns(columns: Vec<(Text, Column)>) -> Result<DataFrame, DataError>;
    pub fn with_column(name: &str, column: Column) -> DataFrame;
    pub fn row_count(&self) -> usize;
    pub fn column_count(&self) -> usize;
    pub fn column_names(&self) -> Vec<&str>;
    pub fn get_column<T: SeriesElement>(&self, name: &str) -> Option<Series<T>>;
    pub fn get_column_dynamic(&self, name: &str) -> Option<&Column>;
    pub fn drop_column(mut self, name: &str) -> Self;
    pub fn rename_column(mut self, old: &str, new: &str) -> Self;
}
```

### 3.3 Slicing and Views

```
pub fn slice(&self, range: Range<usize>) -> DataFrameView;
pub fn head(&self, n: usize) -> DataFrameView;
pub fn tail(&self, n: usize) -> DataFrameView;
pub fn filter(&self, predicate: impl Fn(&DataFrameRow) -> bool) -> DataFrame;
pub fn select(&self, columns: &[&str]) -> DataFrameView;
```

- `DataFrameView` is a borrowed, zero‑copy slice of a data frame.  It implements all read‑only operations.  `filter` creates a new data frame with only the rows satisfying the predicate (copies data).
- `DataFrameRow` is a lightweight proxy providing access to a single row across all columns.

### 3.4 Aggregation and Grouping

```
pub fn group_by(&self, by: &[&str]) -> GroupBy;
pub fn agg(&self, agg_exprs: &[(Text, Aggregation)]) -> DataFrame;
pub fn sort(&self, by: &[&str], ascending: bool) -> DataFrame;
pub fn join(&self, other: &DataFrame, on: &[&str], how: JoinType) -> DataFrame;
```

- `Aggregation` enum: `Sum`, `Mean`, `Min`, `Max`, `Count`, `First`, `Last`, `Std`, `Var`, `Custom(fn(&Series<T>) -> T)`.
- `JoinType`: `Inner`, `Left`, `Right`, `Outer`, `Cross`.

### 3.5 Iteration

```
pub fn iter_rows(&self) -> impl Iterator<Item = DataFrameRow>;
pub fn iter_chunks(&self, chunk_size: usize) -> impl Iterator<Item = DataFrameView>;
```

- Row iteration is slow for large data; chunk iteration is preferred.  Each chunk is a `DataFrameView` that can be processed in parallel.

---

## 4. `GroupBy`

```
pub struct GroupBy {
    df: DataFrame,
    groups: Map<GroupKey, Vec<usize>>,
}
```

- Created by `DataFrame::group_by`.  The groups are computed eagerly (or lazily with a feature flag).
- Methods:
  - `pub fn agg(self, agg_exprs: &[(Text, Aggregation)]) -> DataFrame;`
  - `pub fn apply(self, f: impl Fn(DataFrameView) -> DataFrame) -> DataFrame;`

`apply` passes each group as a `DataFrameView` to the closure, which returns a new `DataFrame`; the results are concatenated.

---

## 5. Series and DataFrame Operations

### 5.1 Arithmetic

Numeric series support element‑wise arithmetic with scalars and other series of the same length: `+`, `-`, `*`, `/`, `%`, `pow`, `sqrt`, `exp`, `log`, etc.  Missing values (represented as `Option<T>` in a future extension, or via a separate mask) are not yet implemented; all values are required.

### 5.2 String Operations

`Series<Text>` supports: `contains`, `starts_with`, `ends_with`, `to_lowercase`, `to_uppercase`, `strip`, `replace`, `split`, `concat`, `length`.

### 5.3 DateTime Operations

`Series<DateTime>` supports: `year`, `month`, `day`, `hour`, `minute`, `second`, `weekday`, `to_timestamp`, `strftime`.

---

## 6. I/O

### 6.1 CSV Reader

```
pub struct CsvReader { path: Text, has_header: bool, delimiter: u8, … }

impl CsvReader {
    pub fn new(path: &str) -> CsvReader;
    pub fn delimiter(mut self, d: u8) -> Self;
    pub fn has_header(mut self, yes: bool) -> Self;
    pub fn read(self) -> Result<DataFrame, DataError>;
}
```

- Parsing is done using a hand‑written state machine for speed.  Column types are inferred by scanning the first N rows (configurable).
- The reader uses a buffered reader and can process files larger than memory by chunking.  A lazy `LazyCsvReader` returns an iterator of `DataFrameView` chunks.

### 6.2 JSON Reader

```
pub struct JsonReader { … }
impl JsonReader { … }  // parses newline‑delimited JSON (NDJSON) or array of objects.
```

### 6.3 Parquet Reader (optional, feature `parquet`)

```
pub struct ParquetReader { … }
impl ParquetReader { … }   // efficient columnar read with predicate pushdown.
```

### 6.4 SQL Reader

```
pub struct SqlReader { … }
impl SqlReader { … }   // connects via blaze‑sql, executes a query, returns a DataFrame.
```

### 6.5 Writing

```
impl DataFrame {
    pub fn to_csv(&self, path: &str) -> Result<(), DataError>;
    pub fn to_json(&self, path: &str) -> Result<(), DataError>;
    pub fn to_parquet(&self, path: &str) -> Result<(), DataError>;  // feature gated
}
```

- Writing is synchronous and carries the `io` effect.

---

## 7. Error Handling

```
pub enum DataError {
    Io(std::io::Error),
    InvalidSchema(Text),
    TypeMismatch(Text),
    MissingColumn(Text),
    DuplicateColumn(Text),
    ParseError(Text),
    LengthMismatch(Text),
    NotYetImplemented(Text),
}
```

---

## 8. Implementation Notes

- The internal storage of `Series<Text>` uses a SoA layout: the strings are stored in a single contiguous buffer (like `blaze::string::Text` but batch‑allocated).  A separate array of offsets allows O(1) indexing.  This makes string operations vectorisable.
- For categorical columns, the data is stored as a series of integer indices plus a dictionary of unique string values.  The dictionary is shared across slices.
- The `DataFrame` column map is `Map<Text, Column>` where each `Column` is an enum variant holding the series; dynamic operations match on the variant and delegate to the appropriate method.  For performance, common operations like `sort` and `filter` are specialised per type via compile‑time macros.
- Parallelism: all operations that iterate over rows (map, filter, agg) use `rayon`‑style work‑stealing via Blaze’s thread pool, but without an external dependency—they rely on `for` loops with pure bodies that the compiler auto‑parallelises (since Blaze’s standard library already provides a thread pool).
- The crate uses `@derive` to generate `Serialize` and `Deserialize` for saving/loading data frame schemas and data.

---

## 9. Testing

- **CSV round‑trip:** Load a CSV, write to CSV, load again, compare data frames.
- **Filter:** Create a data frame, filter rows with a predicate, verify row count.
- **GroupBy + agg:** Group by a column, compute sum of another, verify totals.
- **Join:** Create two data frames, join on a key, verify row count and content.
- **Series ops:** Add two numeric series, verify element‑wise sum.
- **String ops:** Create a string series, apply `to_uppercase`, verify results.
- **Large file:** Generate a CSV with 1 million rows, load it, ensure no out‑of‑memory and that chunked reading works.
- **Error handling:** Provide empty CSV, unexpected number of columns, etc., verify appropriate errors.

All tests must pass on all supported platforms.
