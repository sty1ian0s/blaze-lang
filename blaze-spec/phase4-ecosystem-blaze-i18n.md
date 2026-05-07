# Phase 4 – Ecosystem Crate: `blaze‑i18n`

> **Goal:** Provide a high‑level, data‑oriented internationalisation (i18n) framework built on top of `std::l10n`.  It adds compile‑time message extraction, plural‑aware translation catalogs, locale fallback chains, and easy integration with `blaze‑gui` and `blaze‑cli`.  All catalog loading carries the `io` effect; message formatting is pure.

---

## 1. Core Concepts

- **`Catalog`** – a compiled translation catalog for a single locale, loaded from `.po` / `.mo` files or embedded via `@bundle`.
- **`Translator`** – a request‑scoped object that resolves locale from preferences and formats messages.
- **`MessageId`** – a stable identifier for translatable strings (derived from source string and optional context).
- **`LocaleChain`** – a list of locales to try when a message is not found in the preferred locale.

---

## 2. `Catalog`

```
pub struct Catalog {
    locale: Locale,
    entries: Map<Text, Text>,     // msgid → msgstr
    plurals: Map<Text, PluralForm>,
}

impl Catalog {
    pub fn load(path: &str) -> Result<Catalog, I18nError>;
    pub fn from_po(po: &str) -> Catalog;
    pub fn get(&self, msgid: &str) -> Option<&str>;
    pub fn get_plural(&self, msgid: &str, count: u64) -> Option<&str>;
}
```

- `load` reads compiled message catalogs (`.mo` files) generated from PO sources.
- `from_po` parses a PO file at compile‑time or runtime.
- Plurals are stored as `PluralForm` which contains the rules for each plural category.

---

## 3. `Translator`

```
pub struct Translator {
    locale_chain: LocaleChain,
    catalogs: Map<Locale, Catalog>,
}

impl Translator {
    pub fn new(default_locale: &Locale) -> Translator;
    pub fn add_catalog(&mut self, locale: Locale, catalog: Catalog);
    pub fn translate(&self, msgid: &str) -> Text;
    pub fn translate_plural(&self, msgid: &str, msgid_plural: &str, count: u64) -> Text;
}
```

- `translator.translate("Hello")` looks up the message in the locale chain and returns the translation, falling back to the original string.
- `translate_plural` uses the locale’s plural rule to select the correct form.

---

## 4. Message Extraction

The crate provides a `#[i18n]` macro and a `blaze‑i18n‑extract` tool that scans source files for translatable strings and produces a `.pot` template.  The macro marks strings at compile time:

```
#[i18n]
fn greet(name: &str) -> Text {
    i18n!("Hello, {}", name)
}
```

The tool extracts the string and context, emits `.pot`, and later the developer can merge with `.po` files.

---

## 5. Integration with `blaze‑gui` and `blaze‑cli`

- The `Translator` can be stored in an actor’s state; widgets can call `translator.translate(...)` directly because it is pure after loading.
- `blaze‑cli` commands can accept a `--lang` argument that sets the locale.

---

## 6. Error Handling

```
pub enum I18nError {
    Io(std::io::Error),
    InvalidCatalog,
    MissingTranslation,
}
```

---

## 7. Testing

- **Load catalog:** Load a small PO file, translate known strings, verify.
- **Plurals:** Test plural forms for English and a language with more categories (e.g., Russian).
- **Fallback:** Set a chain of locales, verify that an unrecognised locale falls back to the next.

All tests must pass on all platforms.
