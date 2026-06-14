# Summary of Architecture Evolution

This document explains how the architecture has evolved from the original monolithic design to the current workspace-based design.

## Original Design (Monolithic)

The original design (documented in `docs/架构设计.md`) described a monolithic Rust application with:

- **Flat 2-layer architecture**: Entry layer (CLI/HTTP) → Core layer (read/write/data/VBA/diff)
- **Single crate**: All modules in one Rust crate
- **Python diff project**: Separate `excel-tool-diff/` for Python-based diff with git integration

## Current Design (Workspace)

The current design (implemented from Phase 1 onward) uses a Cargo workspace:

- **Workspace layout**: 4 independent crates
- **Dependency management**: Clear acyclic dependency graph
- **Rust-only**: No Python implementation

### Key Changes

| Original | Current |
|----------|---------|
| Single crate | Workspace with 4 crates |
| Python diff project | Rust `excel-diff` crate |
| Manual feature gating | Independent binary crates |
| Diff auto-generated in write ops | Diff invoked at entry layer |
| Monolithic compilation | Independent compilation |

### Motivation for Change

1. **Circular dependency prevention**: Write operations needing diff led to potential circular dependencies.
2. **Independent reuse**: `excel-diff` can be used standalone (e.g., as git diff driver).
3. **Build optimization**: CLI doesn't need HTTP dependencies.
4. **Clearer separation**: Each crate has a single responsibility.

## Migration Path

The migration from monolithic to workspace will be implemented as part of Phase 1. All new development will follow the workspace structure.

## Cross-Reference

- **Original architecture**: `docs/架构设计.md` (reference only)
- **Workspace architecture**: `docs/architecture/`
- **Implementation plan**: `docs/plan/`

All new implementation should follow the workspace-based design.