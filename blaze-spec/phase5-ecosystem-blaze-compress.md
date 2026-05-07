# Phase 5 – Ecosystem Crate: `blaze‑compress`

> **Goal:** Specify the `blaze‑compress` crate, which provides a unified, data‑oriented interface for compression and decompression algorithms in Blaze.  It supports common lossless compression formats (Deflate, Gzip, Zlib, Brotli, LZ4, Zstandard) and lightweight framing for archival purposes.  All operations are pure (unless performing I/O) and operate on byte slices, making zero‑copy processing possible for non‑streaming cases.  The crate is built on the linear memory model, with all internal state managed via `Dispose`.

---

## 1. Core Traits

### 1.1 `Compressor`

```
pub trait Compressor {
    fn compress(&mut self, input: &[u8], output: &mut Vec<u8>) -> Result<(), CompressError>;
    fn finish(&mut self, output: &mut Vec<u8>) -> Result<(), CompressError>;
}
```

- `compress` feeds a chunk of uncompressed data into the compressor.  The output may be partial; call `finish` to flush any remaining data.
- The compressor may maintain internal state (dictionary, window) and must be disposed when done.

### 1.2 `Decompressor`

```
pub trait Decompressor {
    fn decompress(&mut self, input: &[u8], output: &mut Vec<u8>) -> Result<(), CompressError>;
    fn is_finished(&self) -> bool;
}
```

- `decompress` processes compressed input and appends decompressed bytes to the output buffer.  It may buffer input internally.
- `is_finished` returns `true` when the end of the compressed stream has been reached and all output has been emitted.

---

## 2. Supported Algorithms

Each algorithm is implemented as a struct that implements `Compressor` and/or `Decompressor`.

### 2.1 Deflate / Zlib / Gzip

```
pub struct DeflateEncoder { … }
impl Compressor for DeflateEncoder { … }
pub struct DeflateDecoder { … }
impl Decompressor for DeflateDecoder { … }

pub struct GzipEncoder { … }
impl Compressor for GzipEncoder { … }
pub struct GzipDecoder { … }
impl Decompressor for GzipDecoder { … }

pub struct ZlibEncoder { … }
impl Compressor for ZlibEncoder { … }
pub struct ZlibDecoder { … }
impl Decompressor for ZlibDecoder { … }
```

- Gzip adds a header and footer; Deflate is the raw stream; Zlib adds a small header and Adler‑32 checksum.

### 2.2 Brotli

```
pub struct BrotliEncoder { level: u32, … }
impl Compressor for BrotliEncoder { … }
pub struct BrotliDecoder { … }
impl Decompressor for BrotliDecoder { … }
```

- Level 0‑11, default 6. High compression ratio, slower than Deflate.

### 2.3 LZ4

```
pub struct Lz4Encoder { acceleration: u32, … }
impl Compressor for Lz4Encoder { … }
pub struct Lz4Decoder { … }
impl Decompressor for Lz4Decoder { … }
```

- Extremely fast, moderate compression. Suitable for real‑time data.

### 2.4 Zstandard (Zstd)

```
pub struct ZstdEncoder { level: i32, … }
impl Compressor for ZstdEncoder { … }
pub struct ZstdDecoder { … }
impl Decompressor for ZstdDecoder { … }
```

- Level 1‑22, high compression ratio with excellent speed. Supports dictionary training (optional feature).

---

## 3. Convenience Functions

For simple cases where the entire data fits in memory, the crate provides one‑shot functions that allocate and return the result.

```
pub fn compress_deflate(data: &[u8]) -> Result<Vec<u8>, CompressError>;
pub fn decompress_deflate(data: &[u8]) -> Result<Vec<u8>, CompressError>;
pub fn compress_gzip(data: &[u8]) -> Result<Vec<u8>, CompressError>;
pub fn decompress_gzip(data: &[u8]) -> Result<Vec<u8>, CompressError>;
pub fn compress_brotli(data: &[u8], level: u32) -> Result<Vec<u8>, CompressError>;
pub fn decompress_brotli(data: &[u8]) -> Result<Vec<u8>, CompressError>;
pub fn compress_lz4(data: &[u8]) -> Result<Vec<u8>, CompressError>;
pub fn decompress_lz4(data: &[u8]) -> Result<Vec<u8>, CompressError>;
pub fn compress_zstd(data: &[u8], level: i32) -> Result<Vec<u8>, CompressError>;
pub fn decompress_zstd(data: &[u8]) -> Result<Vec<u8>, CompressError>;
```

- These are implemented on top of the `Compressor`/`Decompressor` traits, creating temporary state objects and calling `compress`/`finish`.

---

## 4. Streaming Compression

For large data, the crate provides adaptors that implement `std::io::Write` (for compression) and `std::io::Read` (for decompression), allowing integration with Blaze’s I/O system.

### 4.1 `CompressWriter<W: Write>`

```
pub struct CompressWriter<W: Write> {
    writer: W,
    encoder: Box<dyn Compressor>,
    buf: Vec<u8>,
}
impl<W: Write> Write for CompressWriter<W> { … }
```

- All bytes written to `CompressWriter` are compressed on the fly and passed to the inner writer.  On drop (or explicit `finish`), any remaining data is flushed.

### 4.2 `DecompressReader<R: Read>`

```
pub struct DecompressReader<R: Read> {
    reader: R,
    decoder: Box<dyn Decompressor>,
    buf: Vec<u8>,
    out_pos: usize,
}
impl<R: Read> Read for DecompressReader<R> { … }
```

- Reads from the inner reader, decompresses, and returns decompressed bytes.

---

## 5. Archive Support (Optional, feature `tar`)

The crate can bundle/unbundle files using a simple tar‑like format:

```
pub fn archive_files(paths: &[&str], output: &str, compression: Compression) -> Result<(), CompressError>;
pub fn extract_archive(input: &str, output_dir: &str) -> Result<(), CompressError>;
```

- `Compression` enum: `None`, `Gzip`, `Brotli`, `Zstd`.  The archive is a tar file compressed with the chosen algorithm.

---

## 6. Error Handling

```
pub enum CompressError {
    Io(std::io::Error),
    InvalidInput(Text),
    OutOfBounds,
    InternalError(Text),
    CompressionFailed(Text),
    DecompressionFailed(Text),
    UnsupportedFeature(Text),
}
```

- Errors from the underlying C libraries (if any) are mapped to these variants.

---

## 7. Implementation Notes

- The crate uses native Blaze implementations for Deflate, LZ4, and Zstd (pure Blaze).  For Brotli and Zstd, optional C bindings can be enabled via features `brotli-c` and `zstd-c` for maximum performance, but the default is a pure‑Blaze port that is still competitive.
- The `Compressor` and `Decompressor` trait objects are used only in the streaming adaptors; all one‑shot functions are monomorphised and dispatch statically.
- Memory management for internal buffers uses `Vec<u8>` which are automatically disposed.  No manual allocation occurs in the public API.
- The crate is data‑oriented: all compression state is stored in plain structs, no virtual calls inside the hot loop.

---

## 8. Testing

- **Round‑trip for each algorithm:** Generate random bytes, compress, decompress, verify equality.
- **Empty input:** Compress an empty slice, decompress, verify empty.
- **Streaming write/read:** Create a `CompressWriter<Vec<u8>>`, write chunks, finish, then create a `DecompressReader`s over the result, read back, compare.
- **Error handling:** Provide a corrupted compressed stream (e.g., truncated Gzip), expect `DecompressionFailed`.
- **Archive:** Create an archive with two files, extract, verify files and contents.
- **Performance:** Benchmark compression and decompression speeds for each algorithm against reference implementations (optional).

All tests must pass on all supported platforms.
