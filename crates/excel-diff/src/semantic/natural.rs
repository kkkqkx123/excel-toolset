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
                        .and_then(|h| h.get_header(sheet, e.col as usize))
                        .unwrap_or(&e.cell_ref);
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
                        .and_then(|h| h.get_header(sheet, e.col as usize))
                        .unwrap_or(&e.cell_ref);
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
            old_value,
            new_value,
            ..
        } => {
            let header_str = header
                .as_deref()
                .filter(|h| !h.is_empty())
                .map(|h| format!(" ({})", h))
                .unwrap_or_default();
            let desc = format!(
                "value {} recalculated, formula {} unchanged",
                format_value_change(old_value, new_value),
                old_formula.as_deref().unwrap_or("")
            );
            format!("Cell {}{} in \"{}\": {}", cell_ref, header_str, sheet, desc)
        }

        LogicalOperation::SheetResized {
            sheet,
            row_delta,
            col_delta,
        } => {
            let mut changes = Vec::new();
            if *row_delta != 0 {
                changes.push(format!(
                    "{} {} rows",
                    row_delta.abs(),
                    if *row_delta > 0 { "added" } else { "removed" }
                ));
            }
            if *col_delta != 0 {
                changes.push(format!(
                    "{} {} columns",
                    col_delta.abs(),
                    if *col_delta > 0 { "added" } else { "removed" }
                ));
            }
            format!("Sheet \"{}\" resized: {}", sheet, changes.join(", "))
        }

        LogicalOperation::SheetAdded {
            sheet,
            row_count,
            cell_count,
        } => {
            format!(
                "Sheet \"{}\" added with {} rows and {} cells",
                sheet, row_count, cell_count
            )
        }

        LogicalOperation::SheetDeleted {
            sheet,
            row_count,
            cell_count,
        } => {
            format!(
                "Sheet \"{}\" deleted ({} rows, {} cells)",
                sheet, row_count, cell_count
            )
        }
    }
}

fn format_value_change(old: &Option<String>, new: &Option<String>) -> String {
    match (old, new) {
        (Some(o), Some(n)) => format!("{} -> {}", o, n),
        (Some(o), None) => format!("{} removed", o),
        (None, Some(n)) => format!("{} added", n),
        (None, None) => "no change".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::semantic::context::HeaderContext;
    use crate::semantic::grouper::{CellEntry, LogicalOperation};
    use std::collections::HashMap;

    fn row_added_op(sheet: &str, row: u32, entries: Vec<(&str, u16, &str)>) -> LogicalOperation {
        LogicalOperation::RowAdded {
            sheet: sheet.into(),
            row,
            entries: entries
                .into_iter()
                .map(|(ref_str, col, val)| CellEntry {
                    cell_ref: ref_str.into(),
                    col,
                    value: val.into(),
                })
                .collect(),
        }
    }

    fn row_deleted_op(sheet: &str, row: u32, entries: Vec<(&str, u16, &str)>) -> LogicalOperation {
        LogicalOperation::RowDeleted {
            sheet: sheet.into(),
            row,
            entries: entries
                .into_iter()
                .map(|(ref_str, col, val)| CellEntry {
                    cell_ref: ref_str.into(),
                    col,
                    value: val.into(),
                })
                .collect(),
        }
    }

    fn cell_modified_op(
        sheet: &str,
        cell_ref: &str,
        col: u16,
        header: Option<&str>,
        old_val: Option<&str>,
        new_val: Option<&str>,
        old_f: Option<&str>,
        new_f: Option<&str>,
    ) -> LogicalOperation {
        LogicalOperation::CellModified {
            sheet: sheet.into(),
            cell_ref: cell_ref.into(),
            col,
            header: header.map(|s| s.into()),
            old_value: old_val.map(|s| s.into()),
            new_value: new_val.map(|s| s.into()),
            old_formula: old_f.map(|s| s.into()),
            new_formula: new_f.map(|s| s.into()),
        }
    }

    fn cell_passive_op(
        sheet: &str,
        cell_ref: &str,
        col: u16,
        header: Option<&str>,
        old_val: Option<&str>,
        new_val: Option<&str>,
        old_f: Option<&str>,
    ) -> LogicalOperation {
        LogicalOperation::CellPassive {
            sheet: sheet.into(),
            cell_ref: cell_ref.into(),
            col,
            header: header.map(|s| s.into()),
            old_value: old_val.map(|s| s.into()),
            new_value: new_val.map(|s| s.into()),
            old_formula: old_f.map(|s| s.into()),
            new_formula: old_f.map(|s| s.into()),
        }
    }

    // ── RowAdded ──

    #[test]
    fn test_format_row_added() {
        let op = row_added_op("Sheet1", 2, vec![("A3", 0, "Bob"), ("B3", 1, "300")]);
        let s = format_operation(&op, None);
        assert_eq!(s, "Add row 3 in \"Sheet1\": A3=Bob, B3=300");
    }

    #[test]
    fn test_format_row_added_with_headers() {
        let op = row_added_op("Sheet1", 2, vec![("A3", 0, "Bob"), ("B3", 1, "300")]);
        let mut h = HashMap::new();
        h.insert("Sheet1".into(), vec!["Name".into(), "Score".into()]);
        let ctx = HeaderContext::new(h);
        let s = format_operation(&op, Some(&ctx));
        assert_eq!(s, "Add row 3 in \"Sheet1\": Name=Bob, Score=300");
    }

    // ── RowDeleted ──

    #[test]
    fn test_format_row_deleted() {
        let op = row_deleted_op("Sheet1", 1, vec![("A2", 0, "Alice"), ("B2", 1, "200")]);
        let s = format_operation(&op, None);
        assert_eq!(s, "Delete row 2 in \"Sheet1\": A2=Alice, B2=200");
    }

    #[test]
    fn test_format_row_deleted_with_headers() {
        let op = row_deleted_op("Sheet1", 1, vec![("A2", 0, "Alice")]);
        let mut h = HashMap::new();
        h.insert("Sheet1".into(), vec!["Name".into()]);
        let ctx = HeaderContext::new(h);
        let s = format_operation(&op, Some(&ctx));
        assert_eq!(s, "Delete row 2 in \"Sheet1\": Name=Alice");
    }

    // ── CellModified ──

    #[test]
    fn test_format_cell_modified_value_change() {
        let op = cell_modified_op("S", "B2", 1, None, Some("100"), Some("200"), None, None);
        let s = format_operation(&op, None);
        assert!(s.contains("Cell B2"));
        assert!(s.contains("changed from 100 to 200"));
    }

    #[test]
    fn test_format_cell_modified_formula_change() {
        let op = cell_modified_op(
            "S",
            "C5",
            2,
            None,
            Some("10"),
            Some("20"),
            Some("=A1+1"),
            Some("=B1+1"),
        );
        let s = format_operation(&op, None);
        assert!(s.contains("formula changed from =A1+1 to =B1+1"));
    }

    #[test]
    fn test_format_cell_modified_new_cell() {
        let op = cell_modified_op(
            "S",
            "D1",
            3,
            None,
            None,
            Some("hello"),
            None,
            Some("=NOW()"),
        );
        let s = format_operation(&op, None);
        assert!(s.contains("added as hello"));
        assert!(s.contains("formula: =NOW()"));
    }

    #[test]
    fn test_format_cell_modified_removed_cell() {
        let op = cell_modified_op("S", "A1", 0, None, Some("old"), None, Some("=1+1"), None);
        let s = format_operation(&op, None);
        assert!(s.contains("removed value old"));
        assert!(s.contains("formula: =1+1"));
    }

    #[test]
    fn test_format_cell_modified_with_header() {
        let op = cell_modified_op(
            "S",
            "B2",
            1,
            Some("Revenue"),
            Some("100"),
            Some("200"),
            None,
            None,
        );
        let s = format_operation(&op, None);
        assert!(s.contains("(Revenue)"));
        assert!(s.contains("B2"));
    }

    #[test]
    fn test_format_cell_modified_empty_header_not_shown() {
        let op = cell_modified_op("S", "A1", 0, Some(""), Some("x"), Some("y"), None, None);
        let s = format_operation(&op, None);
        assert!(!s.contains("()"));
    }

    // ── CellPassive ──

    #[test]
    fn test_format_cell_passive() {
        let op = cell_passive_op(
            "S",
            "B3",
            1,
            None,
            Some("30"),
            Some("50"),
            Some("=SUM(B1:B2)"),
        );
        let s = format_operation(&op, None);
        assert!(s.contains("30 -> 50"));
        assert!(s.contains("=SUM(B1:B2)"));
        assert!(s.contains("recalculated"));
    }

    #[test]
    fn test_format_cell_passive_with_header() {
        let op = cell_passive_op(
            "S",
            "B3",
            1,
            Some("Total"),
            Some("30"),
            Some("50"),
            Some("=SUM(B1:B2)"),
        );
        let s = format_operation(&op, None);
        assert!(s.contains("(Total)"));
    }

    // ── SheetResized ──

    #[test]
    fn test_format_sheet_resized_rows() {
        let op = LogicalOperation::SheetResized {
            sheet: "Sheet1".into(),
            row_delta: 3,
            col_delta: 0,
        };
        let s = format_operation(&op, None);
        assert_eq!(s, "Sheet \"Sheet1\" resized: 3 added rows");
    }

    #[test]
    fn test_format_sheet_resized_columns() {
        let op = LogicalOperation::SheetResized {
            sheet: "Sheet1".into(),
            row_delta: 0,
            col_delta: -2,
        };
        let s = format_operation(&op, None);
        assert_eq!(s, "Sheet \"Sheet1\" resized: 2 removed columns");
    }

    #[test]
    fn test_format_sheet_resized_both() {
        let op = LogicalOperation::SheetResized {
            sheet: "Sheet1".into(),
            row_delta: 5,
            col_delta: 1,
        };
        let s = format_operation(&op, None);
        assert_eq!(s, "Sheet \"Sheet1\" resized: 5 added rows, 1 added columns");
    }

    // ── SheetAdded ──

    #[test]
    fn test_format_sheet_added() {
        let op = LogicalOperation::SheetAdded {
            sheet: "NewSheet".into(),
            row_count: 3,
            cell_count: 9,
        };
        let s = format_operation(&op, None);
        assert_eq!(s, "Sheet \"NewSheet\" added with 3 rows and 9 cells");
    }

    // ── SheetDeleted ──

    #[test]
    fn test_format_sheet_deleted() {
        let op = LogicalOperation::SheetDeleted {
            sheet: "OldSheet".into(),
            row_count: 2,
            cell_count: 6,
        };
        let s = format_operation(&op, None);
        assert_eq!(s, "Sheet \"OldSheet\" deleted (2 rows, 6 cells)");
    }

    // ── format_value_change ──

    #[test]
    fn test_format_value_change_both_some() {
        assert_eq!(
            format_value_change(&Some("10".into()), &Some("20".into())),
            "10 -> 20"
        );
    }

    #[test]
    fn test_format_value_change_removed() {
        assert_eq!(format_value_change(&Some("10".into()), &None), "10 removed");
    }

    #[test]
    fn test_format_value_change_added() {
        assert_eq!(format_value_change(&None, &Some("20".into())), "20 added");
    }

    #[test]
    fn test_format_value_change_none() {
        assert_eq!(format_value_change(&None, &None), "no change");
    }
}
