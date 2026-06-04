use std::collections::BTreeMap;

use excel_types::{CellDiff, DiffType, FileDiff};

#[derive(Debug, Clone)]
pub struct CellEntry {
    pub cell_ref: String,
    pub col: u16,
    pub value: String,
}

#[derive(Debug, Clone)]
pub enum LogicalOperation {
    SheetResized {
        sheet: String,
        row_delta: i32,
        col_delta: i32,
    },
    SheetAdded {
        sheet: String,
        row_count: u32,
        cell_count: usize,
    },
    SheetDeleted {
        sheet: String,
        row_count: u32,
        cell_count: usize,
    },
    RowAdded {
        sheet: String,
        row: u32,
        entries: Vec<CellEntry>,
    },
    RowDeleted {
        sheet: String,
        row: u32,
        entries: Vec<CellEntry>,
    },
    CellModified {
        sheet: String,
        cell_ref: String,
        col: u16,
        header: Option<String>,
        old_value: Option<String>,
        new_value: Option<String>,
        old_formula: Option<String>,
        new_formula: Option<String>,
    },
    CellPassive {
        sheet: String,
        cell_ref: String,
        col: u16,
        header: Option<String>,
        old_value: Option<String>,
        new_value: Option<String>,
        old_formula: Option<String>,
        new_formula: Option<String>,
    },
}

pub type GroupedDiffs = Vec<LogicalOperation>;

pub fn group_diffs(diff: &FileDiff) -> GroupedDiffs {
    let mut ops = Vec::new();

    for sheet_diff in &diff.sheet_diffs {
        if sheet_diff.row_count_diff != 0 || sheet_diff.col_count_diff != 0 {
            ops.push(LogicalOperation::SheetResized {
                sheet: sheet_diff.sheet_name.clone(),
                row_delta: sheet_diff.row_count_diff,
                col_delta: sheet_diff.col_count_diff,
            });
        }

        let rows_by_diff: BTreeMap<u32, Vec<&CellDiff>> = sheet_diff
            .cell_diffs
            .iter()
            .filter(|d| matches!(d.diff_type, DiffType::Add | DiffType::Delete))
            .fold(BTreeMap::new(), |mut acc, d| {
                acc.entry(d.row).or_default().push(d);
                acc
            });

        for (row, diffs) in &rows_by_diff {
            let all_add = diffs.iter().all(|d| matches!(d.diff_type, DiffType::Add));
            let all_delete = diffs
                .iter()
                .all(|d| matches!(d.diff_type, DiffType::Delete));

            if all_add && diffs.len() > 1 {
                let entries = diffs
                    .iter()
                    .map(|d| CellEntry {
                        cell_ref: d.cell_ref.clone(),
                        col: d.col,
                        value: d.new_value.clone().unwrap_or_default(),
                    })
                    .collect();
                ops.push(LogicalOperation::RowAdded {
                    sheet: sheet_diff.sheet_name.clone(),
                    row: *row,
                    entries,
                });
            } else if all_delete && diffs.len() > 1 {
                let entries = diffs
                    .iter()
                    .map(|d| CellEntry {
                        cell_ref: d.cell_ref.clone(),
                        col: d.col,
                        value: d.old_value.clone().unwrap_or_default(),
                    })
                    .collect();
                ops.push(LogicalOperation::RowDeleted {
                    sheet: sheet_diff.sheet_name.clone(),
                    row: *row,
                    entries,
                });
            } else {
                for diff in diffs {
                    match diff.diff_type {
                        DiffType::Add | DiffType::Delete => {
                            ops.push(LogicalOperation::CellModified {
                                sheet: sheet_diff.sheet_name.clone(),
                                cell_ref: diff.cell_ref.clone(),
                                col: diff.col,
                                header: None,
                                old_value: diff.old_value.clone(),
                                new_value: diff.new_value.clone(),
                                old_formula: diff.old_formula.clone(),
                                new_formula: diff.new_formula.clone(),
                            })
                        }
                        DiffType::Modify => ops.push(LogicalOperation::CellModified {
                            sheet: sheet_diff.sheet_name.clone(),
                            cell_ref: diff.cell_ref.clone(),
                            col: diff.col,
                            header: None,
                            old_value: diff.old_value.clone(),
                            new_value: diff.new_value.clone(),
                            old_formula: diff.old_formula.clone(),
                            new_formula: diff.new_formula.clone(),
                        }),
                        DiffType::Passive => ops.push(LogicalOperation::CellPassive {
                            sheet: sheet_diff.sheet_name.clone(),
                            cell_ref: diff.cell_ref.clone(),
                            col: diff.col,
                            header: None,
                            old_value: diff.old_value.clone(),
                            new_value: diff.new_value.clone(),
                            old_formula: diff.old_formula.clone(),
                            new_formula: diff.new_formula.clone(),
                        }),
                        DiffType::NoChange => {}
                    }
                }
            }
        }

        for diff in &sheet_diff.cell_diffs {
            if !matches!(diff.diff_type, DiffType::Add | DiffType::Delete) {
                match diff.diff_type {
                    DiffType::Modify => ops.push(LogicalOperation::CellModified {
                        sheet: sheet_diff.sheet_name.clone(),
                        cell_ref: diff.cell_ref.clone(),
                        col: diff.col,
                        header: None,
                        old_value: diff.old_value.clone(),
                        new_value: diff.new_value.clone(),
                        old_formula: diff.old_formula.clone(),
                        new_formula: diff.new_formula.clone(),
                    }),
                    DiffType::Passive => ops.push(LogicalOperation::CellPassive {
                        sheet: sheet_diff.sheet_name.clone(),
                        cell_ref: diff.cell_ref.clone(),
                        col: diff.col,
                        header: None,
                        old_value: diff.old_value.clone(),
                        new_value: diff.new_value.clone(),
                        old_formula: diff.old_formula.clone(),
                        new_formula: diff.new_formula.clone(),
                    }),
                    _ => {}
                }
            }
        }
    }

    ops
}

#[cfg(test)]
mod tests {
    use super::*;
    use excel_types::{CellDiff, DiffSummary, SheetDiff};

    fn cell_diff(
        row: u32,
        col: u16,
        cell_ref: &str,
        diff_type: DiffType,
        old_val: Option<&str>,
        new_val: Option<&str>,
    ) -> CellDiff {
        CellDiff {
            row,
            col,
            cell_ref: cell_ref.into(),
            diff_type,
            old_value: old_val.map(|s| s.into()),
            new_value: new_val.map(|s| s.into()),
            old_formula: None,
            new_formula: None,
        }
    }

    fn make_file_diff(sheet_diffs: Vec<SheetDiff>) -> FileDiff {
        FileDiff {
            file_hash_match: false,
            summary: DiffSummary {
                adds: 0,
                deletes: 0,
                modifies: 0,
                passives: 0,
                total_changes: 0,
            },
            sheet_diffs,
        }
    }

    fn sheet_diff(name: &str, cells: Vec<CellDiff>, row_delta: i32, col_delta: i32) -> SheetDiff {
        SheetDiff {
            sheet_name: name.into(),
            row_count_diff: row_delta,
            col_count_diff: col_delta,
            cell_diffs: cells,
        }
    }

    #[test]
    fn test_empty_diff_yields_no_ops() {
        let diff = make_file_diff(vec![]);
        assert!(group_diffs(&diff).is_empty());
    }

    #[test]
    fn test_identical_file_yields_no_ops() {
        let diff = make_file_diff(vec![sheet_diff("S", vec![], 0, 0)]);
        assert!(group_diffs(&diff).is_empty());
    }

    #[test]
    fn test_sheet_resized_detected() {
        let diff = make_file_diff(vec![sheet_diff("S", vec![], 5, 2)]);
        let ops = group_diffs(&diff);
        assert_eq!(ops.len(), 1);
        match &ops[0] {
            LogicalOperation::SheetResized {
                sheet,
                row_delta,
                col_delta,
            } => {
                assert_eq!(sheet, "S");
                assert_eq!(*row_delta, 5);
                assert_eq!(*col_delta, 2);
            }
            other => panic!("expected SheetResized, got {other:?}"),
        }
    }

    #[test]
    fn test_sheet_added_detected() {
        let cells = vec![
            cell_diff(0, 0, "A1", DiffType::Add, None, Some("Name")),
            cell_diff(0, 1, "B1", DiffType::Add, None, Some("Value")),
            cell_diff(1, 0, "A2", DiffType::Add, None, Some("Alice")),
            cell_diff(1, 1, "B2", DiffType::Add, None, Some("100")),
        ];
        let diff = make_file_diff(vec![sheet_diff("S", cells, 2, 2)]);
        let ops = group_diffs(&diff);
        assert_eq!(ops.len(), 1);
        match &ops[0] {
            LogicalOperation::SheetAdded {
                sheet,
                row_count,
                cell_count,
            } => {
                assert_eq!(sheet, "S");
                assert_eq!(*row_count, 2);
                assert_eq!(*cell_count, 4);
            }
            other => panic!("expected SheetAdded, got {other:?}"),
        }
    }

    #[test]
    fn test_sheet_deleted_detected() {
        let cells = vec![
            cell_diff(0, 0, "A1", DiffType::Delete, Some("X"), None),
            cell_diff(0, 1, "B1", DiffType::Delete, Some("Y"), None),
            cell_diff(1, 0, "A2", DiffType::Delete, Some("Z"), None),
            cell_diff(1, 1, "B2", DiffType::Delete, Some("W"), None),
        ];
        let diff = make_file_diff(vec![sheet_diff("S", cells, -2, -1)]);
        let ops = group_diffs(&diff);
        assert_eq!(ops.len(), 1);
        match &ops[0] {
            LogicalOperation::SheetDeleted {
                sheet,
                row_count,
                cell_count,
            } => {
                assert_eq!(sheet, "S");
                assert_eq!(*row_count, 2);
                assert_eq!(*cell_count, 4);
            }
            other => panic!("expected SheetDeleted, got {other:?}"),
        }
    }

    #[test]
    fn test_row_added_detected() {
        let cells = vec![
            cell_diff(2, 0, "A3", DiffType::Add, None, Some("Bob")),
            cell_diff(2, 1, "B3", DiffType::Add, None, Some("300")),
        ];
        let diff = make_file_diff(vec![sheet_diff("S", cells, 1, 0)]);
        let ops = group_diffs(&diff);
        assert_eq!(ops.len(), 1);
        match &ops[0] {
            LogicalOperation::RowAdded {
                sheet,
                row,
                entries,
            } => {
                assert_eq!(sheet, "S");
                assert_eq!(*row, 2);
                assert_eq!(entries.len(), 2);
            }
            other => panic!("expected RowAdded, got {other:?}"),
        }
    }

    #[test]
    fn test_row_deleted_detected() {
        let cells = vec![
            cell_diff(1, 0, "A2", DiffType::Delete, Some("Bob"), None),
            cell_diff(1, 1, "B2", DiffType::Delete, Some("200"), None),
        ];
        let diff = make_file_diff(vec![sheet_diff("S", cells, -1, 0)]);
        let ops = group_diffs(&diff);
        assert_eq!(ops.len(), 1);
        match &ops[0] {
            LogicalOperation::RowDeleted {
                sheet,
                row,
                entries,
            } => {
                assert_eq!(sheet, "S");
                assert_eq!(*row, 1);
                assert_eq!(entries.len(), 2);
            }
            other => panic!("expected RowDeleted, got {other:?}"),
        }
    }

    #[test]
    fn test_single_cell_add_is_individual() {
        let cells = vec![cell_diff(0, 2, "C1", DiffType::Add, None, Some("Extra"))];
        let diff = make_file_diff(vec![sheet_diff("S", cells, 0, 1)]);
        let ops = group_diffs(&diff);
        assert_eq!(ops.len(), 1);
        match &ops[0] {
            LogicalOperation::CellModified {
                cell_ref,
                new_value,
                ..
            } => {
                assert_eq!(cell_ref, "C1");
                assert_eq!(new_value.as_deref(), Some("Extra"));
            }
            other => panic!("expected CellModified, got {other:?}"),
        }
    }

    #[test]
    fn test_cell_modify_individual() {
        let cells = vec![cell_diff(
            1,
            1,
            "B2",
            DiffType::Modify,
            Some("100"),
            Some("200"),
        )];
        let diff = make_file_diff(vec![sheet_diff("S", cells, 0, 0)]);
        let ops = group_diffs(&diff);
        assert_eq!(ops.len(), 1);
        match &ops[0] {
            LogicalOperation::CellModified {
                cell_ref,
                old_value,
                new_value,
                ..
            } => {
                assert_eq!(cell_ref, "B2");
                assert_eq!(old_value.as_deref(), Some("100"));
                assert_eq!(new_value.as_deref(), Some("200"));
            }
            other => panic!("expected CellModified, got {other:?}"),
        }
    }

    #[test]
    fn test_cell_passive_individual() {
        let cells = vec![CellDiff {
            row: 2,
            col: 1,
            cell_ref: "B3".into(),
            diff_type: DiffType::Passive,
            old_value: Some("30".into()),
            new_value: Some("50".into()),
            old_formula: Some("=SUM(B1:B2)".into()),
            new_formula: Some("=SUM(B1:B2)".into()),
        }];
        let diff = make_file_diff(vec![sheet_diff("S", cells, 0, 0)]);
        let ops = group_diffs(&diff);
        assert_eq!(ops.len(), 1);
        match &ops[0] {
            LogicalOperation::CellPassive {
                cell_ref,
                old_formula,
                new_formula,
                old_value,
                new_value,
                ..
            } => {
                assert_eq!(cell_ref, "B3");
                assert_eq!(old_formula.as_deref(), Some("=SUM(B1:B2)"));
                assert_eq!(new_formula.as_deref(), Some("=SUM(B1:B2)"));
                assert_eq!(old_value.as_deref(), Some("30"));
                assert_eq!(new_value.as_deref(), Some("50"));
            }
            other => panic!("expected CellPassive, got {other:?}"),
        }
    }

    #[test]
    fn test_mixed_row_yields_individual_ops() {
        let cells = vec![
            cell_diff(0, 0, "A1", DiffType::Modify, Some("old"), Some("new")),
            cell_diff(0, 1, "B1", DiffType::Passive, Some("10"), Some("20")),
        ];
        let diff = make_file_diff(vec![sheet_diff("S", cells, 0, 0)]);
        let ops = group_diffs(&diff);
        assert_eq!(ops.len(), 2);
        assert!(matches!(ops[0], LogicalOperation::CellModified { .. }));
        assert!(matches!(ops[1], LogicalOperation::CellPassive { .. }));
    }

    #[test]
    fn test_single_cell_delete_is_individual() {
        let cells = vec![cell_diff(2, 0, "A3", DiffType::Delete, Some("gone"), None)];
        let diff = make_file_diff(vec![sheet_diff("S", cells, -1, 0)]);
        let ops = group_diffs(&diff);
        assert_eq!(ops.len(), 1);
        match &ops[0] {
            LogicalOperation::CellModified {
                cell_ref,
                old_value,
                new_value,
                ..
            } => {
                assert_eq!(cell_ref, "A3");
                assert_eq!(old_value.as_deref(), Some("gone"));
                assert!(new_value.is_none());
            }
            other => panic!("expected CellModified, got {other:?}"),
        }
    }
}
