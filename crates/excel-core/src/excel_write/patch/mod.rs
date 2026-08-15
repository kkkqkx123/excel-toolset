//! Preserving write: incremental rewriting at the zip level.
//!
//! Root cause: previously `modify_file` / `modify_file_with_wb` rebuilt the whole
//! workbook via `rust_xlsxwriter::Workbook::new()`, writing back only the "values + formulas"
//! that calamine read, and wiping out everything calamine did not read from the source file:
//! styles / merges / charts / comments / data validation / frozen panes / drawing layer.
//!
//! This module rewrites only the target sheet's `<sheetData>` **in place** inside the existing
//! xlsx zip package; every other part (`styles.xml`, `drawings/*`, `charts/*`,
//! `comments*.xml`, other sheets, `rels`, `[Content_Types].xml`) is copied byte-for-byte,
//! preserving 100% of the source file's features.
//!
//! Compiled only when the `zip` feature is enabled (on by default with `full`).




// ───────────────────────────────────────────────────────────────────────────
// Public entry points
// ───────────────────────────────────────────────────────────────────────────

/// Preserving write: rewrites only the cells specified by `edits` in `sheet`,
/// keeping every other zip part byte-for-byte.
///
/// Each element of `edits` is `(0-based row index, 0-based column index, new cell data)`.

// ── 子模块：单元格/视图/工作表 公共入口 + zip/sheet_model/features/sheet_mgmt 内部实现 ──
mod cells;
mod views;
mod sheets;
mod zip_io;
mod sheet_model;
mod features;
mod sheet_mgmt;

// 对外公开 API 路径保持稳定：excel_write::patch::<entry>
pub use cells::*;
pub use views::*;
pub use sheets::*;
pub use zip_io::preserve_all_parts_transfer;

// 内部实现项对子模块可见
use self::{zip_io::*, sheet_model::*, features::*, sheet_mgmt::*};

#[cfg(test)]
mod tests;
