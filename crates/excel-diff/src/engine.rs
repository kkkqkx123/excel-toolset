use std::collections::HashMap;

use excel_core::cell_ref;
use excel_types::{CellDiff, DiffType, SheetData, SheetDiff};

#[cfg(test)]
mod tests {
    use super::*;
    use excel_types::{CellData, CellDataType};
    use std::collections::HashMap;

    #[test]
    fn test_diff_identical_returns_no_changes() {
        let mut old = HashMap::new();
        old.insert(
            "Sheet1".to_string(),
            SheetData {
                name: "Sheet1".to_string(),
                rows: vec![vec![CellData {
                    value: Some("hello".into()),
                    data_type: CellDataType::String,
                    formula: None,
                }]],
            },
        );
        let new = old.clone();

        let diffs = diff_sheet_maps(&old, &new);
        assert!(diffs.is_empty() || diffs[0].cell_diffs.is_empty());
    }

    #[test]
    fn test_diff_detects_modification() {
        let old = sheet_data_with_values(&["hello"]);
        let new = sheet_data_with_values(&["world"]);

        let diffs = compute_cell_diffs(&old, &new);
        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].diff_type, DiffType::Modify);
        assert_eq!(diffs[0].old_value, Some("hello".into()));
        assert_eq!(diffs[0].new_value, Some("world".into()));
    }

    #[test]
    fn test_diff_empty_to_data() {
        let old = SheetData {
            name: "S".into(),
            rows: vec![],
        };
        let new = SheetData {
            name: "S".into(),
            rows: vec![vec![CellData {
                value: Some("added".into()),
                data_type: CellDataType::String,
                formula: None,
            }]],
        };
        let diffs = compute_cell_diffs(&old, &new);
        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].diff_type, DiffType::Add);
        assert_eq!(diffs[0].cell_ref, "A1");
    }

    #[test]
    fn test_diff_data_to_empty() {
        let old = SheetData {
            name: "S".into(),
            rows: vec![vec![CellData {
                value: Some("removed".into()),
                data_type: CellDataType::String,
                formula: None,
            }]],
        };
        let new = SheetData {
            name: "S".into(),
            rows: vec![],
        };
        let diffs = compute_cell_diffs(&old, &new);
        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].diff_type, DiffType::Delete);
    }

    #[test]
    fn test_diff_cell_ref_formatting() {
        let new = SheetData {
            name: "S".into(),
            rows: vec![
                vec![CellData {
                    value: Some("A1".into()),
                    data_type: CellDataType::String,
                    formula: None,
                }],
                vec![CellData {
                    value: Some("B1".into()),
                    data_type: CellDataType::String,
                    formula: None,
                }],
            ],
        };
        let diffs = compute_cell_diffs(
            &SheetData {
                name: "S".into(),
                rows: vec![],
            },
            &new,
        );
        assert_eq!(diffs.len(), 2);
        assert_eq!(diffs[0].cell_ref, "A1");
        assert_eq!(diffs[1].cell_ref, "A2");
    }

    #[test]
    fn test_diff_sheet_maps_add_sheet() {
        let mut old = HashMap::new();
        old.insert("S1".into(), empty_sheet());

        let mut new = HashMap::new();
        new.insert("S1".into(), empty_sheet());
        new.insert(
            "S2".into(),
            SheetData {
                name: "S2".into(),
                rows: vec![vec![CellData {
                    value: Some("x".into()),
                    data_type: CellDataType::String,
                    formula: None,
                }]],
            },
        );

        let diffs = diff_sheet_maps(&old, &new);
        assert_eq!(diffs.len(), 2);
        assert_eq!(diffs[1].sheet_name, "S2");
        assert!(!diffs[1].cell_diffs.is_empty());
    }

    #[test]
    fn test_diff_sheet_maps_delete_sheet() {
        let mut old = HashMap::new();
        old.insert("S1".into(), empty_sheet());
        old.insert("S2".into(), empty_sheet());

        let mut new = HashMap::new();
        new.insert("S1".into(), empty_sheet());

        let diffs = diff_sheet_maps(&old, &new);
        assert_eq!(diffs.len(), 2);
        assert_eq!(diffs[1].sheet_name, "S2");
    }

    #[test]
    fn test_diff_sheet_maps_row_count_diff() {
        let mut old = HashMap::new();
        old.insert(
            "S1".into(),
            SheetData {
                name: "S1".into(),
                rows: vec![vec![]],
            },
        );

        let mut new = HashMap::new();
        new.insert(
            "S1".into(),
            SheetData {
                name: "S1".into(),
                rows: vec![vec![], vec![]],
            },
        );

        let diffs = diff_sheet_maps(&old, &new);
        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].row_count_diff, 1);
    }

    fn empty_sheet() -> SheetData {
        SheetData {
            name: "S".into(),
            rows: vec![],
        }
    }

    fn sheet_data_with_values(values: &[&str]) -> SheetData {
        SheetData {
            name: "Sheet1".to_string(),
            rows: vec![
                values
                    .iter()
                    .map(|v| CellData {
                        value: Some(v.to_string()),
                        data_type: CellDataType::String,
                        formula: None,
                    })
                    .collect(),
            ],
        }
    }
}
