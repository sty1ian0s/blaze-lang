# Phase 4 – Ecosystem Crate: `blaze‑arch`

> **Goal:** Provide a data‑oriented, zero‑copy library for reading and writing CPU architecture‑specific binary formats (ELF, PE, Mach‑O, and raw binary).  It is the foundation for build tools, linkers, debuggers, and bootloaders.  All parsing is pure and deterministic; writing to files carries the `io` effect.

---

## 1. Core Types

### 1.1 `ObjectFile`

```
pub enum ObjectFile {
    Elf32(ElfFile32),
    Elf64(ElfFile64),
    Pe(PeFile),
    MachO(MachOFile),
    Raw(Vec<u8>),
}
```

- Represents a parsed object file.  The variants correspond to the supported formats.

### 1.2 `ElfFile64`

```
pub struct ElfFile64 {
    pub header: ElfHeader64,
    pub section_headers: Vec<ElfSectionHeader64>,
    pub sections: Vec<SectionData>,
    pub program_headers: Vec<ElfProgramHeader64>,
    pub segments: Vec<SegmentData>,
}
```

- Similar structures for ELF32, PE, and Mach‑O, each with their format‑specific headers.

---

## 2. Parsing

```
pub fn parse(data: &[u8]) -> Result<ObjectFile, ArchError>;
```

- Auto‑detects the format from the magic bytes and parses the file into the appropriate variant.

---

## 3. Writing

```
pub fn write(obj: &ObjectFile, path: &str) -> Result<(), ArchError>;
pub fn write_to_vec(obj: &ObjectFile) -> Result<Vec<u8>, ArchError>;
```

- Serializes the object file back into bytes, preserving headers and alignment.

---

## 4. Error Handling

```
pub enum ArchError {
    Io(std::io::Error),
    InvalidFormat,
    UnsupportedArchitecture,
    UnsupportedVersion,
}
```

---

## 5. Testing

- **ELF round‑trip:** Parse an ELF binary, write it back, verify byte‑for‑byte equality.
- **PE round‑trip:** Same for Windows PE executables.
- **Invalid input:** Provide malformed files, expect `InvalidFormat`.

All tests must pass on all platforms.
