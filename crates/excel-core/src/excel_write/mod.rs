use rust_xlsxwriter::Workbook;

use crate::security::compute_file_hash;
use crate::types::*;

mod batch;
mod cell;
mod chart;
mod csv;
mod data;
mod ops;
mod sheet;
mod style;
mod write;

// Re-export
pub use batch::*;
pub use csv::*;
pub use ops::*;
pub use style::build_format;
pub use style::parse_color;

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
