use std::collections::BTreeMap;

use excel_core::types::{CellDiff, DiffType, FileDiff};

/// A cell entry within a row-level operation.
#[derive(Debug, Clone)]
pub struct RowEntry {
    pub col: u16,
    pub label: String,
    pub value: String,
}

/// A semantically meaningful change operation derived from raw cell diffs.
#[derive(Debug, Clone)]
pub enum LogicalOperation {
    /// All cells in a row were added (new row).
    RowAdded {
        sheet: String,
        row: u32,
        entries: Vec<RowEntry>,
    },
    /// All cells in a row were removed (row deleted).
    RowDeleted {
        sheet: String,
        row: u32,
        entries: Vec<RowEntry>,
    },
    /// An individual cell was modified (value or formula changed).
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
    /// A formula cell whose value changed due to dependency updates (formula text unchanged).
    CellPassive {
        sheet: String,
        cell_ref: String,
        col: u16,
        header: Option<String>,
        old_formula: Option<String>,
        new_formula: Option<String>,
        old_value: Option<String>,
        new_value: Option<String>,
    },
    /// An entire sheet was added.
    SheetAdded {
        sheet: String,
        row_count: usize,
        cell_count: usize,
    },
    /// An entire sheet was deleted.
    SheetDeleted {
        sheet: String,
        row_count: usize,
        cell_count: usize,
    },
    /// Sheet structure changed (row/col count) but no cell-level diffs.
    SheetResized {
        sheet: String,
        row_delta: i32,
        col_delta: i32,
    },
}

fn is_all_same_type(cells: &[&CellDiff], ty: DiffType) -> bool {
    cells.iter().all(|c| c.diff_type == ty)
}

/// Groups raw CellDiffs into semantic operations.
///
/// Algorithm:
/// 1. Per-row: group cell diffs by row.
///    - If >=2 cells in a row are all Add → RowAdded
///    - If >=2 cells in a row are all Delete → RowDeleted
///    - Otherwise → individual CellModified / CellPassive
/// 2. Coalesce per-sheet: if all row ops are RowAdded (>=2 rows) → SheetAdded.
///    Same for RowDeleted → SheetDeleted.
/// 3. Empty diffs with row/col delta → SheetResized
pub fn group_diffs(file_diff: &FileDiff) -> Vec<LogicalOperation> {
    let mut ops = Vec::new();

    for sd in &file_diff.sheet_diffs {
        let sheet = &sd.sheet_name;

        if sd.cell_diffs.is_empty() {
            if sd.row_count_diff != 0 || sd.col_count_diff != 0 {
                ops.push(LogicalOperation::SheetResized {
                    sheet: sheet.clone(),
                    row_delta: sd.row_count_diff,
                    col_delta: sd.col_count_diff,
                });
            }
            continue;
        }

        // Group by row
        let mut by_row: BTreeMap<u32, Vec<&CellDiff>> = BTreeMap::new();
        for cd in &sd.cell_diffs {
            by_row.entry(cd.row).or_default().push(cd);
        }

        let mut row_ops: Vec<LogicalOperation> = Vec::new();

        for (row, cells) in &by_row {
            if cells.len() >= 2 && is_all_same_type(cells, DiffType::Add) {
                row_ops.push(LogicalOperation::RowAdded {
                    sheet: sheet.clone(),
                    row: *row,
                    entries: cells
                        .iter()
                        .map(|c| RowEntry {
                            col: c.col,
                            label: c.cell_ref.clone(),
                            value: c.new_value.clone().unwrap_or_default(),
                        })
                        .collect(),
                });
            } else if cells.len() >= 2 && is_all_same_type(cells, DiffType::Delete) {
                row_ops.push(LogicalOperation::RowDeleted {
                    sheet: sheet.clone(),
                    row: *row,
                    entries: cells
                        .iter()
                        .map(|c| RowEntry {
                            col: c.col,
                            label: c.cell_ref.clone(),
                            value: c.old_value.clone().unwrap_or_default(),
                        })
                        .collect(),
                });
            } else {
                for cd in cells {
                    match cd.diff_type {
                        DiffType::Modify => {
                            row_ops.push(LogicalOperation::CellModified {
                                sheet: sheet.clone(),
                                cell_ref: cd.cell_ref.clone(),
                                col: cd.col,
                                header: None,
                                old_value: cd.old_value.clone(),
                                new_value: cd.new_value.clone(),
                                old_formula: cd.old_formula.clone(),
                                new_formula: cd.new_formula.clone(),
                            });
                        }
                        DiffType::Passive => {
                            row_ops.push(LogicalOperation::CellPassive {
                                sheet: sheet.clone(),
                                cell_ref: cd.cell_ref.clone(),
                                col: cd.col,
                                header: None,
                                old_formula: cd.old_formula.clone(),
                                new_formula: cd.new_formula.clone(),
                                old_value: cd.old_value.clone(),
                                new_value: cd.new_value.clone(),
                            });
                        }
                        DiffType::Add => {
                            row_ops.push(LogicalOperation::CellModified {
                                sheet: sheet.clone(),
                                cell_ref: cd.cell_ref.clone(),
                                col: cd.col,
                                header: None,
                                old_value: None,
                                new_value: cd.new_value.clone(),
                                old_formula: None,
                                new_formula: cd.new_formula.clone(),
                            });
                        }
                        DiffType::Delete => {
                            row_ops.push(LogicalOperation::CellModified {
                                sheet: sheet.clone(),
                                cell_ref: cd.cell_ref.clone(),
                                col: cd.col,
                                header: None,
                                old_value: cd.old_value.clone(),
                                new_value: None,
                                old_formula: cd.old_formula.clone(),
                                new_formula: None,
                            });
                        }
                        _ => {}
                    }
                }
            }
        }

        // Coalesce per-sheet
        let all_row_add = !row_ops.is_empty()
            && row_ops
                .iter()
                .all(|op| matches!(op, LogicalOperation::RowAdded { .. }));
        let all_row_delete = !row_ops.is_empty()
            && row_ops
                .iter()
                .all(|op| matches!(op, LogicalOperation::RowDeleted { .. }));

        if all_row_add && row_ops.len() >= 2 {
            let total_cells: usize = row_ops
                .iter()
                .map(|op| match op {
                    LogicalOperation::RowAdded { entries, .. } => entries.len(),
                    _ => 0,
                })
                .sum();
            ops.push(LogicalOperation::SheetAdded {
                sheet: sheet.clone(),
                row_count: row_ops.len(),
                cell_count: total_cells,
            });
        } else if all_row_delete && row_ops.len() >= 2 {
            let total_cells: usize = row_ops
                .iter()
                .map(|op| match op {
                    LogicalOperation::RowDeleted { entries, .. } => entries.len(),
                    _ => 0,
                })
                .sum();
            ops.push(LogicalOperation::SheetDeleted {
                sheet: sheet.clone(),
                row_count: row_ops.len(),
                cell_count: total_cells,
            });
        } else {
            ops.extend(row_ops);
        }
    }

    ops
}

#[cfg(test)]
mod tests {
    use super::*;
    use excel_core::types::{CellDiff, DiffSummary, SheetDiff};

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
