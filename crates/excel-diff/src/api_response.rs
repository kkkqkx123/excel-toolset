use excel_types::{DiffType, FileDiff, SheetDiff};

#[cfg(test)]
mod tests {
    use super::*;
    use excel_types::{CellDiff, DiffSummary};
    use std::collections::HashMap;

    fn make_diff(file_hash_match: bool, cell_diffs: Vec<CellDiff>) -> FileDiff {
        let summary = {
            let mut a = 0u32;
            let mut d = 0u32;
            let mut m = 0u32;
            let mut p = 0u32;
            for cd in &cell_diffs {
                match cd.diff_type {
                    DiffType::Add => a += 1,
                    DiffType::Delete => d += 1,
                    DiffType::Modify => m += 1,
                    DiffType::Passive => p += 1,
                    DiffType::NoChange => {}
                }
            }
            DiffSummary {
                adds: a as usize,
                deletes: d as usize,
                modifies: m as usize,
                passives: p as usize,
                total_changes: (a + d + m + p) as usize,
            }
        };
        FileDiff {
            file_hash_match,
            sheet_diffs: vec![SheetDiff {
                sheet_name: "Sheet1".into(),
                row_count_diff: 0,
                col_count_diff: 0,
                cell_diffs,
            }],
            summary,
        }
    }

    #[test]
    fn test_to_api_response_hash_match() {
        let diff = make_diff(true, vec![]);
        let json = to_api_response(&diff, None);
        assert_eq!(json["success"], true);
        assert_eq!(json["file_hash_match"], true);
        assert_eq!(json["summary"]["total_changes"], 0);
    }

    #[test]
    fn test_to_api_response_with_modify_diff() {
        let diff = make_diff(
            false,
            vec![CellDiff {
                row: 1,
                col: 2,
                cell_ref: "C2".into(),
                diff_type: DiffType::Modify,
                old_value: Some("old".into()),
                new_value: Some("new".into()),
                old_formula: None,
                new_formula: None,
            }],
        );
        let json = to_api_response(&diff, None);
        let sheet = &json["sheets"][0];
        let cell = &sheet["cell_diffs"][0];
        assert_eq!(cell["cell_ref"], "C2");
        assert_eq!(cell["diff_type"], "Modify");
        assert_eq!(cell["old_value"], "old");
        assert_eq!(cell["new_value"], "new");
    }

    #[test]
    fn test_to_api_response_with_formula() {
        let diff = make_diff(
            false,
            vec![CellDiff {
                row: 0,
                col: 0,
                cell_ref: "A1".into(),
                diff_type: DiffType::Modify,
                old_value: None,
                new_value: None,
                old_formula: Some("=SUM(A1:A3)".into()),
                new_formula: Some("=AVERAGE(A1:A3)".into()),
            }],
        );
        let json = to_api_response(&diff, None);
        let cell = &json["sheets"][0]["cell_diffs"][0];
        assert_eq!(cell["old_formula"], "=SUM(A1:A3)");
        assert_eq!(cell["new_formula"], "=AVERAGE(A1:A3)");
    }

    #[test]
    fn test_to_api_response_sheet_metadata() {
        let diff = FileDiff {
            file_hash_match: false,
            sheet_diffs: vec![SheetDiff {
                sheet_name: "MySheet".into(),
                row_count_diff: -1,
                col_count_diff: 2,
                cell_diffs: vec![],
            }],
            summary: DiffSummary {
                adds: 0,
                deletes: 0,
                modifies: 0,
                passives: 0,
                total_changes: 0,
            },
        };
        let json = to_api_response(&diff, None);
        let sheet = &json["sheets"][0];
        assert_eq!(sheet["sheet_name"], "MySheet");
        assert_eq!(sheet["row_count_diff"], -1);
        assert_eq!(sheet["col_count_diff"], 2);
    }

    #[test]
    fn test_to_api_response_no_dependency_chain_without_tracker() {
        let diff = make_diff(
            false,
            vec![CellDiff {
                row: 0,
                col: 0,
                cell_ref: "A1".into(),
                diff_type: DiffType::Passive,
                old_value: Some("10".into()),
                new_value: Some("20".into()),
                old_formula: Some("=B1+1".into()),
                new_formula: Some("=B1+1".into()),
            }],
        );
        let json = to_api_response(&diff, None);
        let cell = &json["sheets"][0]["cell_diffs"][0];
        assert_eq!(cell["diff_type"], "Passive");
        assert!(cell.get("dependency_chain").is_none());
    }

    #[test]
    fn test_to_api_response_with_dependency_chain() {
        let mut deps = HashMap::new();
        deps.insert("A1".into(), ["B1".into()].into());
        let tracker = FormulaTracker { dependencies: deps };
        let diff = make_diff(
            false,
            vec![CellDiff {
                row: 0,
                col: 0,
                cell_ref: "A1".into(),
                diff_type: DiffType::Passive,
                old_value: Some("10".into()),
                new_value: Some("20".into()),
                old_formula: Some("=B1+1".into()),
                new_formula: Some("=B1+1".into()),
            }],
        );
        let json = to_api_response(&diff, Some(&tracker));
        let cell = &json["sheets"][0]["cell_diffs"][0];
        assert!(cell.get("dependency_chain").is_some());
    }
}
