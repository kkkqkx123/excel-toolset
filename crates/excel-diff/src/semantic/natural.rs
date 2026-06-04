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
