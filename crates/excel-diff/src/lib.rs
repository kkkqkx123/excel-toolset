pub mod api_response;
pub mod engine;
pub mod file_diff;
pub mod formula_tracker;
pub mod git_driver;
pub mod helpers;
pub mod range_diff;
pub mod semantic;
pub mod sheet_diff;
pub mod summarize;

pub use file_diff::diff_files;
pub use range_diff::diff_range;
pub use sheet_diff::diff_sheets;

use excel_types::{CellDiff, SheetData};

pub fn compute_diffs(old_data: &SheetData, new_data: &SheetData) -> Vec<CellDiff> {
    engine::compute_cell_diffs(old_data, new_data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use excel_types::{CellData, CellDataType, DiffType};

    #[test]
    fn test_compute_diffs_delegates_to_engine() {
        let old = SheetData {
            name: "S".into(),
            rows: vec![vec![CellData {
                value: Some("old".into()),
                data_type: CellDataType::String,
                formula: None,
            }]],
        };
        let new = SheetData {
            name: "S".into(),
            rows: vec![vec![CellData {
                value: Some("new".into()),
                data_type: CellDataType::String,
                formula: None,
            }]],
        };
        let diffs = compute_diffs(&old, &new);
        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].diff_type, DiffType::Modify);
    }

    #[test]
    fn test_compute_diffs_identical() {
        let data = SheetData {
            name: "S".into(),
            rows: vec![],
        };
        let diffs = compute_diffs(&data, &data);
        assert!(diffs.is_empty());
    }
}