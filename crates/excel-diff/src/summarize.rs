use excel_types::{DiffSummary, DiffType, SheetDiff};

#[cfg(test)]
mod tests {
    use super::*;
    use excel_types::CellDiff;

    fn cell_diff(diff_type: DiffType) -> CellDiff {
        CellDiff {
            row: 0,
            col: 0,
            cell_ref: "A1".into(),
            diff_type,
            old_value: None,
            new_value: None,
            old_formula: None,
            new_formula: None,
        }
    }

    #[test]
    fn test_summarize_empty() {
        let result = summarize(&[]);
        assert_eq!(result.total_changes, 0);
        assert_eq!(result.adds, 0);
        assert_eq!(result.deletes, 0);
        assert_eq!(result.modifies, 0);
        assert_eq!(result.passives, 0);
    }

    #[test]
    fn test_summarize_only_adds() {
        let sd = SheetDiff {
            sheet_name: "S1".into(),
            row_count_diff: 0,
            col_count_diff: 0,
            cell_diffs: vec![cell_diff(DiffType::Add); 3],
        };
        let result = summarize(&[sd]);
        assert_eq!(result.adds, 3);
        assert_eq!(result.total_changes, 3);
    }

    #[test]
    fn test_summarize_mixed_types() {
        let sd = SheetDiff {
            sheet_name: "S1".into(),
            row_count_diff: 0,
            col_count_diff: 0,
            cell_diffs: vec![
                cell_diff(DiffType::Add),
                cell_diff(DiffType::Add),
                cell_diff(DiffType::Delete),
                cell_diff(DiffType::Modify),
                cell_diff(DiffType::Passive),
                cell_diff(DiffType::NoChange),
            ],
        };
        let result = summarize(&[sd]);
        assert_eq!(result.adds, 2);
        assert_eq!(result.deletes, 1);
        assert_eq!(result.modifies, 1);
        assert_eq!(result.passives, 1);
        assert_eq!(result.total_changes, 5);
    }

    #[test]
    fn test_summarize_multiple_sheets() {
        let s1 = SheetDiff {
            sheet_name: "S1".into(),
            row_count_diff: 0,
            col_count_diff: 0,
            cell_diffs: vec![cell_diff(DiffType::Add); 2],
        };
        let s2 = SheetDiff {
            sheet_name: "S2".into(),
            row_count_diff: 0,
            col_count_diff: 0,
            cell_diffs: vec![cell_diff(DiffType::Delete); 3],
        };
        let result = summarize(&[s1, s2]);
        assert_eq!(result.adds, 2);
        assert_eq!(result.deletes, 3);
        assert_eq!(result.total_changes, 5);
    }
}
