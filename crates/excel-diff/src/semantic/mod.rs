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
        // Should only contain summary, no detail lines
        assert_eq!(text, "Total 1 changes: 1 modified");
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
}
