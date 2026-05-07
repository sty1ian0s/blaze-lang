# Blaze Implementation Roadmap

> **Status:** Normative for the Blaze project.  This document defines the delivery order, milestones, and estimated effort for every phase of the Blaze language, compiler, standard library, and ecosystem.  It reflects the JIT‑first strategy: the JIT is built first and used to develop everything else; the AOT compiler is produced from the JIT codebase via `blaze fix`.

---

## 1. Phases at a Glance

| Phase | Name | Main Deliverable | Depends On |
|-------|------|------------------|------------|
| 0 | Bootstrap Interpreter (`blazei`) | Go‑based interpreter for minimal subset | – |
| 1 | Self‑Hosting JIT Compiler (`blazej`) | Full parser + flat AST + JIT codegen, written in Blaze‑Core, compiled by `blazei` | Phase 0 |
| 2 | Full Language | Generics, linear types, effects, actors, contracts, attributes, all comptime macros | Phase 1 |
| 3a | Core Standard Library | `builtins`, `mem`, `string`, `io`, `fmt`, `iter`, `cmp`, `collections`, … | Phase 2 |
| 3b | Systems Standard Library | `fs`, `path`, `sync`, `future`, `time`, `os`, … | Phase 3a |
| 3c | Extended Standard Library | `simd`, `random`, `fixed`, `regex`, `dynamic`, `meta`, `hardware`, static‑allocation | Phase 3b |
| 3d | Specialised Standard Library (optional) | `gpu`, `l10n` | Phase 3c |
| 4‑5 | Ecosystem Crates | 87 crates for networking, graphics, data, embedded, etc. | Phase 3c (core std) |
| AOT | Production AOT Compiler | Full AOT compiler, built from JIT sources via `blaze fix` | Phase 2 (JIT compiler) |

---

## 2. Phase 0 – Bootstrap Interpreter

**Goal:** A Go program that parses and executes a tiny subset of Blaze (affine variables, `i32`, `bool`, functions, structs, `if`/`while`/`loop`).  It needs no self‑hosting capability.

**Key tasks:**
- Lexer, recursive‑descent parser for the Phase 0 grammar.
- Semantic analysis (symbol table, type checker for `i32`/`bool`, affine‑use checker).
- Stack‑based byte‑code compiler and interpreter.
- Tests for every construct.

**Deliverable:** `blazei` binary that can run `.blz` files and print results or exit with an error.

**Effort:** ~2‑3 months for an experienced systems developer.

---

## 3. Phase 1 – Self‑Hosting JIT Compiler

**Goal:** Write a Blaze compiler **in Blaze** (using the subset understood by `blazei`) that accepts the **full** Blaze syntax, builds the flat AST, and performs complete semantic analysis.  For code generation it uses a simple JIT (native code via the Blaze‑Machine backend, or a fast interpreter).  The compiler is first compiled by `blazei`, then self‑hosts.

**Key tasks:**
- Lexer and parser for the full LL(1) grammar, producing the flat AST.
- Semantic analysis pass (name resolution, full type checking, linearity, region inference, effect inference).
- JIT code generation (fast debug mode, opt‑level 0).
- C‑emitter fallback so that the compiler can be bootstrapped as a native binary.

**Deliverable:** `blazej` binary that compiles and immediately runs any Blaze program (with “not yet supported” for unimplemented language features).

**Effort:** ~4‑6 months after Phase 0.

---

## 4. Phase 2 – Full Language

**Goal:** Incrementally add all remaining language features to the JIT compiler, using the test‑first approach.

**Sub‑phases (each 1‑2 months):**
- **2a:** Expanded types & linear ownership (`@copy`, `Dispose`, partial moves).
- **2b:** Generics, traits, `impl`, `dyn Trait`, `where`, coherence/orphan rules.
- **2c:** Effect system, purity, auto‑parallelisation.
- **2d:** Actors, async, channels, supervision, runtime scheduler.
- **2e:** Contracts, invariants, `@fuzz`, `@test_case`, doctests, release‑mode removal.
- **2f:** All remaining attributes, conditional compilation, domain bundles, macros.

**Deliverable:** JIT compiler that supports the entire Blaze language.  The tooling (`blaze check`, `blaze test`, `blaze fmt`, `blaze lsp`) is developed alongside.

**Effort:** ~10‑12 months cumulative from Phase 1.

---

## 5. Phase 3a – Core Standard Library

**Goal:** Implement every module defined in the core library specification, using JIT for development.

**Modules:** `builtins`, `mem`, `string`, `io`, `fmt`, `iter`, `cmp`, `clone`, `default`, `debug`, `hash`, `collections`, `env`, `process`, `ops`, `units`, `window`.

**Key tasks:**
- Write tests, then implementation, for each module.
- Build the pre‑compiled `.blzlib` archives for the host platform.

**Deliverable:** Core library usable from any Blaze program.

**Effort:** ~3‑4 months, parallelisable across multiple contributors.

---

## 6. Phase 3b – Systems Library

**Modules:** `fs`, `path`, `sync`, `future`, `binary`, `endian`, `time`, `os`.

**Effort:** ~2 months.

---

## 7. Phase 3c – Extended Library

**Modules:** `simd`, `random`, `fixed`, `regex`, `dynamic`, `meta`, `hardware`, static‑allocation support.

**Effort:** ~3 months.

---

## 8. Phase 3d – Specialised Library (Optional)

**Modules:** `gpu`, `l10n`.

**Effort:** ~1‑2 months.

---

## 9. Phases 4‑5 – Ecosystem Crates

These crates are developed by the community and the core team over time.  The official list of 87 crates is prioritised by demand.  Each crate is a separate project with its own test suite.

**Examples of early focus crates:** `blaze‑serde`, `blaze‑http`, `blaze‑json`, `blaze‑toml`, `blaze‑gui`, `blaze‑sql`, `blaze‑crypto`.

**Effort:** Ongoing, years 3‑5 of the project.

---

## 10. AOT Compiler

**Goal:** Produce a fully optimising ahead‑of‑time compiler from the same source code as the JIT compiler.

**How:**
1. The JIT compiler source is run through `blaze fix --aot-ify` to add all required type annotations, contracts, and effect specifications.
2. The resulting fully‑annotated code is compiled by the JIT compiler with the full optimisation pipeline enabled.
3. The output is a statically‑linked, optimised native binary – the **production Blaze compiler**.

**Deliverable:** `blazec` – the AOT compiler used for release builds.

**Effort:** ~2‑3 months after the JIT compiler is feature‑complete (Phase 2).  The actual work is mostly in the optimiser; the front‑end is shared.

---

## 11. Key Milestones

1. **M1 – “Hello, Blaze”** (Phase 0 + early Phase 1) – First self‑hosting compile: `blazei` compiles a Blaze‑written compiler that compiles itself.  Marks the point where the project leaves Go behind.

2. **M2 – Language Complete** (end of Phase 2) – Every language feature works in JIT mode.  Developer tools (`check`, `test`, `fmt`, `lsp`) are usable.

3. **M3 – Standard Library Ready** (end of Phase 3c) – All mandatory standard library modules are implemented, tested, and available.

4. **M4 – Production AOT** (AOT compiler delivered) – `blazec` produces optimised native binaries.  The language is ready for production use.

5. **M5 – Ecosystem Launch** (first wave of Phase 4 crates) – The most important ecosystem crates (`serde`, `http`, `json`, `gui`, `sql`, `crypto`) are stable enough for early adopters.

---

## 12. Timeline (Estimated)

| Milestone | Cumulative Time |
|-----------|-----------------|
| M1 – Hello, Blaze | 8‑12 months |
| M2 – Language Complete | 24‑30 months |
| M3 – Standard Library Ready | 30‑36 months |
| M4 – Production AOT | 32‑38 months |
| M5 – Ecosystem Launch | 36‑48 months |

Times assume a small team of 2‑5 full‑time contributors.  With more contributors, the library and ecosystem phases can be accelerated significantly.
