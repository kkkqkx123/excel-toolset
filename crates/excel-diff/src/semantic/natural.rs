use excel_types::DiffSummary;

use super::context::HeaderContext;
use super::grouper::LogicalOperation;

pub fn format_operation(op: &LogicalOperation, headers: Option<&HeaderContext>) -> String {
    match op {
        LogicalOperation::RowAdded {
            sheet,
            row,
            entries,
        } => {
            let parts: Vec<String> = entries
                .iter()
                .map(|e| {
                    let label = headers
                        .and_then(|h| h.get_header(sheet, e.col))
                        .unwrap_or(&e.label);
                    format!("{}={}", label, e.value)
                })
                .collect();
            format!("Add row {} in \"{}\": {}", row + 1, sheet, parts.join(", "))
        }

        LogicalOperation::RowDeleted {
            sheet,
            row,
            entries,
        } => {
            let parts: Vec<String> = entries
                .iter()
                .map(|e| {
                    let label = headers
                        .and_then(|h| h.get_header(sheet, e.col))
                        .unwrap_or(&e.label);
                    format!("{}={}", label, e.value)
                })
                .collect();
            format!(
                "Delete row {} in \"{}\": {}",
                row + 1,
                sheet,
                parts.join(", ")
            )
        }

        LogicalOperation::CellModified {
            sheet,
            cell_ref,
            header,
            old_value,
            new_value,
            old_formula,
            new_formula,
            ..
        } => {
            let header_str = header
                .as_deref()
                .filter(|h| !h.is_empty())
                .map(|h| format!(" ({})", h))
                .unwrap_or_default();
            let desc = if let (Some(old_f), Some(new_f)) = (old_formula, new_formula) {
                format!("formula changed from {} to {}", old_f, new_f)
            } else if let (Some(old_v), Some(new_v)) = (old_value, new_value) {
                format!("changed from {} to {}", old_v, new_v)
            } else if let (None, Some(new_v)) = (old_value, new_value) {
                let formula_info = new_formula
                    .as_ref()
                    .map(|f| format!(" (formula: {})", f))
                    .unwrap_or_default();
                format!("added as {}{}", new_v, formula_info)
            } else if let (Some(old_v), None) = (old_value, new_value) {
                let formula_info = old_formula
                    .as_ref()
                    .map(|f| format!(" (formula: {})", f))
                    .unwrap_or_default();
                format!("removed value {}{}", old_v, formula_info)
            } else if let (Some(f), _) = (new_formula, old_value) {
                format!("formula set to {}", f)
            } else if let (_, Some(_f)) = (old_formula, new_value) {
                format!("changed to {}", new_value.as_deref().unwrap_or(""))
            } else {
                String::new()
            };
            format!("Cell {}{} in \"{}\": {}", cell_ref, header_str, sheet, desc)
        }

        LogicalOperation::CellPassive {
            sheet,
            cell_ref,
            header,
            old_formula,
            new_formula,
            old_value,
            new_value,
            ..
        } => {
            let header_str = header
                .as_deref()
                .filter(|h| !h.is_empty())
                .map(|h| format!(" ({})", h))
                .unwrap_or_default();
            let old_v = old_value.as_deref().unwrap_or("empty");
            let new_v = new_value.as_deref().unwrap_or("empty");
            let formula_display = new_formula
                .as_deref()
                .or(old_formula.as_deref())
                .unwrap_or("");
            format!(
                "Cell {}{} in \"{}\" passively updated from {} to {} (formula: {})",
                cell_ref, header_str, sheet, old_v, new_v, formula_display,
            )
        }

        LogicalOperation::SheetAdded {
            sheet,
            row_count,
            cell_count,
        } => {
            format!(
                "Add a new worksheet \"{}\" with {} rows and {} cells.",
                sheet, row_count, cell_count
            )
        }

        LogicalOperation::SheetDeleted {
            sheet,
            row_count,
            cell_count,
        } => {
            format!(
                "Delete worksheet \"{}\" (original {} rows, {} cells)",
                sheet, row_count, cell_count
            )
        }

        LogicalOperation::SheetResized {
            sheet,
            row_delta,
            col_delta,
        } => {
            let mut parts: Vec<String> = Vec::new();
            if *row_delta != 0 {
                parts.push(format!(
                    "rows {}by {}",
                    if *row_delta > 0 {
                        "increased "
                    } else {
                        "decreased "
                    },
                    row_delta.abs()
                ));
            }
            if *col_delta != 0 {
                parts.push(format!(
                    "columns {}by {}",
                    if *col_delta > 0 {
                        "increased "
                    } else {
                        "decreased "
                    },
                    col_delta.abs()
                ));
            }
            format!("Sheet \"{}\" resized: {}", sheet, parts.join(", "))
        }
    }
}

pub fn generate_summary_text(summary: &DiffSummary) -> String {
    let mut parts = Vec::new();

    if summary.adds > 0 {
        parts.push(format!("{} added", summary.adds));
    }
    if summary.deletes > 0 {
        parts.push(format!("{} deleted", summary.deletes));
    }
    if summary.modifies > 0 {
        parts.push(format!("{} modified", summary.modifies));
    }
    if summary.passives > 0 {
        parts.push(format!("{} passive update", summary.passives));
    }

    if parts.is_empty() {
        return "No changes".into();
    }

    format!(
        "Total {} changes: {}",
        summary.total_changes,
        parts.join(", ")
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    use crate::semantic::context::HeaderContext;
    use crate::semantic::grouper::RowEntry;

    #[test]
    fn test_summary_empty() {
        let s = DiffSummary {
            adds: 0,
            deletes: 0,
            modifies: 0,
            passives: 0,
            total_changes: 0,
        };
        assert_eq!(generate_summary_text(&s), "No changes");
    }

    #[test]
    fn test_summary_with_changes() {
        let s = DiffSummary {
            adds: 2,
            deletes: 1,
            modifies: 3,
            passives: 1,
            total_changes: 7,
        };
        let text = generate_summary_text(&s);
        assert!(text.contains("7 changes"));
        assert!(text.contains("2 added"));
        assert!(text.contains("3 modified"));
    }

    #[test]
    fn test_format_row_added() {
        let op = LogicalOperation::RowAdded {
            sheet: "Sheet1".into(),
            row: 5,
            entries: vec![
                RowEntry {
                    col: 0,
                    label: "A6".into(),
                    value: "Charlie".into(),
                },
                RowEntry {
                    col: 1,
                    label: "B6".into(),
                    value: "300".into(),
                },
            ],
        };
        let text = format_operation(&op, None);
        assert_eq!(text, "Add row 6 in \"Sheet1\": A6=Charlie, B6=300");
    }

    #[test]
    fn test_format_row_added_with_headers() {
        let mut h = HashMap::new();
        h.insert("Sheet1".into(), vec!["Name".into(), "Value".into()]);
        let ctx = HeaderContext::new(h);

        let op = LogicalOperation::RowAdded {
            sheet: "Sheet1".into(),
            row: 5,
            entries: vec![
                RowEntry {
                    col: 0,
                    label: "A6".into(),
                    value: "Charlie".into(),
                },
                RowEntry {
                    col: 1,
                    label: "B6".into(),
                    value: "300".into(),
                },
            ],
        };
        let text = format_operation(&op, Some(&ctx));
        assert_eq!(text, "Add row 6 in \"Sheet1\": Name=Charlie, Value=300");
    }

    #[test]
    fn test_format_sheet_added() {
        let op = LogicalOperation::SheetAdded {
            sheet: "Sheet2".into(),
            row_count: 3,
            cell_count: 9,
        };
        let text = format_operation(&op, None);
        assert_eq!(
            text,
            "Add a new worksheet \"Sheet2\" with 3 rows and 9 cells."
        );
    }

    #[test]
    fn test_format_sheet_deleted() {
        let op = LogicalOperation::SheetDeleted {
            sheet: "Sheet2".into(),
            row_count: 2,
            cell_count: 4,
        };
        let text = format_operation(&op, None);
        assert_eq!(
            text,
            "Delete worksheet \"Sheet2\" (original 2 rows, 4 cells)"
        );
    }

    #[test]
    fn test_format_cell_passive() {
        let op = LogicalOperation::CellPassive {
            sheet: "Sheet1".into(),
            cell_ref: "C5".into(),
            col: 2,
            header: None,
            old_formula: Some("=SUM(C1:C4)".into()),
            new_formula: Some("=SUM(C1:C4)".into()),
            old_value: Some("100".into()),
            new_value: Some("150".into()),
        };
        let text = format_operation(&op, None);
        assert_eq!(
            text,
            "Cell C5 in \"Sheet1\" passively updated from 100 to 150 (formula: =SUM(C1:C4))"
        );
    }

    #[test]
    fn test_format_cell_passive_with_header() {
        use std::collections::HashMap;
        let mut h = HashMap::new();
        h.insert(
            "Sheet1".into(),
            vec!["Name".into(), "Salary".into(), "Total".into()],
        );
        let ctx = HeaderContext::new(h);

        let op = LogicalOperation::CellPassive {
            sheet: "Sheet1".into(),
            cell_ref: "C5".into(),
            col: 2,
            header: Some("Total".into()),
            old_formula: Some("=SUM(C1:C4)".into()),
            new_formula: Some("=SUM(C1:C4)".into()),
            old_value: Some("100".into()),
            new_value: Some("150".into()),
        };
        let text = format_operation(&op, Some(&ctx));
        assert_eq!(
            text,
            "Cell C5 (Total) in \"Sheet1\" passively updated from 100 to 150 (formula: =SUM(C1:C4))"
        );
    }

    #[test]
    fn test_format_sheet_resized() {
        let op = LogicalOperation::SheetResized {
            sheet: "Sheet1".into(),
            row_delta: 3,
            col_delta: -1,
        };
        let text = format_operation(&op, None);
        assert_eq!(
            text,
            "Sheet \"Sheet1\" resized: rows increased by 3, columns decreased by 1"
        );
    }

    #[test]
    fn test_format_cell_modified_value_change() {
        let op = LogicalOperation::CellModified {
            sheet: "Sheet1".into(),
            cell_ref: "B2".into(),
            col: 1,
            header: None,
            old_value: Some("100".into()),
            new_value: Some("200".into()),
            old_formula: None,
            new_formula: None,
        };
        let text = format_operation(&op, None);
        assert_eq!(text, "Cell B2 in \"Sheet1\": changed from 100 to 200");
    }

    #[test]
    fn test_format_cell_modified_formula_change() {
        let op = LogicalOperation::CellModified {
            sheet: "Sheet1".into(),
            cell_ref: "B3".into(),
            col: 1,
            header: None,
            old_value: Some("0".into()),
            new_value: Some("0".into()),
            old_formula: Some("=SUM(B1:B2)".into()),
            new_formula: Some("=AVERAGE(B1:B2)".into()),
        };
        let text = format_operation(&op, None);
        assert_eq!(
            text,
            "Cell B3 in \"Sheet1\": formula changed from =SUM(B1:B2) to =AVERAGE(B1:B2)"
        );
    }
}
