use rust_xlsxwriter::Workbook;

use crate::security::compute_file_hash;
use crate::types::*;

mod batch;
mod core;
mod csv;
mod data_mut;
mod format;
mod operations;

pub use batch::*;
pub use core::{ensure_dimensions, modify_file, modify_file_with_wb, write_sheet_data};
pub use csv::*;
// data_mut functions are pub(crate), not re-exported publicly.
// Use operations.rs for file-level public API.
pub use format::{build_format, parse_color};
pub use operations::*;

pub fn create_file(path: &str, sheet_name: &str) -> Result<WriteResult> {
    let mut wb = Workbook::new();
    let ws = wb.add_worksheet();
    ws.set_name(sheet_name).map_err(AppError::Xlsx)?;
    wb.save(path).map_err(AppError::Xlsx)?;

    let new_hash = compute_file_hash(path).map_err(AppError::Io)?;
    Ok(WriteResult {
        success: true,
        message: format!("Created {}", path),
        backup_info: None,
        old_hash: String::new(),
        new_hash,
        diff: None,
    })
}
