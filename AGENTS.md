# AGENTS.md

## Repository structure

```
excel-tool-gateway/    (Rust, planned — no Cargo.toml yet)
excel-diff/            (Python, planned — no pyproject.toml yet)
├── ref/               Reference source: calamine + rust_xlsxwriter
├── docs/              Design docs, plans
│   ├── plan/          Phased implementation plan (6 phases)
│   └── ...
└── diff/              Diff subsystem research
```

This repo contains **two independent sub-projects** under one monorepo. The project is pre-implementation — only docs and reference code exist.

## Sub-project 1: excel-tool-gateway (Rust)

- **Purpose**: CLI + HTTP gateway for Excel atomic operations, designed for AI Agent consumption.
- **Dependencies**: `calamine` (read-only), `rust_xlsxwriter` (write-only), `clap`, `axum`, `serde`.
- **Ref source**: `ref/calamine/`, `ref/rust_xlsxwriter/`.
- **Architecture**: Flat 2-layer (entry CLI/HTTP → core functions). No DDD, no service layer.
- **Write pattern**: `calamine.read → create new Workbook → rust_xlsxwriter.write → overwrite file`. `rust_xlsxwriter` cannot modify existing files.
- **All write ops**: MUST call security module (hash → backup → dry-run check → execute).
- **Diff**: Built-in `excel_diff` module for cell/row/sheet/file comparison; all write ops auto-return diff.
- **No Cargo.toml yet**: Phase 1 of `docs/plan/phase1-foundation.md` must be completed before any Rust code compiles.
- **Build** (once scaffolded): `cargo build`, `cargo test`, `cargo clippy`, `cargo fmt --check`.

## Sub-project 2: excel-diff (Python)

- **Purpose**: Git-integrated Excel diff tool with smart formula noise reduction.
- **Dependencies**: `openpyxl`, `GitPython`, `click`, `rich`.
- **Ref source**: `diff/docs/research/`.
- **MVP delivery**: CLI first, Web frontend second.
- **Git driver**: registers as `git diff` driver for `.xlsx` files.
- **Formula noise reduction**: Distinguishes active edits from auto-calculated formula cascades.

## Language

Always use English in code, comments, logging, error info or other string literal. Use Chinese in docs (except code block)
**Never use any Chinese in any code files or code block.**

## Key conventions

- **CLI output**: Default is JSON (`--pretty` for human-readable).
- **HTTP response**: Unified `ApiResponse<T>` with `{success, message, file_hash, data, diff, backup_info}`.
- **Clip dependencies**: `cargo clippy -- -D warnings` required before all PRs.
- **Implementation order**: Follow `docs/plan/` phases sequentially (P1→P6).

**No-backward-compatible**
At present, the project is in the development stage and there is no need to specifically consider backward compatibility. Prioritize ensuring the long-term maintainability of the architecture and refactoring design defects as early as possible.
