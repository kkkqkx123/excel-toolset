pub mod api_response;
pub mod engine;
pub mod file_diff;
pub mod formula_tracker;
pub mod git_driver;
pub mod helpers;
pub mod range_diff;
pub mod sheet_diff;
pub mod summarize;

pub use file_diff::diff_files;
pub use range_diff::diff_range;
pub use sheet_diff::diff_sheets;

use excel_core::types::{CellDiff, SheetData};

pub fn compute_diffs(old_data: &SheetData, new_data: &SheetData) -> Vec<CellDiff> {
    engine::compute_cell_diffs(old_data, new_data)
}
