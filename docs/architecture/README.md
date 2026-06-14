# Architecture Overview

This document describes the architecture of the Excel Tool Gateway project.

## Project Overview

| Dimension | Content |
|-----------|---------|
| Name | Excel Tool Gateway |
| Type | Rust Workspace (monorepo) |
| Target | AI Agent Excel operations — CLI + HTTP gateway |
| Core Dependencies | `calamine` (read-only) + `rust_xlsxwriter` (write-only) |
| Architecture Principle | Flat two-layer (entry → core); no DDD; workspace-level modularity |

## Core Design Principles

1. **Workspace-level modularity**: The project is a Cargo workspace. The Rust codebase is split into independent, dependency-minimized crates following the "下沉核心 + 双入口上浮" (sink core, lift interfaces) pattern.

2. **Core engine下沉**: All Excel operations (read, write, data processing, security, VBA) live in a shared `excel-core` crate. CLI and HTTP entry crates depend on it — it never depends on them.

3. **Diff独立可扩展**: The `excel-diff` crate is an independent crate that depends on `excel-core`'s read capabilities. It provides both standalone diff queries and write-operation-affiliated diff generation. It can serve as a git diff driver or web UI diff backend.

4. **双入口上浮**: `excel-cli` and `excel-http` are independent entry crates sharing only `excel-core`. Neither depends on the other. Feature gating (CLI vs HTTP) is replaced by independent binary crates.

5. **Write operations do NOT automatically call diff**: All write operations return raw results. The calling layer (CLI/HTTP) decides whether to invoke diff — either as a post-write affiliated diff or as an independent diff query. This avoids circular dependencies between `excel-core`'s write module and `excel-diff`.

6. **No Python implementation**: The `excel-tool-diff` directory and `excel-tool-diff/docs/research/` documents are research references only. The actual implementation is entirely in Rust.

## Module Map

```
excel-tool-gateway (workspace root)
├── excel-core         # read, write, data, security, vba, types, cell_ref
│   └── no external entry dependency
├── excel-diff         # diff engine (depends on excel-core read)
│   └── no entry dependency
├── excel-cli          # clap binary (depends on excel-core, excel-diff)
│   └── no http dependency
├── excel-http         # axum binary (depends on excel-core, excel-diff)
│   └── no cli dependency
└── main.rs            # workspace-level aggregate entry (calls each binary)
```

For the complete workspace structure, data flow, and implementation details, see [cargo-workspace.md](./cargo-workspace.md).

## Technology Stack

| Layer | Technology |
|-------|-----------|
| Excel read | `calamine` 0.31 |
| Excel write | `rust_xlsxwriter` 0.50 |
| CLI | `clap` 4 |
| HTTP | `axum` 0.7 + `tokio` |
| Serialization | `serde` + `serde_json` |
| Security | `sha2`, `chrono` |

## Phases

See [docs/plan/README.md](../plan/README.md) for the phased implementation plan.

- **P1**: Workspace scaffold + core types + basic modules
- **P2**: Read/write core
- **P3**: Data processing + advanced capabilities
- **P4**: CLI + HTTP entries (as separate crates)
- **P5**: Diff subsystem (as separate crate)
- **P6**: Integration, testing, and release