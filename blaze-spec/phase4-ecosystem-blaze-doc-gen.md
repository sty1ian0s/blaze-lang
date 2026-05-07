# Phase 4 – Ecosystem Crate: `blaze‑doc‑gen`

> **Goal:** Provide a pure, data‑oriented documentation generator for Blaze source code.  It parses `///` and `//!` doc comments, extracts Markdown and code examples, and produces static HTML documentation with search, cross‑references, and doctest integration.  The crate is used by `blaze doc` and can be embedded in CI pipelines and IDEs.  All parsing is pure; writing files carries the `io` effect.

---

## 1. Core Types

### 1.1 `DocCrate`

```
pub struct DocCrate {
    pub name: Text,
    pub version: Text,
    pub modules: Vec<DocModule>,
    pub readme: Option<Text>,
}
```

- Represents a parsed crate’s documentation, ready to render.

### 1.2 `DocModule`

```
pub struct DocModule {
    pub name: Text,
    pub doc: Option<Text>,               // //! comments
    pub items: Vec<DocItem>,
    pub submodules: Vec<DocModule>,
}
```

### 1.3 `DocItem`

```
pub enum DocItem {
    Function(DocFunction),
    Struct(DocStruct),
    Enum(DocEnum),
    Trait(DocTrait),
    TypeAlias(DocTypeAlias),
    Constant(DocConstant),
    Static(DocStatic),
    Module(DocModule),
}

pub struct DocFunction {
    pub name: Text,
    pub signature: Text,
    pub doc: Option<Text>,
    pub code_examples: Vec<Text>,
}

pub struct DocStruct {
    pub name: Text,
    pub doc: Option<Text>,
    pub fields: Vec<DocField>,
    pub methods: Vec<DocFunction>,
    pub trait_impls: Vec<Text>,
}

pub struct DocField {
    pub name: Text,
    pub ty: Text,
    pub doc: Option<Text>,
}

pub struct DocEnum {
    pub name: Text,
    pub doc: Option<Text>,
    pub variants: Vec<DocVariant>,
}

pub struct DocVariant {
    pub name: Text,
    pub doc: Option<Text>,
    pub fields: Vec<DocField>,
}

pub struct DocTrait {
    pub name: Text,
    pub doc: Option<Text>,
    pub methods: Vec<DocFunction>,
    pub associated_types: Vec<DocTypeAlias>,
}

pub struct DocTypeAlias {
    pub name: Text,
    pub definition: Text,
    pub doc: Option<Text>,
}

pub struct DocConstant {
    pub name: Text,
    pub ty: Text,
    pub value: Text,
    pub doc: Option<Text>,
}

pub struct DocStatic {
    pub name: Text,
    pub ty: Text,
    pub doc: Option<Text>,
}
```

- All types are plain `@data` structs, owning their strings linearly.  They are constructed by the parser from the compiler’s AST and doc comments.

---

## 2. Parsing

### 2.1 `extract_docs`

```
pub fn extract_docs(crate_path: &str) -> Result<DocCrate, DocGenError>;
```

- Uses the compiler’s `std::meta` reflection to extract all public items and their doc comments.  Does not re‑compile the crate; instead, it reads the pre‑compiled `.blzlib` metadata (which includes doc comments).

### 2.2 Markdown Parsing

- Doc comments are parsed as CommonMark Markdown, with support for fenced code blocks, inline code, lists, and links.
- Code blocks tagged `blaze` (or untagged) are extracted as `code_examples` and are run as doctests by `blaze test`.
- Intra‑crate links (e.g., `[`MyStruct`]`) are resolved to the correct item page.

---

## 3. HTML Generation

### 3.1 `render_html`

```
pub fn render_html(doc: &DocCrate, config: &DocConfig) -> Result<DocOutput, DocGenError>;
```

- Produces a set of static HTML files (one per module/item) and a search index (JavaScript/JSON).

### 3.2 `DocConfig`

```
pub struct DocConfig {
    pub output_dir: Text,
    pub title: Text,
    pub favicon: Option<Text>,
    pub include_source: bool,
    pub theme: DocTheme,
}

pub enum DocTheme { Light, Dark, Auto }
```

- `include_source` embeds the original `.blz` source in the documentation for each item.
- `Auto` theme follows the user’s browser preference.

### 3.3 `DocOutput`

```
pub struct DocOutput {
    pub files: Vec<(Text, Vec<u8>)>,   // (path, content)
    pub search_index: Vec<u8>,
}
```

- `blaze doc` writes these files to the output directory.

---

## 4. Search Index

The crate generates a JSON search index containing every documented item’s name, module, type, and a short snippet.  The HTML pages include a small JavaScript search engine that uses this index for offline, client‑side search.

---

## 5. Integration with LSP

The `blaze lsp` server can use `extract_docs` to provide hover documentation, completions with signatures, and doc‑comment previews.

---

## 6. Error Handling

```
pub enum DocGenError {
    Io(std::io::Error),
    MetadataNotFound,
    ParseError(Text),
}
```

---

## 7. Testing

- **Doc extraction:** Create a small crate with documented items, extract docs, verify structure.
- **Code examples:** Verify that code blocks are correctly extracted and tagged.
- **HTML rendering:** Render to HTML and check for expected strings (function name, struct name, description).
- **Search index:** Generate the index and verify that a known item’s name appears.

All tests must pass on all platforms.
