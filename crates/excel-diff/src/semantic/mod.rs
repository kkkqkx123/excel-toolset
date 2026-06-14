pub mod context;
pub mod grouper;
pub mod natural;

use excel_types::FileDiff;

use crate::semantic::grouper::{GroupedDiffs, LogicalOperation, group_diffs};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verbosity {
    Summary,
    Detail,
}

#[derive(Debug, Clone)]
pub struct SemanticReport {
    pub summary: String,
    pub operations: GroupedDiffs,
    pub detail_sentences: Vec<String>,
}

pub fn to_natural_text(
    diff: &FileDiff,
    _headers: Option<&crate::semantic::context::HeaderContext>,
    verbosity: Verbosity,
) -> String {
    let ops = group_diffs(diff);

    if ops.is_empty() {
        return "No changes".to_string();
    }

    match verbosity {
        Verbosity::Summary => {
            let mut parts = Vec::new();
            if diff.summary.adds > 0 {
                parts.push(format!("{} added", diff.summary.adds));
            }
            if diff.summary.deletes > 0 {
                parts.push(format!("{} deleted", diff.summary.deletes));
            }
            if diff.summary.modifies > 0 {
                parts.push(format!("{} modified", diff.summary.modifies));
            }
            if diff.summary.passives > 0 {
                parts.push(format!("{} passive", diff.summary.passives));
            }
            let detail = if parts.is_empty() {
                "no changes".to_string()
            } else {
                parts.join(", ")
            };
            format!("Total {} changes: {}", diff.summary.total_changes, detail)
        }
        Verbosity::Detail => {
            let mut text = format!(
                "Total {} changes: {} added, {} deleted, {} modified, {} passive",
                diff.summary.total_changes,
                diff.summary.adds,
                diff.summary.deletes,
                diff.summary.modifies,
                diff.summary.passives
            );

            for op in &ops {
                text.push_str(&format!("\n{}", natural::format_operation(op, None)));
            }

            text
        }
    }
}

pub fn to_semantic_report(
    diff: &FileDiff,
    _headers: Option<&crate::semantic::context::HeaderContext>,
) -> SemanticReport {
    let ops = group_diffs(diff);

    if ops.is_empty() {
        return SemanticReport {
            summary: "No changes".to_string(),
            operations: vec![],
            detail_sentences: vec![],
        };
    }

    let detail_sentences = ops
        .iter()
        .map(|op| natural::format_operation(op, None))
        .collect();

    SemanticReport {
        summary: format!("Total {} changes", diff.summary.total_changes),
        operations: ops,
        detail_sentences,
    }
}

pub fn enrich_headers(
    mut ops: GroupedDiffs,
    headers: Option<&crate::semantic::context::HeaderContext>,
    _diff: &FileDiff,
) -> GroupedDiffs {
    if let Some(ctx) = headers {
        for op in &mut ops {
            match op {
                LogicalOperation::CellModified {
                    sheet, col, header, ..
                }
                | LogicalOperation::CellPassive {
                    sheet, col, header, ..
                } => {
                    *header = ctx.get_header(sheet, *col as usize).map(|s| s.to_string());
                }
                _ => {}
            }
        }
    }

    ops
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::semantic::context::HeaderContext;
    use excel_types::{CellDiff, DiffSummary, DiffType, SheetDiff};

    fn make_diff(sheet_diffs: Vec<SheetDiff>, modifies: usize) -> FileDiff {
        FileDiff {
            file_hash_match: false,
            sheet_diffs,
            summary: DiffSummary {
                adds: 0,
                deletes: 0,
                modifies,
                passives: 0,
                total_changes: modifies,
            },
        }
    }

    fn make_diff_full(
        sheet_diffs: Vec<SheetDiff>,
        adds: usize,
        deletes: usize,
        modifies: usize,
        passives: usize,
    ) -> FileDiff {
        let total = adds + deletes + modifies + passives;
        FileDiff {
            file_hash_match: false,
            sheet_diffs,
            summary: DiffSummary {
                adds,
                deletes,
                modifies,
                passives,
                total_changes: total,
            },
        }
    }

    #[test]
    fn test_to_natural_text_empty() {
        let diff = make_diff(vec![], 0);
        let text = to_natural_text(&diff, None, Verbosity::Summary);
        assert_eq!(text, "No changes");
    }

    #[test]
    fn test_to_natural_text_verbosity_summary() {
        let diff = make_diff(
            vec![SheetDiff {
                sheet_name: "S".into(),
                row_count_diff: 0,
                col_count_diff: 0,
                cell_diffs: vec![CellDiff {
                    row: 0,
                    col: 0,
                    cell_ref: "A1".into(),
                    diff_type: DiffType::Modify,
                    old_value: Some("old".into()),
                    new_value: Some("new".into()),
                    old_formula: None,
                    new_formula: None,
                }],
            }],
            1,
        );
        let text = to_natural_text(&diff, None, Verbosity::Summary);
        assert_eq!(text, "Total 1 changes: 1 modified");
    }

    #[test]
    fn test_to_natural_text_verbosity_detail() {
        let diff = make_diff(
            vec![SheetDiff {
                sheet_name: "S".into(),
                row_count_diff: 0,
                col_count_diff: 0,
                cell_diffs: vec![CellDiff {
                    row: 0,
                    col: 0,
                    cell_ref: "A1".into(),
                    diff_type: DiffType::Modify,
                    old_value: Some("old".into()),
                    new_value: Some("new".into()),
                    old_formula: None,
                    new_formula: None,
                }],
            }],
            1,
        );
        let text = to_natural_text(&diff, None, Verbosity::Detail);
        assert!(text.contains("Total 1 changes"));
        assert!(text.contains("Cell A1"));
        assert!(text.contains("changed from old to new"));
    }

    #[test]
    fn test_to_natural_text_summary_mixed_types() {
        let sheet_diff = SheetDiff {
            sheet_name: "S".into(),
            row_count_diff: 0,
            col_count_diff: 0,
            cell_diffs: vec![
                CellDiff {
                    row: 0,
                    col: 0,
                    cell_ref: "A1".into(),
                    diff_type: DiffType::Add,
                    old_value: None,
                    new_value: Some("x".into()),
                    old_formula: None,
                    new_formula: None,
                },
                CellDiff {
                    row: 0,
                    col: 1,
                    cell_ref: "B1".into(),
                    diff_type: DiffType::Add,
                    old_value: None,
                    new_value: Some("y".into()),
                    old_formula: None,
                    new_formula: None,
                },
                CellDiff {
                    row: 1,
                    col: 0,
                    cell_ref: "A2".into(),
                    diff_type: DiffType::Delete,
                    old_value: Some("z".into()),
                    new_value: None,
                    old_formula: None,
                    new_formula: None,
                },
                CellDiff {
                    row: 2,
                    col: 0,
                    cell_ref: "A3".into(),
                    diff_type: DiffType::Modify,
                    old_value: Some("a".into()),
                    new_value: Some("b".into()),
                    old_formula: None,
                    new_formula: None,
                },
                CellDiff {
                    row: 2,
                    col: 1,
                    cell_ref: "B3".into(),
                    diff_type: DiffType::Modify,
                    old_value: Some("c".into()),
                    new_value: Some("d".into()),
                    old_formula: None,
                    new_formula: None,
                },
                CellDiff {
                    row: 2,
                    col: 2,
                    cell_ref: "C3".into(),
                    diff_type: DiffType::Modify,
                    old_value: Some("e".into()),
                    new_value: Some("f".into()),
                    old_formula: None,
                    new_formula: None,
                },
                CellDiff {
                    row: 3,
                    col: 0,
                    cell_ref: "A4".into(),
                    diff_type: DiffType::Passive,
                    old_value: Some("10".into()),
                    new_value: Some("20".into()),
                    old_formula: Some("=B1+1".into()),
                    new_formula: Some("=B1+1".into()),
                },
            ],
        };
        let diff = make_diff_full(vec![sheet_diff], 2, 1, 3, 1);
        let text = to_natural_text(&diff, None, Verbosity::Summary);
        assert_eq!(
            text,
            "Total 7 changes: 2 added, 1 deleted, 3 modified, 1 passive"
        );
    }

    #[test]
    fn test_to_natural_text_summary_no_changes_with_summary() {
        let diff = FileDiff {
            file_hash_match: true,
            sheet_diffs: vec![],
            summary: DiffSummary {
                adds: 0,
                deletes: 0,
                modifies: 0,
                passives: 0,
                total_changes: 0,
            },
        };
        let text = to_natural_text(&diff, None, Verbosity::Summary);
        assert_eq!(text, "No changes");
    }

    #[test]
    fn test_semantic_report_roundtrip() {
        let diff = make_diff(vec![], 0);
        let report = to_semantic_report(&diff, None);
        assert_eq!(report.summary, "No changes");
        assert!(report.operations.is_empty());
        assert!(report.detail_sentences.is_empty());
    }

    #[test]
    fn test_semantic_report_with_diffs() {
        let diff = make_diff(
            vec![SheetDiff {
                sheet_name: "S".into(),
                row_count_diff: 0,
                col_count_diff: 0,
                cell_diffs: vec![CellDiff {
                    row: 0,
                    col: 0,
                    cell_ref: "A1".into(),
                    diff_type: DiffType::Modify,
                    old_value: Some("old".into()),
                    new_value: Some("new".into()),
                    old_formula: None,
                    new_formula: None,
                }],
            }],
            1,
        );
        let report = to_semantic_report(&diff, None);
        assert_eq!(report.summary, "Total 1 changes");
        assert_eq!(report.detail_sentences.len(), 1);
    }

    #[test]
    fn test_enrich_headers_sets_header_on_cell_ops() {
        let ops = vec![LogicalOperation::CellModified {
            sheet: "S".into(),
            cell_ref: "B2".into(),
            col: 1,
            header: None,
            old_value: Some("100".into()),
            new_value: Some("200".into()),
            old_formula: None,
            new_formula: None,
        }];

        use std::collections::HashMap;
        let mut h = HashMap::new();
        h.insert("S".into(), vec!["A".into(), "B".into()]);
        let ctx = HeaderContext::new(h);

        let enriched = enrich_headers(ops, Some(&ctx), &make_diff(vec![], 0));
        match &enriched[0] {
            LogicalOperation::CellModified { header, .. } => {
                assert_eq!(header.as_deref(), Some("B"));
            }
            _ => panic!("expected CellModified"),
        }
    }

    #[test]
    fn test_enrich_headers_skips_non_cell_ops() {
        let ops = vec![LogicalOperation::SheetResized {
            sheet: "S".into(),
            row_delta: 1,
            col_delta: 0,
        }];
        let enriched = enrich_headers(ops, None, &make_diff(vec![], 0));
        assert_eq!(enriched.len(), 1);
        assert!(matches!(enriched[0], LogicalOperation::SheetResized { .. }));
    }
}
