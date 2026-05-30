# Cargo Workspace Structure

This document details the Cargo workspace structure for the Excel Tool Gateway project.

## Overall Architecture

```
excel-tool-gateway/                    # Workspace root
├── Cargo.toml                        # [workspace] manifest
├── crates/
│   ├── excel-core/                   # Core engine crate (lib)
│   │   ├── src/lib.rs                # Public API: read, write, data, security, vba
│   │   ├── src/types.rs              # Shared types: ApiResponse, FileInfo, etc.
│   │   ├── src/cell_ref.rs           # Cell reference parsing
│   │   ├── src/file_util.rs          # File operations
│   │   ├── src/security.rs         # Security: hash, backup, rollback
│   │   ├── src/excel_read.rs       # Read module (calamine)
│   │   ├── src/excel_write.rs      # Write module (rust_xlsxwriter)
│   │   ├── src/excel_data.rs       # Data processing
│   │   └── src/vba_util.rs          # VBA utilities
│   │
│   ├── excel-diff/                 # Diff engine crate (lib)
│   │   ├── src/lib.rs                # Public API: diff_files, diff_sheets, compute_diffs
│   │   ├── src/diff_core.rs         # Diff algorithms and structures
│   │   └── src/git_driver.rs        # Git integration (diff driver)
│   │
│   ├── excel-cli/                  # CLI binary crate (bin)
│   │   ├── src/main.rs               # Entry point
│   │   ├── src/cli/
│   │   │   ├── mod.rs
│   │   │   ├── commands.rs           # All clap subcommands
│   │   │   └── handlers.rs           # Command logic
│   │   └── Cargo.toml              # Depends on excel-core, excel-diff
│   │
│   └── excel-http/                 # HTTP binary crate (bin)
│       ├── src/main.rs             # Entry point
│       ├── src/http/
│       │   ├── mod.rs
│       │   ├── router.rs            # Route definitions
│       │   ├── handlers.rs          # Request handlers
│       │   └── middleware.rs        # Global middleware
│       └── Cargo.toml              # Depends on excel-core, excel-diff
│
├── .gitignore
├── LICENSE
└── README.md
```

## Key Design Decisions

### 1. Why a Workspace?

- **Independent compilation**: `excel-cli` can compile without pulling in `axum`/`tokio`, reducing build times and dependencies.
- **Modular reuse**: `excel-diff` can be used by external tools (e.g., git diff drivers) without including CLI or HTTP code.
- **Clear boundaries**: Each crate has a single responsibility.
- **No circular dependencies**: The dependency graph is strictly acyclic:
  ```
  excel-cli → excel-core
  excel-cli → excel-diff
  excel-http → excel-core
  excel-http → excel-diff
  excel-diff → excel-core
  ```

### 2. Core Engine (`excel-core`)

- A library crate providing all core Excel functionality.
- **Dependencies**: `calamine`, `rust_xlsxwriter`, `serde`, `sha2`, `chrono`
- **Exports**: Pure functions only — no entry logic.
- **Key modules**:
  - `excel_read`: Read-only operations via calamine
  - `excel_write`: Write operations via rust_xlsxwriter ("read → new workbook → write" pattern)
  - `security`: File hashing, backup creation, rollback
  - `types`: Shared types across all crates

### 3. Diff Engine (`excel-diff`)

- An independent library crate focused solely on diff computation.
- **Dependencies**: `excel-core`, `serde`
- **Exports**:
  - `diff_files(old_path, new_path)` → `Result<FileDiff>`
  - `diff_sheets(...)` → `Result<SheetDiff>`
  - `compute_diffs(old_data, new_data)` → `Vec<CellDiff>` (for use after writes)
  - `install_git_driver()` → Registers as system git diff tool for .xlsx files
- **Does NOT depend on any entry crate** — keeps it reusable.

### 4. CLI Entry (`excel-cli`)

- A binary crate that depends on `excel-core` and `excel-diff`.
- **Dependencies**: `excel-core`, `excel-diff`, `clap`
- **Exports**: No public API — executable only.
- **Workflow**:
  - Parse command-line arguments
  - For reads: call `excel-core`
  - For writes: call `excel-core` write + optionally call `excel-diff` to generate affiliated diff
  - Format result as JSON (default) or pretty-printed

### 5. HTTP Entry (`excel-http`)

- A binary crate that depends on `excel-core` and `excel-diff`.
- **Dependencies**: `excel-core`, `excel-diff`, `axum`, `tokio`
- **Exports**: RESTful API at `/api/*`
- **Workflow**:
  - Handle HTTP requests
  - Call `excel-core` for business logic
  - Optionally call `excel-diff` for write-affiliated diffs
  - Return unified `ApiResponse<T>` JSON

## Migration from Monolithic to Workspace

The original monolithic structure has been refactored into this workspace layout:

| Original | New |
|----------|-----|
| `src/excel_read.rs` | `crates/excel-core/src/excel_read.rs` |
| `src/excel_write.rs` | `crates/excel-core/src/excel_write.rs` |
| `src/excel_diff.rs` | `crates/excel-diff/src/diff_core.rs` |
| `src/cli/` | `crates/excel-cli/src/cli/` |
| `src/http/` | `crates/excel-http/src/http/` |

## Implementation Notes

- **Write operations no longer auto-call diff**: In the original design, `excel_write.rs` called `excel_diff::compute_cell_diffs`. This created a circular dependency risk. Now, diff generation is handled at the entry layer.
- **Shared types moved to `excel-core`**: `types.rs` is now part of `excel-core`, making it the single source of truth for shared types.
- **Security module reused**: The `security.rs` module remains in `excel-core` and is used by both entries.