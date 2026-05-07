# Phase 4 – Ecosystem Crate: `blaze‑cli`

> **Goal:** Provide a data‑oriented, declarative framework for building command‑line interfaces in Blaze.  It leverages the `@cli` macro to automatically derive argument parsers, subcommands, and help text from a user‑defined struct.  The crate produces zero‑cost abstractions: argument parsing is monomorphised and requires no heap allocation beyond the final configuration struct.  All I/O (printing help, reading stdin) carries the `io` effect, but parsing itself is pure.

---

## 1. Core Concepts

A CLI application is defined by a top‑level struct annotated with `@cli`.  Each field corresponds to an option, flag, positional argument, or subcommand.  The `@cli` macro generates:

- A `CliParser` that can be used to parse `std::env::args()` into the struct.
- A `CliHelp` impl that generates and prints help text.
- `From<Vec<Text>>` (or similar) that performs the actual parsing.

The crate integrates with `blaze‑config` so that CLI arguments override values from config files and environment variables.

---

## 2. The `@cli` Macro

### 2.1 Basic Usage

```
@cli(name = "myapp", version = "1.0", about = "Does wonderful things")
struct App {
    /// Host to connect to
    host: String,
    /// Port number
    port: u16,
    /// Enable verbose output
    #[short('v')]
    verbose: bool,
    /// Config file path
    #[long("config-file")]
    config_file: Option<String>,
    /// Subcommand
    #[subcommand]
    command: Option<Command>,
}
```

- The struct name becomes the application; the `name`, `version`, `about` attributes are used in the help header.
- Fields are positional by default (if no `#[long]` or `#[short]` specified), and required unless they have a default or are `Option<T>`.
- Boolean flags (type `bool`) are set to `true` if present, unless a `--no-` prefix variant is used.
- `Option<T>` fields are optional; their presence sets `Some(value)`, absence leaves `None`.
- `#[subcommand]` indicates an enum field that represents nested subcommands (see below).

### 2.2 Field Attributes

- `#[long("my‑name")]` – long option name (used with `--my‑name`).
- `#[short('m')]` – short option name (used with `-m`).
- `#[default = value]` – default value if the argument is not provided (overrides `Default` trait).
- `#[arg_name = "NAME"]` – placeholder name in help text.
- `#[help = "text"]` – custom help text (overrides doc comment).
- `#[required]` – marks an option as required even if it has a default.
- `#[multiple]` – for `Vec<T>`, expects multiple occurrences (`-a 1 -a 2`).
- `#[conflicts_with = "other_field"]` – prevents simultaneous use.
- `#[requires = "other_field"]` – this option requires another option to be present.

---

## 3. Subcommands

Subcommands are represented by an enum where each variant is a separate command.  `@cli` derives subcommand parsing automatically.

```
enum Command {
    /// Run the server
    Run {
        /// Port to listen on
        port: u16,
    },
    /// Show configuration
    Config,
    /// Manage users
    Manage {
        /// User operation
        #[subcommand]
        action: ManageAction,
    },
}

enum ManageAction {
    Add { name: String, email: String },
    Remove { name: String },
    List,
}
```

- The top‑level `App` can have a `#[subcommand]` field of type `Option<Command>`.  If the first argument matches a variant name (kebab‑case), it is parsed as that subcommand.  Subcommands can nest arbitrarily.
- Each variant’s fields are options specific to that subcommand.

---

## 4. Parsing and Execution

The generated `CliParser` provides:

```
pub struct CliParser<T: Cli> { … }
impl<T: Cli> CliParser<T> {
    pub fn new() -> Self;
    pub fn parse(self) -> Result<T, CliError>;
    pub fn parse_from(self, args: &[Text]) -> Result<T, CliError>;
    pub fn parse_with_config(self, config: ConfigLoader<T>) -> Result<T, CliError>;
    pub fn print_help(&self);
    pub fn print_version(&self);
}
```

- `parse()` reads from `std::env::args()` (ignoring the program name) and returns the parsed struct.  On error, it prints an error message and exits (or returns the error, depending on a setting).
- `parse_from` can be used for testing with a custom argument list.
- `parse_with_config` first loads from config files/environment, then overrides with CLI arguments.
- `print_help` writes the generated help to `stderr` (or a provided writer).
- `print_version` writes the program name and version.

---

## 5. Integration with `blaze‑config`

When used together, the `App` struct can be annotated with both `@config` and `@cli`:

```
@config
@cli(name = "myapp")
struct AppConfig { … }
```

The loading order is: defaults → config file → environment → CLI arguments.  The `@cli` parser acts as the highest‑priority source.  This is implemented by `parse_with_config`, which calls `ConfigLoader::build()` to get a partially filled struct and then merges the CLI values.

---

## 6. Help Generation

The help text is generated automatically from struct field names, doc comments, and attributes.  It includes:

- Usage line: `myapp [OPTIONS] [SUBCOMMAND]`
- Options section: list of all options with long/short names, arg placeholder, and help text.
- Subcommands section: list of available subcommands with short description.

The formatting uses 80‑column width, wrapping as needed.  Colours can be enabled via a feature `colors` (uses ANSI codes).

---

## 7. Error Handling

```
pub enum CliError {
    MissingRequiredOption(Text),
    InvalidValue(Text),
    UnrecognizedOption(Text),
    MissingSubcommand,
    InvalidSubcommand(Text),
    Io(std::io::Error),
    ConfigError(ConfigError),
}
```

- `MissingRequiredOption` is returned when a required field is not present.
- `InvalidValue` is returned when a string cannot be parsed into the field type.
- `UnrecognizedOption` is returned for unknown flags.
- The parser by default prints the error and exits, but a developer mode can be enabled to return the error for programmatic handling.

---

## 8. Type Support

The crate supports parsing the following types directly:

- Strings (`String`)
- Integers (`i8`..`i128`, `u8`..`u128`, `isize`, `usize`)
- Floats (`f32`, `f64`)
- Booleans (`bool`)
- Dates/times (via `std::time::Duration`, `SystemTime` – parsed from human‑readable strings like `"5s"`, `"2025‑01‑01"`)
- Paths (`PathBuf` from `std::path`)
- Enums (parsed from string variants)
- `Option<T>` for optional values
- `Vec<T>` for multiple values

Custom types can implement the `FromStr` trait (a standard Blaze trait for parsing from text) to be used.

---

## 9. Testing

- **Basic parsing:** Define a struct, parse known arguments, verify field values.
- **Subcommands:** Define a nested enum, parse subcommand, verify the correct variant and its fields.
- **Required and optional:** Test missing required options produce `MissingRequiredOption`.
- **Conflicts and requires:** Test conflicting options are rejected; required dependencies are enforced.
- **Help output:** Capture the help text, verify it contains the program name, options, and subcommands.
- **Integration with config:** Set a config file, set environment variable, provide CLI argument; verify CLI wins.
- **Error handling:** Provide invalid value (e.g., `"abc"` for integer), expect `InvalidValue`.

All tests must pass on all platforms.
