pub mod context;
pub mod grouper;
pub mod natural;

use excel_core::types::FileDiff;

use self::context::HeaderContext;
use self::grouper::{LogicalOperation, group_diffs};
use self::natural::{format_operation, generate_summary_text};

/// Controls the level of detail in generated natural language text.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Verbosity {
    /// Only the one-line summary (number of changes by type).
    Summary,
    /// Summary + one sentence per logical operation.
    Normal,
    /// Full detail including individual cell refs and values.
    #[default]
    Detailed,
}

/// A semantic diff report containing both structured operations and natural language text.
#[derive(Debug, Clone)]
pub struct SemanticDiffReport {
    /// One-sentence summary (e.g. "5 changes: 2 new, 3 revised").
    pub summary: String,
    /// Structured logical operations derived from raw cell diffs.
    pub operations: Vec<LogicalOperation>,
    /// One natural language sentence per operation.
    pub detail_sentences: Vec<String>,
}

impl SemanticDiffReport {
    /// Returns the full natural language text by joining summary and detail sentences.
    /// The result is a single string with each sentence on a new line.
    pub fn to_natural_text(&self, verbosity: Verbosity) -> String {
        match verbosity {
            Verbosity::Summary => self.summary.clone(),
            Verbosity::Normal | Verbosity::Detailed => {
                let mut lines = vec![self.summary.clone()];
                lines.extend(self.detail_sentences.clone());
                lines.join("\n")
            }
        }
    }
}

/// Converts a `FileDiff` into a `SemanticDiffReport`.
///
/// * `file_diff` - The raw diff result from `diff_files()` or similar.
/// * `headers` - Optional column header context for semantically richer descriptions.
pub fn to_semantic_report(
    file_diff: &FileDiff,
    headers: Option<&HeaderContext>,
) -> SemanticDiffReport {
    let ops = group_diffs(file_diff);
    let ops_with_headers = enrich_headers(ops, headers, file_diff);
    let summary = generate_summary_text(&file_diff.summary);
    let detail_sentences: Vec<String> = ops_with_headers
        .iter()
        .map(|op| format_operation(op, headers))
        .collect();

    SemanticDiffReport {
        summary,
        operations: ops_with_headers,
        detail_sentences,
    }
}

/// A convenience wrapper: directly produce natural language text from a `FileDiff`.
///
/// Equivalent to `to_semantic_report(file_diff, headers).to_natural_text(verbosity)`.
pub fn to_natural_text(
    file_diff: &FileDiff,
    headers: Option<&HeaderContext>,
    verbosity: Verbosity,
) -> String {
    let report = to_semantic_report(file_diff, headers);
    report.to_natural_text(verbosity)
}

fn enrich_headers(
    ops: Vec<LogicalOperation>,
    headers: Option<&HeaderContext>,
    _file_diff: &FileDiff,
) -> Vec<LogicalOperation> {
    let Some(ctx) = headers else { return ops };

    ops.into_iter()
        .map(|op| match op {
            LogicalOperation::CellModified {
                sheet,
                cell_ref,
                col,
                header: _,
                old_value,
                new_value,
                old_formula,
                new_formula,
            } => {
                let h = ctx.get_header(&sheet, col).map(|s| s.to_string());
                LogicalOperation::CellModified {
                    sheet,
                    cell_ref,
                    col,
                    header: h,
                    old_value,
                    new_value,
                    old_formula,
                    new_formula,
                }
            }
            LogicalOperation::CellPassive {
                sheet,
                cell_ref,
                col,
                header: _,
                old_formula,
                new_formula,
                old_value,
                new_value,
            } => {
                let h = ctx.get_header(&sheet, col).map(|s| s.to_string());
                LogicalOperation::CellPassive {
                    sheet,
                    cell_ref,
                    col,
                    header: h,
                    old_formula,
                    new_formula,
                    old_value,
                    new_value,
                }
            }
            other => other,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use excel_core::types::{CellDiff, DiffSummary, DiffType, SheetDiff};

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
