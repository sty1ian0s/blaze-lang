# Phase 5 – Ecosystem Crate: `blaze‑config`

> **Goal:** Specify the `blaze‑config` crate, which provides a data‑oriented, layered configuration system for Blaze applications.  It supports loading configuration values from multiple sources (files, environment variables, command‑line arguments, and programmatic defaults) and merging them into a single, strongly‑typed configuration struct.  The crate leverages Blaze’s macro and reflection capabilities to derive parsers automatically, and integrates with `blaze‑serde` for file format support (JSON, YAML, TOML).  All file I/O carries the `io` effect; pure in‑memory operations are pure.

---

## 1. Core Concepts

Configuration is represented by a user‑defined struct annotated with `@config`.  Each field corresponds to a configuration key, and can be populated from multiple sources according to a priority order (e.g., command‑line → environment → file → default).  The crate provides a builder that:

1. Loads from the lowest‑priority source (e.g., a default file).
2. Overrides with values from higher‑priority sources (e.g., environment variables, arguments).
3. Finally applies hard‑coded defaults from the struct’s `Default` implementation.

All values are strongly typed (strings, integers, floats, booleans, durations, paths, arrays).  Custom deserialization functions can be specified for complex types.

---

## 2. `@config` Macro

The `@config` attribute is applied to a struct to derive the configuration loading logic.  It generates:

- A `ConfigLoader` for the struct with methods to add sources and build the final configuration.
- A `ConfigError` type specific to that configuration if not already defined.
- The necessary `Deserialize` implementations if the struct implements `blaze‑serde`.

```
@config
struct AppConfig {
    pub host: String,
    pub port: u16,
    pub database_url: String,
    pub max_connections: u32 = 100,      // default value
    pub enable_caching: bool = true,
    pub log_level: String = "info",
    pub rate_limit: Option<f64>,
}
```

The generated loader will:

- Read from a TOML/YAML/JSON file: any key that matches a field name (or a specified rename) is deserialized.
- Read from environment variables: `APP_HOST`, `APP_PORT`, etc. (prefix derived from struct name or explicit).
- Read from command‑line arguments: `--host`, `--port`, `--enable‑caching`, etc.

---

## 3. Configuration Sources

### 3.1 `ConfigSource` Enum

```
pub enum ConfigSource {
    File(Path),
    Environment(Text),          // prefix for env vars
    CommandLine,                // uses the current process arguments
    Map(Map<Text, Text>),       // in‑memory key‑value pairs
    Defaults,
}
```

- Multiple sources can be added to the loader; they are processed in order (the last source added wins).
- `File` reads the specified file and auto‑detects format from extension (`.json`, `.yaml`, `.toml`).
- `Environment` reads all env vars starting with the given prefix, strips the prefix, lowercases, and maps to field names.
- `CommandLine` parses the program arguments; flags/options are matched to field names (kebab‑case matches snake_case).

### 3.2 `ConfigLoader<T>`

```
pub struct ConfigLoader<T: Config> {
    sources: Vec<ConfigSource>,
    override_map: Map<Text, Text>,
}

impl<T: Config + DeserializeOwned> ConfigLoader<T> {
    pub fn new() -> Self;
    pub fn add_source(mut self, source: ConfigSource) -> Self;
    pub fn set_override(mut self, key: &str, value: &str) -> Self;
    pub fn build(self) -> Result<T, ConfigError>;
}
```

- `build` processes all sources in order and returns the final configuration struct.
- `set_override` allows programmatic overrides after all sources, useful for test‑specific settings.

---

## 4. Field Attributes

Within the `@config` struct, individual fields can be annotated to control parsing:

- `@config(name = "custom_name")` – maps this field to a different key name in files and env vars.
- `@config(default = 42)` – explicitly sets a default (overrides `Default` trait).
- `@config(env_only)` – only read from environment variables, never from files.
- `@config(file_only)` – never read from environment or args.
- `@config(deserialize_with = "path::to::function")` – custom deserialization function.
- `@config(merge)` – for nested structs, merges recursively (by default sub‑configs are replaced whole).

---

## 5. Error Handling

```
pub enum ConfigError {
    Io(std::io::Error),
    FileFormat(Text),
    MissingField(Text),
    InvalidValue(Text),
    Deserialization(Text),
    EnvVarNotFound(Text),
    CommandLineParse(Text),
    DuplicateSource(Text),
}
```

- `MissingField` is returned when a required field (no default) is not found in any source.
- `InvalidValue` is returned when a string cannot be parsed into the field’s type (e.g., `"abc"` for `u16`).

---

## 6. Environment Variable and CLI Parsing

### 6.1 Environment Variable Mapping

- Prefix: by default the struct name is converted to SCREAMING_SNAKE_CASE and used as prefix.  This can be overridden via `@config(prefix = "MYAPP")`.
- Field names are converted to SCREAMING_SNAKE_CASE and appended: `max_connections` → `MAX_CONNECTIONS`.  The full environment variable is `<PREFIX>_<FIELD>`.
- For nested structs, the nesting is flattened with underscores.

### 6.2 Command‑Line Argument Mapping

- Arguments are matched by kebab‑case name: `max‑connections` → `max_connections`.
- Boolean flags can be set via presence: `--enable‑caching` sets to `true`, `--no‑enable‑caching` sets to `false`.
- Values with spaces must be quoted: `--host="localhost"`.
- The parser understands `--help` and can print usage (via `blaze‑cli` if integrated), but that’s optional.

---

## 7. File Loading

The crate integrates with `blaze‑serde` for file deserialization.  By default, it tries to load the file in the following order of formats (based on extension):

- `.toml` → `blaze‑toml`
- `.yaml` → `blaze‑yaml`
- `.json` → `blaze‑json`

The file is read completely into memory and then deserialized.  For very large configs, a streaming mode could be added but is out of scope for version 1.0.

---

## 8. Testing

- **Defaults:** Define a config struct with defaults, build without sources, verify defaults.
- **File loading:** Write a temporary TOML file, add it as source, verify values override defaults.
- **Environment variables:** Set environment variables, verify they override file values.
- **Command‑line arguments:** Pass arguments to the test process (or mock the argument list), verify they override everything.
- **Custom deserialization:** Use a field with a custom deserialize function that converts a string to a custom type.
- **Error handling:** Create a config file with a malformed value, expect `InvalidValue`.  Remove a required field, expect `MissingField`.

All tests must pass on all platforms.
