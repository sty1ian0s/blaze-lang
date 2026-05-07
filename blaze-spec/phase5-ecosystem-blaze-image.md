# Phase 5 – Ecosystem Crate: `blaze‑image`

> **Goal:** Specify the `blaze‑image` crate, which provides a data‑oriented, zero‑copy image processing library built on Blaze’s `blaze‑tensor` (or simple SoA arrays) and `blaze‑serde`.  It supports common pixel formats, colour space conversions, geometric transformations, filtering, and integration with GUI and file I/O.  All processing functions are pure and automatically parallelised, making the crate suitable for real‑time graphics, computer vision, and scientific imaging.

---

## 1. Core Types

### 1.1 `Image`

```
pub struct Image {
    width: u32,
    height: u32,
    format: PixelFormat,
    color_space: ColorSpace,
    data: Vec<u8>,          // packed pixels, layout depends on format
}
```

- Linear; `Dispose` frees the pixel data.  `Image` can be cloned (deep copy) via explicit `.clone()`.
- `PixelFormat` is an enum specifying the number of channels and their bit depth: `Gray8`, `Gray16`, `RGB8`, `RGBA8`, `BGRA8`, `RGB16`, `RGBA16`, `RGBA32F`, `BayerRG8`, etc.
- `ColorSpace`: `sRGB`, `Linear`, `CIELab`, `HSV`, `YCbCr`, etc.

### 1.2 Constructors

```
impl Image {
    pub fn new(width: u32, height: u32, format: PixelFormat, color_space: ColorSpace) -> Image;
    pub fn from_bytes(width: u32, height: u32, format: PixelFormat, color_space: ColorSpace, data: Vec<u8>) -> Image;
    pub fn from_raw(width: u32, height: u32, format: PixelFormat, color_space: ColorSpace, data: Vec<u8>) -> Result<Image, ImageError>;
    pub fn from_file(path: &str) -> Result<Image, ImageError>;   // auto‑detect format
    pub fn from_memory(buf: &[u8], format: ImageFormat) -> Result<Image, ImageError>;
}
```

- `from_raw` validates that the data length matches the expected size for the given dimensions and pixel format.

### 1.3 Properties

```
impl Image {
    pub fn width(&self) -> u32;
    pub fn height(&self) -> u32;
    pub fn format(&self) -> PixelFormat;
    pub fn color_space(&self) -> ColorSpace;
    pub fn as_bytes(&self) -> &[u8];
    pub fn as_mut_bytes(&mut self) -> &mut [u8];
    pub fn pixel_count(&self) -> usize;
}
```

---

## 2. Pixel Access

The crate provides safe, checked pixel access via coordinates and a `Pixel` type that is an enum over all possible channel combinations.

### 2.1 `Pixel` Enum

```
pub enum Pixel {
    Gray8(u8),
    Gray16(u16),
    RGB8(u8, u8, u8),
    RGBA8(u8, u8, u8, u8),
    RGBA16(u16, u16, u16, u16),
    RGBA32F(f32, f32, f32, f32),
    // … other formats
}
```

### 2.2 Methods

```
impl Image {
    pub fn get_pixel(&self, x: u32, y: u32) -> Option<Pixel>;
    pub fn set_pixel(&mut self, x: u32, y: u32, pixel: &Pixel) -> Result<(), ImageError>;
    pub unsafe fn get_pixel_unchecked(&self, x: u32, y: u32) -> Pixel;
    pub unsafe fn set_pixel_unchecked(&mut self, x: u32, y: u32, pixel: &Pixel);
}
```

- The unchecked versions assume coordinates are within bounds; they are used internally by iterators after bounds checks are elided by the compiler.

---

## 3. Iterators and Parallel Processing

The crate provides chunked iterators for processing scanlines, tiles, or SoA‑viewed pixels.

```
impl Image {
    pub fn rows(&self) -> RowIterator;
    pub fn rows_mut(&mut self) -> RowIteratorMut;
    pub fn par_rows(&self) -> impl ParallelIterator<Item = &[u8]>;   // using Blaze's auto‑parallel for loops
    pub fn par_rows_mut(&mut self) -> impl ParallelIterator<Item = &mut [u8]>;
}
```

- Iterators return slices representing whole rows or tiles, enabling vectorised operations.

---

## 4. Colour Space Conversion

Colour conversion functions take `&Image` and return a new `Image` in the target colour space.  They are pure and automatically parallelised.

```
pub fn convert_color(image: &Image, target_space: ColorSpace) -> Image;
pub fn to_grayscale(image: &Image) -> Image;
pub fn to_rgba(image: &Image) -> Image;
pub fn premultiply_alpha(image: &Image) -> Image;
pub fn unpremultiply_alpha(image: &Image) -> Image;
```

- These use lookup tables and hardware‑accelerated SIMD where possible (via `blaze‑simd`).  Floating‑point conversions use precise formulas.

---

## 5. Geometric Transformations

### 5.1 Resize

```
pub fn resize(image: &Image, new_width: u32, new_height: u32, filter: Filter) -> Image;
```

- `Filter` enum: `Nearest`, `Linear`, `Cubic`, `Lanczos3`.  The implementation uses separable convolution for performance and correctly handles edge clamping.

### 5.2 Rotate, Flip, Crop

```
pub fn rotate(image: &Image, angle: f32, filter: Filter) -> Image;
pub fn flip_horizontal(image: &Image) -> Image;
pub fn flip_vertical(image: &Image) -> Image;
pub fn crop(image: &Image, x: u32, y: u32, width: u32, height: u32) -> Image;
```

---

## 6. Filters and Effects

A set of common image processing filters, all pure and parallelised.

```
pub fn blur(image: &Image, radius: f32) -> Image;
pub fn sharpen(image: &Image, amount: f32) -> Image;
pub fn median(image: &Image, radius: u32) -> Image;
pub fn sobel(image: &Image) -> Image;          // edge detection
pub fn threshold(image: &Image, value: u8) -> Image;
pub fn brightness(image: &Image, amount: f32) -> Image;
pub fn contrast(image: &Image, amount: f32) -> Image;
pub fn gamma_correction(image: &Image, gamma: f32) -> Image;
pub fn histogram_equalization(image: &Image) -> Image;
```

- For large radii, the blur uses a separable box‑blur approximation.  Median uses a fast histogram‑based approach.

---

## 7. Drawing Primitives (Optional, feature `drawing`)

For simple annotations, the crate provides drawing functions that mutate an `Image` in place.

```
pub fn draw_line(image: &mut Image, x0: u32, y0: u32, x1: u32, y1: u32, color: Pixel);
pub fn draw_rect(image: &mut Image, x: u32, y: u32, w: u32, h: u32, color: Pixel);
pub fn draw_circle(image: &mut Image, cx: u32, cy: u32, radius: u32, color: Pixel);
pub fn draw_text(image: &mut Image, text: &str, x: u32, y: u32, font: &Font, color: Pixel);
```

---

## 8. Image I/O

The crate supports reading and writing common image formats.  Each format is feature‑gated to avoid unnecessary dependencies.

```
pub enum ImageFormat { Png, Jpeg, Tiff, Bmp, Tga, Webp, Hdr, Exr }

pub fn load(path: &str) -> Result<Image, ImageError>;
pub fn save(path: &str, image: &Image, format: ImageFormat) -> Result<(), ImageError>;
pub fn encode(image: &Image, format: ImageFormat) -> Result<Vec<u8>, ImageError>;
pub fn decode(buf: &[u8], format: ImageFormat) -> Result<Image, ImageError>;
```

- PNG and JPEG are built‑in using `blaze‑compress` (zlib, deflate).  Others rely on external `blaze‑tiff`, `blaze‑webp` crates.  The crate provides a registry for external format plugins.

---

## 9. Error Handling

```
pub enum ImageError {
    Io(std::io::Error),
    InvalidFormat(Text),
    UnsupportedFormat(Text),
    InvalidDimensions(Text),
    PixelOutOfBounds,
    ConversionError(Text),
    FilterError(Text),
}
```

---

## 10. Testing

- **Construction and I/O:** Create an image, save to PNG, load back, compare pixels.
- **Colour conversion:** Convert an RGB image to grayscale and back, verify expected values.
- **Resize:** Scale an image by 2x nearest‑neighbour, verify pixel duplication.
- **Filters:** Apply a 3x3 box blur to a test pattern, compare with expected result.
- **Parallel processing:** Use `par_rows` to brighten an image; verify all pixels changed equally.
- **Error handling:** Attempt to load a non‑existent file, expect `Io` error.
- **Memory safety:** Ensure that out‑of‑bounds pixel access via `set_pixel` returns an error, and that `unsafe` unchecked access works correctly within bounds.

All tests must pass on all platforms.
