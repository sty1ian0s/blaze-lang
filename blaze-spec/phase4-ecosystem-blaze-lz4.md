# Phase 4 – Ecosystem Crate: `blaze‑lz4`

> **Goal:** Provide a standalone, zero‑copy LZ4 compression and decompression library built entirely in Blaze.  It supports the standard LZ4 block format and the LZ4 Frame format for compressed streams.  All operations are pure and operate on byte slices; no heap allocation occurs inside the hot path.  The crate is designed for applications that need extremely fast compression without pulling in the full `blaze‑compress` dependency.

---

## 1. Core Types

### 1.1 `Lz4Encoder`

```
pub struct Lz4Encoder {
    acceleration: u32,
    // internal state
}
impl Lz4Encoder {
    pub fn new(acceleration: u32) -> Lz4Encoder;
    pub fn compress(&mut self, input: &[u8], output: &mut Vec<u8>) -> Result<usize, Lz4Error>;
    pub fn finish(&mut self, output: &mut Vec<u8>) -> Result<(), Lz4Error>;
}
```

- `acceleration` controls the trade‑off between speed and compression ratio (1 = best compression, higher = faster).
- `compress` compresses a chunk of data and appends the result to `output`.  It returns the number of compressed bytes written.
- `finish` flushes any remaining data (for frame format).  The block format does not require finishing.

### 1.2 `Lz4Decoder`

```
pub struct Lz4Decoder { … }
impl Lz4Decoder {
    pub fn new() -> Lz4Decoder;
    pub fn decompress(&mut self, input: &[u8], output: &mut Vec<u8>) -> Result<(), Lz4Error>;
}
```

- `decompress` decodes an LZ4 block or frame and appends the decompressed data to `output`.

---

## 2. Convenience Functions

```
pub fn compress_block(data: &[u8]) -> Result<Vec<u8>, Lz4Error>;
pub fn decompress_block(data: &[u8]) -> Result<Vec<u8>, Lz4Error>;
pub fn compress_frame(data: &[u8]) -> Result<Vec<u8>, Lz4Error>;
pub fn decompress_frame(data: &[u8]) -> Result<Vec<u8>, Lz4Error>;
```

- One‑shot functions that allocate and return the result, suitable for data that fits in memory.

---

## 3. LZ4 Block Format

The block format consists of a sequence of literal runs and match copies.  The encoder uses a hash table to find matches in the previous data.  The implementation follows the LZ4 specification exactly.  The maximum back‑reference distance is 64 KiB.

---

## 4. LZ4 Frame Format

The frame format adds a magic number, content size (optional), and an end mark.  It allows the data to be split into independently decompressible blocks.  The crate supports both reading and writing LZ4 frames.

---

## 5. Error Handling

```
pub enum Lz4Error {
    InvalidInput,
    OutputBufferTooSmall,
    CompressionFailed,
    DecompressionFailed,
}
```

---

## 6. Testing

- **Block round‑trip:** Compress a random byte array, decompress, verify equality.
- **Frame round‑trip:** Compress a byte array as a frame, decompress, verify.
- **Edge cases:** Empty input, input containing only repeated bytes, large input (several MB).
- **Error handling:** Provide a corrupted compressed buffer, expect `DecompressionFailed`.

All tests must pass on all platforms.
