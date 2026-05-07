# Blaze Phase 3d – Specialized Library (Optional): Localisation (`std::l10n`)

> **Goal:** Define the optional `std::l10n` module for localisation and internationalisation.  This module provides types and functions for formatting messages according to locale‑specific rules, handling plural forms, number and date formatting, and locale selection.  It is designed to be data‑driven, with locale data loaded from compiled resource bundles at compile time or runtime.  All operations are pure or carry the `io` effect only when loading external resources.

---

## 1. Locale

### 1.1 `Locale`

```
pub struct Locale {
    language: Text,
    region: Option<Text>,
    script: Option<Text>,
    variant: Option<Text>,
}
```

- Represents a Unicode locale identifier as defined by BCP 47 (e.g., `"en‑US"`, `"fr‑CA"`, `"zh‑Hans‑CN"`).  The `Locale` is a small, copyable type (annotated `@copy`).

- **Constructors:**
```
impl Locale {
    pub fn new(language: &str) -> Locale;
    pub fn with_region(language: &str, region: &str) -> Locale;
    pub fn from_tag(tag: &str) -> Result<Locale, Error>;
    pub fn default_locale() -> Locale;    // system default
    pub fn to_tag(&self) -> Text;
}
```

- `new(language)` – creates a locale with only a language subtag (e.g., `"en"`).
- `with_region(language, region)` – adds a region subtag.
- `from_tag` parses a full BCP 47 string; returns an error if malformed.
- `default_locale()` returns the system’s current locale (may involve an I/O operation to detect).
- `to_tag` serialises the locale back to a BCP 47 string.

---

## 2. Message Formatting

### 2.1 `Message`

```
pub struct Message {
    locale: Locale,
    text: Text,
}
```

- An opaque compiled message format that has been pre‑processed for the given locale.

- **Construction:**
```
impl Message {
    pub fn new(locale: &Locale, pattern: &str) -> Result<Message, Error>;
    pub fn format(&self, args: &[impl Display]) -> Text;
}
```

- `new` compiles a message pattern (using the ICU MessageFormat syntax) for the given locale.  Patterns support plural and select forms, placeholders, etc.
- `format` produces the formatted string by substituting the provided arguments into the pattern, applying locale‑specific rules (e.g., plural rules, number formatting).  Arguments are passed as a tuple of values implementing `Display` (or `dyn Display`); the engine uses the order of appearance of placeholders.

### 2.2 Pattern Syntax (subset)

The pattern syntax is a subset of ICU MessageFormat:

- Simple replacement: `{0}` references the first argument.
- Formatted replacement: `{0,number,::percent}` formats argument 0 as a percentage using locale rules.
- Plural selection: `{0,plural, =1{one item} other{# items}}` selects the appropriate string based on the value of argument 0.
- Select: `{0,select, male{he} female{she} other{they}}` selects based on enumerated key.

The full syntax is described in a separate ICU MessageFormat specification; our implementation supports these core features.

---

## 3. Number and Date Formatting

### 3.1 `NumberFormatter`

```
pub struct NumberFormatter {
    locale: Locale,
    style: NumberStyle,
}
```

- Formats numbers according to locale‑specific rules (grouping separator, decimal separator, currency symbol, etc.).

- **Constructors and Methods:**
```
impl NumberFormatter {
    pub fn new(locale: &Locale) -> Self;
    pub fn with_style(locale: &Locale, style: NumberStyle) -> Self;
    pub fn format_i64(&self, value: i64) -> Text;
    pub fn format_f64(&self, value: f64) -> Text;
}
```

- **`NumberStyle` Enum:**
```
pub enum NumberStyle {
    Decimal,
    Percent,
    Currency(Text),   // currency code, e.g., "USD"
    Scientific,
}
```

### 3.2 `DateFormatter`

```
pub struct DateFormatter {
    locale: Locale,
    date_style: DateStyle,
    time_style: TimeStyle,
}
```

- Formats dates and times.

- **Constructors and Methods:**
```
impl DateFormatter {
    pub fn new(locale: &Locale) -> Self;
    pub fn with_styles(locale: &Locale, date_style: DateStyle, time_style: TimeStyle) -> Self;
    pub fn format(&self, timestamp: i64) -> Text;   // timestamp in seconds since Unix epoch
    pub fn format_instant(&self, instant: std::time::Instant) -> Text;  // relative?
}
```

- **`DateStyle` and `TimeStyle` enums:**
```
pub enum DateStyle { Full, Long, Medium, Short, None }
pub enum TimeStyle { Full, Long, Medium, Short, None }
```

---

## 4. Plural Rules

### 4.1 `PluralRule`

```
pub struct PluralRule {
    locale: Locale,
}
```

- Encapsulates the CLDR plural rules for a given locale.

- **Methods:**
```
impl PluralRule {
    pub fn new(locale: &Locale) -> Self;
    pub fn select(&self, n: f64) -> PluralCategory;
}
```

- `select` returns the plural category for a number.

### 4.2 `PluralCategory`

```
pub enum PluralCategory {
    Zero,
    One,
    Two,
    Few,
    Many,
    Other,
}
```

---

## 5. Resource Bundles

To avoid embedding large locale data in every binary, the module supports loading compiled resource bundles from external files or included via a comptime macro `@bundle`.

### 5.1 `Bundle`

```
pub struct Bundle {
    locale: Locale,
    entries: Map<Text, Text>,
}
```

- Represents a set of locale‑specific string resources.

- **Methods:**
```
impl Bundle {
    pub fn load_from_file(path: &str) -> Result<Bundle, Error>;
    pub fn get(&self, key: &str) -> Option<&str>;
}
```

- `load_from_file` reads a JSON‑based resource bundle file.  Carries the `io` effect.

### 5.2 `@bundle` Attribute (Optional)

The `@bundle("path/to/bundle.json")` attribute on a module or struct causes the compiler to embed the resource bundle as compile‑time data, zero‑cost.  This is provided by the toolchain, not by the standard library directly, but `std::l10n` documents it.

---

## 6. Implementation Notes

- Locale data (number formats, date formats, plural rules) is notoriously large.  The implementation will not ship with a full CLDR database; instead, it will provide a minimal built‑in set (the system’s default locale data) and allow loading additional data via the `Bundle` mechanism.
- The `Message::format` engine uses a simple recursive descent parser for patterns and resolves plural/select rules via the `PluralRule` module.
- Number and date formatting delegate to OS locale APIs when available (e.g., `snprintf` with locale‑aware formatting on Unix, `GetNumberFormatEx` on Windows) to ensure correctness without large tables.

---

## 7. Testing

- **Locale construction and parsing:** Create a locale from tag strings and verify the components.
- **Message formatting:** Create a message with plural forms, format with various arguments, and compare to expected output.
- **Number formatting:** Format integers and floats with different styles; check grouping separators and decimal separators for a known locale (e.g., `"en‑US"`).
- **Date formatting:** Format a known timestamp and compare to expected string (may need to mock or use a consistent timezone).
- **Plural rules:** Test `PluralRule::select` with known numbers for English (should have `One` for 1, `Other` for 0,2,3,…).
- **Bundle loading:** Write a test JSON resource file, load it, and verify retrieval of a key.

All tests must be guarded by a feature flag (`@cfg(feature = "l10n-enabled")`) and may be skipped on minimal builds.
