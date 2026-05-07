# Phase 5 – Ecosystem Crate: `blaze‑svg`

> **Goal:** Specify the `blaze‑svg` crate, which provides a data‑oriented, pure‑Blaze library for parsing, generating, and manipulating SVG (Scalable Vector Graphics) documents.  It supports the full SVG 1.1 static specification and a subset of SVG 2.0, including paths, shapes, text, gradients, filters, and transform hierarchies.  All parsing and rendering logic is pure; writing to files or GUI surfaces carries the `io`/`gpu` effect via the appropriate backend.  The crate is designed for use in GUI toolkits, data visualisation (`blaze‑plot`), and document generation.

---

## 1. Core Types

### 1.1 `SvgDocument`

```
pub struct SvgDocument {
    pub view_box: Rect,
    pub width: f32,
    pub height: f32,
    pub elements: Vec<SvgElement>,
    pub defs: Map<Text, SvgElement>,
    pub css: Option<Text>,
    pub metadata: Option<Text>,
}
```

- The root of an SVG image.  Linear (move semantics).  `Dispose` drops all elements.
- `Rect` is `{ x, y, width, height }`.

### 1.2 `SvgElement`

```
pub enum SvgElement {
    Group(Group),
    Path(Path),
    Rect(RectElement),
    Circle(Circle),
    Ellipse(Ellipse),
    Line(Line),
    Polyline(Polyline),
    Polygon(Polygon),
    Text(TextElement),
    Image(SvgImage),
    Use(Use),
    Defs(Vec<SvgElement>),
    LinearGradient(Gradient),
    RadialGradient(Gradient),
    Filter(Filter),
    ClipPath(Vec<SvgElement>),
    Mask(Vec<SvgElement>),
    // … more as needed
}
```

- Each element struct contains common attributes (`id`, `transform`, `style`, `class`) and element‑specific geometry.

### 1.3 Common Attributes

All elements have:

```
pub struct SvgAttributes {
    pub id: Option<Text>,
    pub transform: Option<Transform>,
    pub style: Option<StyleMap>,
    pub class: Vec<Text>,
}
```

- `Transform` is an enum of matrix, translate, scale, rotate, skewX, skewY.
- `StyleMap` is a map of CSS property names to values (e.g., `"fill" -> "red"`).

---

## 2. Parsing SVG

### 2.1 `from_str`

```
pub fn from_str(svg: &str) -> Result<SvgDocument, SvgError>;
```

- Parses an SVG XML string into an `SvgDocument`.  The parser is a recursive‑descent XML parser tuned for SVG, supporting entities, CDATA, and namespaces.
- Unknown elements are retained as a generic `ForeignElement` node, but not fully validated.
- CSS is collected into a single string for later processing (but not fully applied – the crate does not contain a CSS engine; it’s up to the user to resolve styles or use a separate `blaze‑css` crate).

### 2.2 `from_bytes`

```
pub fn from_bytes(bytes: &[u8]) -> Result<SvgDocument, SvgError>;
```

- Same as above, but takes a UTF‑8 byte slice.

---

## 3. Writing SVG

### 3.1 `to_string`

```
pub fn to_string(doc: &SvgDocument) -> Text;
```

- Serializes the document to a compact SVG XML string.

### 3.2 `to_string_pretty`

```
pub fn to_string_pretty(doc: &SvgDocument) -> Text;
```

- Pretty‑prints with indentation and line breaks.

### 3.3 `to_writer`

```
pub fn to_writer<W: Write>(writer: &mut W, doc: &SvgDocument) -> Result<(), std::io::Error>;
pub fn to_writer_pretty<W: Write>(writer: &mut W, doc: &SvgDocument) -> Result<(), std::io::Error>;
```

- Writes directly to a `Write` stream.

---

## 4. Path Data

A `Path` element contains a `PathData` which is a sequence of path commands.

### 4.1 `PathCommand`

```
pub enum PathCommand {
    MoveTo { abs: bool, x: f32, y: f32 },
    LineTo { abs: bool, x: f32, y: f32 },
    HorizontalLineTo { abs: bool, x: f32 },
    VerticalLineTo { abs: bool, y: f32 },
    CurveTo { abs: bool, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32 },
    SmoothCurveTo { abs: bool, x2: f32, y2: f32, x: f32, y: f32 },
    QuadraticBezierTo { abs: bool, x1: f32, y1: f32, x: f32, y: f32 },
    SmoothQuadraticBezierTo { abs: bool, x: f32, y: f32 },
    EllipticalArc { abs: bool, rx: f32, ry: f32, x_axis_rotation: f32, large_arc: bool, sweep: bool, x: f32, y: f32 },
    ClosePath,
}
```

- The crate provides a parser for the SVG path data format (`d` attribute).
- `PathData` is `Vec<PathCommand>`.

---

## 5. Building SVG Programmatically

A convenient builder API allows constructing SVG documents without writing XML.

### 5.1 `SvgBuilder`

```
pub struct SvgBuilder {
    doc: SvgDocument,
}
impl SvgBuilder {
    pub fn new(width: f32, height: f32) -> Self;
    pub fn view_box(mut self, vb: Rect) -> Self;
    pub fn group(mut self, g: Group) -> Self;
    pub fn rect(mut self, r: RectElement) -> Self;
    pub fn circle(mut self, c: Circle) -> Self;
    pub fn text(mut self, t: TextElement) -> Self;
    // … etc.
    pub fn build(self) -> SvgDocument;
}
```

- Each element can be created with a `new` function and setter methods (e.g., `Circle::new(cx, cy, r).fill("red")`).

---

## 6. Rendering (Optional, feature `render`)

When the `render` feature is enabled, the crate can rasterize an `SvgDocument` to an `Image` (from `blaze‑image`) using a built‑in software rasterizer.  The rasterizer handles:

- Shapes (rect, circle, ellipse, line, polyline, polygon)
- Paths (via a path tessellation algorithm)
- Text (using a simple bitmap font or an optional font backend via `blaze‑font`)
- Basic gradients (linear, radial) and flat colours
- Alpha blending and opacity
- Clipping

The renderer is pure and can be used in tests or for offline image generation.

```
pub fn render_to_image(doc: &SvgDocument, scale: f32) -> Result<Image, SvgError>;
pub fn render_to_file(doc: &SvgDocument, path: &str, scale: f32, format: ImageFormat) -> Result<(), SvgError>;
```

- `scale` determines the pixel density (e.g., 2.0 for retina).

---

## 7. Error Handling

```
pub enum SvgError {
    Parse(Text),
    InvalidAttribute(Text),
    InvalidPathData(Text),
    RenderError(Text),
    Io(std::io::Error),
}
```

---

## 8. Testing

- **Parse and write:** Parse an SVG file, write it back, re‑parse, and verify structural equality.
- **Path data:** Parse complex path data, verify the correct sequence of commands.
- **Builder:** Construct a document using the builder API, serialize, and verify the XML output matches expected.
- **Rendering (feature):** Render a simple SVG to an image, check a few pixel colours.
