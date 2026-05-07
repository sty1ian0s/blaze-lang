# Phase 4 – Ecosystem Crate: `blaze‑zstd`

> **Goal:** Provide a standalone, zero‑copy Zstandard (Zstd) compression and decompression library built entirely in Blaze.  It supports the Zstd format, including dictionary compression and streaming compression for large data.  All operations are pure unless dictionary training or file I/O is involved.  The crate is designed for use cases requiring high compression ratios with reasonable speed, without pulling in the full `blaze‑compress` dependency.

---

## 1. Core Types

### 1.1 `ZstdEncoder`

```
pub struct ZstdEncoder {
    level: i32,
    // internal state
}
impl ZstdEncoder {
    pub fn new(level: i32) -> ZstdEncoder;
    pub fn compress(&mut self, input: &[u8], output: &mut Vec<u8>) -> Result<usize, ZstdError>;
    pub fn finish(&mut self, output: &mut Vec<u8>) -> Result<(), ZstdError>;
}
```

- `level` is the compression level (1‑22, default 3).  Higher levels yield better compression at the expense of speed and memory.
- `compress` appends compressed data to `output`, returning the number of compressed bytes written.
- `finish` flushes any remaining data.  The encoder can also operate in streaming mode via `write`/`read` adaptors.

### 1.2 `ZstdDecoder`

```
pub struct ZstdDecoder { … }
impl ZstdDecoder {
    pub fn new() -> ZstdDecoder;
    pub fn decompress(&mut self, input: &[u8], output: &mut Vec<u8>) -> Result<(), ZstdError>;
}
```

- `decompress` decodes a Zstd stream and appends the result to `output`.

---

## 2. Dictionary Support

Zstd supports training a dictionary from a set of samples to improve compression on small data.  The crate provides:

```
pub fn train_dictionary(samples: &[&[u8]], max_size: usize) -> Result<Vec<u8>, ZstdError>;
pub fn compress_with_dictionary(data: &[u8], dict: &[u8], level: i32) -> Result<Vec<u8>, ZstdError>;
pub fn decompress_with_dictionary(data: &[u8], dict: &[u8]) -> Result<Vec<u8>, ZstdError>;
```

- `train_dictionary` uses a fast algorithm (similar to `zstd --train`) to produce a dictionary from sample data.

---

## 3. Convenience Functions

```
pub fn compress(data: &[u8], level: i32) -> Result<Vec<u8>, ZstdError>;
pub fn decompress(data: &[u8]) -> Result<Vec<u8>, ZstdError>;
```

- One‑shot compress and decompress for data that fits in memory.

---

## 4. Streaming Interface

For large data, the crate provides `ZstdWriter<W: Write>` and `ZstdReader<R: Read>` adaptors, analogous to those in `blaze‑compress`.  They implement the standard `Read`/`Write` traits.

---

## 5. Error Handling

```
pub enum ZstdError {
    InvalidInput,
    CompressionFailed,
    DecompressionFailed,
    OutputBufferTooSmall,
    DictionaryTooLarge,
    TrainingFailed,
}
```

---

## 6. Testing

- **Compress/decompress:** Round‑trip random data of various sizes, verify corruption‑free.
- **Levels:** Compare compressed size for levels 1, 3, 19, 22 (regression test).
- **Dictionary:** Train a dictionary on similar small strings, compress with dictionary, verify decompression works.
- **Edge cases:** Empty input, large input (100 MB streaming).
- **Error handling:** Provide truncated compressed data, expect `DecompressionFailed`.

All tests must pass on all platforms.
