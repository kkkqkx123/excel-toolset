# AGENTS.md

**No-backward-compatible**
At present, the project is in the development stage and there is no need to specifically consider backward compatibility. Prioritize ensuring the long-term maintainability of the architecture and refactoring design defects as early as possible.

**Language**
Always use English in code files(include config files, comments) and use Simplified Chinese in docs.

**Plan/Design Document**
Avoid including complete code snippets. Mainly using concise natural language descriptions.

**Security Assurance**
Always avoid the use of unwrap. In testing, substitute with expect.
Refrain from using unsafe methods except where directly involving low-level operations.
All instances of unsafe usage must be explicitly documented in the unsafe.md file within the docs\archive directory.

**Type Design Guidelines**
Minimise the use of dynamic dispatch forms such as `dyn`, always prioritising deterministic types.
All instances of dynamic dispatch must be explicitly documented in the `dynamic.md` file within the `docs\archive` directory.

## Repository structure

```
crates/
├── excel-types/   Shared type definitions (CellValue, errors, enums)
├── excel-core/    Core read/write/security/data/vba
├── excel-diff/    Diff engine (depends on excel-core read types)
├── excel-sql/     SQL query engine for Excel files
├── excel-mcp/     MCP Server (JSON-RPC over stdio, 66 tools)
├── excel-cli/     CLI binary (depends on core + diff)
└── excel-http/    HTTP binary (depends on core + diff)
excel-tool-gateway/ (stub — superseded by workspace above)
excel-tool-diff/   (research reference, no Python implementation planned)
docs/
├── architecture/   Architecture docs (5 files)
└── plan/           Phased implementation plan (phase1–phase10)
ref/                Reference source: calamine + rust_xlsxwriter
```

## Cargo workspace structure

All Rust code lives under `crates/`. The workspace root `Cargo.toml` defines 7 members:

- **excel-types**: Shared type definitions used across the workspace
  - `CellValue` enum (String/Number/Bool/Error/Empty), error types, common enums
- **excel-core**: Flat 2-layer library (entry → core functions). No DDD, no service layer.
  - Write pattern: `calamine.read → create new Workbook → rust_xlsxwriter.write → overwrite file`
  - All write ops call security module (hash → backup → dry-run check → execute)
  - Write operations **do not** auto-call diff (avoids circular dep with excel-diff)
- **excel-diff**: Reusable diff engine, depends on excel-core read types only
  - Used by excel-cli and excel-http for affiliated diffs
  - Also usable standalone as git diff driver backend
- **excel-sql**: SQL query engine for Excel files
  - Enables SQL-based data queries on spreadsheet data
- **excel-mcp**: MCP Server implementing JSON-RPC 2.0 over stdio
  - 66 tools across 22 categories (read/write/sheet/format/chart/pivot/comment/merge/style/data-validation/conditional-format/protection/filter/sort/search/freeze-pane/page-setup/defined-name/batch/backup/image/macro)
  - Compatible with Claude Code, Cursor, VS Code MCP clients
  - Start with: `cargo run --bin excel-mcp`
- **excel-cli**: CLI binary. Output JSON by default (`--pretty` for human-readable)
- **excel-http**: HTTP binary. Unified `ApiResponse<T>` response format

## Key conventions

- **Language**: English in code/comments/logging/errors. Chinese in docs (except code blocks).
- **No backward compatibility** during development. Prioritize long-term maintainability.
- **Build commands**:
  - `cargo build --workspace`
  - `cargo test --workspace`
  - `cargo clippy --workspace -- -D warnings`
  - `cargo fmt --check`
- **Implementation order**: Follow `docs/plan/` phases sequentially (P1→P10).

## excel-tool-diff (research only)

The `excel-tool-diff/` directory contains research reference only. No Python implementation is planned. The Rust `excel-diff` crate handles all diff functionality.
