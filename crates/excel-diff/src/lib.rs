mod api_response;
mod engine;
mod file_diff;
mod formula_tracker;
pub mod git_driver;
mod helpers;
mod range_diff;
pub mod semantic;
mod sheet_diff;
pub mod summarize;

pub use file_diff::diff_files;
pub use git_driver::get_git_diff_file_paths;
pub use range_diff::diff_range;
pub use sheet_diff::diff_sheets;

// Export diff_sheet_data API for direct in-memory comparison
pub use engine::compute_cell_diffs as diff_sheet_data;

use excel_types::{CellDiff, SheetData};

/// Compare two SheetData objects in memory.
/// This is useful for testing and scenarios where you already have the data loaded.
///
/// # Arguments
/// * `old_data` - The old sheet data
/// * `new_data` - The new sheet data
///
/// # Returns
/// A vector of cell differences
///
/// # Examples
/// ```
/// use excel_diff::{diff_sheet_data, compute_diffs};
/// use excel_types::{CellData, CellDataType, SheetData};
///
/// let old = SheetData {
///     name: "Sheet1".into(),
///     rows: vec![vec![CellData {
///         value: Some("old".into()),
///         data_type: CellDataType::String,
///         formula: None,
///     }]],
/// };
/// let new = SheetData {
///     name: "Sheet1".into(),
///     rows: vec![vec![CellData {
///         value: Some("new".into()),
///         data_type: CellDataType::String,
///         formula: None,
///     }]],
/// };
/// let diffs = compute_diffs(&old, &new);
/// assert_eq!(diffs.len(), 1);
/// ```
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

    #[test]
    fn test_diff_sheet_data_works() {
        let old = SheetData {
            name: "Sheet1".into(),
            rows: vec![vec![CellData {
                value: Some("old".into()),
                data_type: CellDataType::String,
                formula: None,
            }]],
        };
        let new = SheetData {
            name: "Sheet1".into(),
            rows: vec![vec![CellData {
                value: Some("new".into()),
                data_type: CellDataType::String,
                formula: None,
            }]],
        };

        let diffs = diff_sheet_data(&old, &new);
        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].diff_type, DiffType::Modify);
        assert_eq!(diffs[0].old_value, Some("old".into()));
        assert_eq!(diffs[0].new_value, Some("new".into()));
    }

    #[test]
    fn test_diff_sheet_data_empty() {
        let old = SheetData {
            name: "Sheet1".into(),
            rows: vec![],
        };
        let new = SheetData {
            name: "Sheet1".into(),
            rows: vec![],
        };

        let diffs = diff_sheet_data(&old, &new);
        assert!(diffs.is_empty());
    }

    #[test]
    fn test_diff_sheet_data_add() {
        let old = SheetData {
            name: "Sheet1".into(),
            rows: vec![],
        };
        let new = SheetData {
            name: "Sheet1".into(),
            rows: vec![vec![CellData {
                value: Some("new".into()),
                data_type: CellDataType::String,
                formula: None,
            }]],
        };

        let diffs = diff_sheet_data(&old, &new);
        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].diff_type, DiffType::Add);
    }

    #[test]
    fn test_diff_sheet_data_delete() {
        let old = SheetData {
            name: "Sheet1".into(),
            rows: vec![vec![CellData {
                value: Some("old".into()),
                data_type: CellDataType::String,
                formula: None,
            }]],
        };
        let new = SheetData {
            name: "Sheet1".into(),
            rows: vec![],
        };

        let diffs = diff_sheet_data(&old, &new);
        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].diff_type, DiffType::Delete);
    }
}
