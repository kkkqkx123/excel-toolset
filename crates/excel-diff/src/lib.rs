pub mod diff_core;
pub mod git_driver;

pub use diff_core::{diff_files, diff_range, diff_sheets};

use excel_core::types::{CellDiff, SheetData};

pub fn compute_diffs(old_data: &SheetData, new_data: &SheetData) -> Vec<CellDiff> {
    diff_core::compute_cell_diffs(old_data, new_data)
}
