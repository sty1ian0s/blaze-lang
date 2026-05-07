# Phase 4 – Ecosystem Crate: `blaze‑text`

> **Goal:** Provide a pure, data‑oriented, GPU‑accelerated text layout and rendering engine for the Blaze GUI ecosystem.  The crate loads system fonts or custom font files, caches glyph information in SoA‑optimised atlases, performs bidirectional text shaping (using HarfBuzz or a pure‑Blaze alternative), and exposes a simple API to measure and draw text onto any `blaze‑wgpu` render pass.  It is used by `blaze‑gui`, `blaze‑plot`, `blaze‑svg`, and any application that needs to display or measure text.

---

## 1. Core Concepts

- **Font** – a single typeface loaded from a file or system API, stored as an indexed collection of outlines.
- **FontAtlas** – a GPU texture containing pre‑rasterised glyphs, shared by all text using that font and size.
- **TextLayout** – a positioned sequence of glyphs resulting from shaping a string with a given font, size, and text direction.
- **Pure measurement** – `measure_text` returns width and height without any GPU or I/O calls (after the font is loaded).
- **GPU rendering** – `draw_text` takes a `TextLayout` and writes vertices into a `wgpu` command encoder.
- **Deterministic** – identical input produces identical positions and glyph IDs, enabling reproducible UI tests.

---

## 2. `Font` and `FontFace`

### 2.1 `Font`

```
pub struct Font {
    family: String,
    weight: FontWeight,
    style: FontStyle,
    data: Vec<u8>,               // raw font file bytes (TTF/OTF)
    units_per_em: u32,
    ascent: f64,
    descent: f64,
    line_gap: f64,
    glyph_count: u32,
    // internal tables (cmap, hmtx, …)
}
```

- Linear type; `Dispose` simply drops the data.  `Font` is not `@copy` because it owns heap data.
- `FontWeight` and `FontStyle` are enums: `Normal`, `Bold`, `Italic`, `BoldItalic`, etc.

### 2.2 `FontLoader`

```
pub fn load_system_font(family: &str, weight: FontWeight, style: FontStyle) -> Result<Font, TextError>;
pub fn load_from_file(path: &str) -> Result<Font, TextError>;
pub fn load_from_bytes(bytes: &[u8]) -> Result<Font, TextError>;
```

- `load_system_font` uses OS APIs (`CTFont` on macOS, `DirectWrite` on Windows, `fontconfig` on Linux).
- `load_from_file` and `load_from_bytes` parse the raw font tables.

---

## 3. `FontAtlas`

### 3.1 Struct

```
pub struct FontAtlas {
    texture: wgpu::Texture,
    entries: Map<u32, GlyphEntry>,     // glyph id → atlas rect + metrics
    glyphs: Vec<GlyphCacheInfo>,        // SoA for glyph metadata
    next_x: u32,
    next_y: u32,
    max_row_height: u32,
}
```

- Created by `FontAtlas::new(font: &Font, size: f64, gpu_device: &GpuDevice) -> FontAtlas`.
- As glyphs are requested, they are rasterised on the CPU and uploaded to the texture.
- The atlas grows dynamically (or uses multiple pages) if it runs out of space.

### 3.2 Glyph Entry

```
struct GlyphEntry {
    rect: (u32, u32, u32, u32),   // x, y, width, height in atlas texture
    bearing_x: f64,
    bearing_y: f64,
    advance_width: f64,
    advance_height: f64,
}
```

- `GlyphCacheInfo` stores lifetime and usage counters for eviction (optional).

---

## 4. Text Layout and Shaping

### 4.1 `TextLayout`

```
pub struct TextLayout {
    glyphs: Vec<PositionedGlyph>,
    total_width: f64,
    total_height: f64,
}
pub struct PositionedGlyph {
    pub glyph_id: u32,
    pub x: f64,
    pub y: f64,
    pub advance_width: f64,
}
```

- Created by `layout_text(font: &Font, atlas: &mut FontAtlas, text: &str, font_size: f64, direction: TextDirection) -> TextLayout`.
- Shaping is performed using `harfbuzz` C bindings (if feature `harfbuzz` is enabled) or a pure‑Blaze simple shaper (for Latin/Cyrillic only).
- `TextDirection` is `LTR`, `RTL`, or `Auto` (detected from the first strong directional character).
- The shaper applies kerning, ligatures, and combining marks as supported.

### 4.2 Measurement Only

```
pub fn measure_text(font: &Font, text: &str, font_size: f64) -> (f64, f64);
```

- Returns the width and height of the text without allocating an atlas or layout.  Uses the font’s metrics and a fast approximate width computation (sum of advance widths for the shaped glyphs, no atlas needed).

---

## 5. Rendering

### 5.1 `draw_text`

```
pub fn draw_text(render_pass: &mut wgpu::RenderPass, layout: &TextLayout, atlas: &FontAtlas, position: (f64, f64), color: Color);
```

- For each glyph in `layout`, looks up its atlas rect, generates a quad (two triangles) with texture coordinates, and writes it to the current render pass.
- The renderer expects a pre‑bound pipeline and bind group (provided by `blaze‑gui` or the application).

### 5.2 `prepare_text_pipeline`

```
pub fn prepare_text_pipeline(device: &wgpu::Device, config: &wgpu::SurfaceConfiguration) -> wgpu::RenderPipeline;
```

- Creates the necessary GPU pipeline for rendering glyph quads from a texture atlas.

---

## 6. Error Handling

```
pub enum TextError {
    Io(std::io::Error),
    InvalidFont(Text),
    FontNotFound,
    GlyphNotFound,
    AtlasFull,
}
```

- `GlyphNotFound` is returned if a glyph is missing from the font; the shaping engine may substitute a `.notdef` glyph.

---

## 7. Implementation Notes

- The crate uses `blaze‑wgpu` for GPU texture management.  It does not depend on `blaze‑gui` directly; any renderer can use it.
- The font atlas uses a simple grid packing algorithm (shelf packing) for glyph placement.  It may evict unused glyphs based on an LRU policy to keep the atlas from growing indefinitely.
- For `no_std` or embedded environments, a software‑rendering backend can be provided that bypasses the GPU and uses a CPU bitmap.
- Shaping is delegated to HarfBuzz by default, but a pure‑Blaze fallback (using the same Unicode tables) is available when the `harfbuzz` feature is disabled.  The fallback supports only Latin‑1 scripts, but it’s sufficient for many games and embedded systems.

---

## 8. Testing

- **Font loading:** load a system font, verify that `ascent`, `descent`, and glyph count are non‑zero.
- **Measurement:** measure the width of a known string with a monospace font; compare with expected pixel width.
- **Shape and layout:** shape a simple text, verify that glyph positions are in ascending order and no glyphs overlap (using a test font).
- **Atlas allocation:** rasterise a few glyphs, check that they occupy non‑overlapping regions in the atlas texture.
- **Rendering:** render “Hello” to an off‑screen texture, capture the pixel buffer, and verify that the text is readable (by checking that not all pixels are background color).
- **Performance:** measure the time to layout 10,000 words; must be under 1 ms on a modern CPU.

All tests must pass on all platforms with a GPU (software emulation in CI).
