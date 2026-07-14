use crate::excel_read::{list_sheets, read_sheet_all};
use crate::security::{list_history_entries};
use crate::types::{
    BlueprintSheet, ColumnSummary, DataFlow, Result, SheetOverview, WorkbookBlueprint,
    WorkbookOverview,
};
use crate::utils::cell_ref;

const COLUMN_SAMPLE_ROWS: usize = 100;

pub fn get_workbook_overview(path: &str) -> Result<WorkbookOverview> {
    let sheets = list_sheets(path)?;
    let mut sheet_overviews = Vec::new();
    let mut total_cells = 0usize;
    let mut formula_cells = 0usize;

    for sheet_name in &sheets {
        let data = read_sheet_all(path, sheet_name)?;
        let row_count = data.rows.len();
        let col_count = if row_count > 0 {
            data.rows.iter().map(|r| r.len()).max().unwrap_or(0)
        } else {
            0
        };

        let has_data = row_count > 0;
        let sheet_formula_count: usize = data
            .rows
            .iter()
            .map(|row| row.iter().filter(|c| c.formula.is_some()).count())
            .sum();
        total_cells += row_count * col_count;
        formula_cells += sheet_formula_count;

        let used_range = if row_count > 0 && col_count > 0 {
            let end_col = cell_ref::index_to_col((col_count - 1) as u16);
            format!("A1:{}{}", end_col, row_count)
        } else {
            "A1".to_string()
        };

        sheet_overviews.push(SheetOverview {
            name: sheet_name.clone(),
            used_range,
            row_count,
            col_count,
            has_formulas: sheet_formula_count > 0,
            has_data,
        });
    }

    Ok(WorkbookOverview {
        path: path.to_string(),
        sheets: sheet_overviews,
        named_ranges_count: 0,
        total_cells,
        formula_cells,
    })
}

pub fn get_sheet_overview(path: &str, sheet: &str) -> Result<SheetOverview> {
    let data = read_sheet_all(path, sheet)?;
    let row_count = data.rows.len();
    let col_count = if row_count > 0 {
        data.rows.iter().map(|r| r.len()).max().unwrap_or(0)
    } else {
        0
    };

    let has_data = row_count > 0;
    let sheet_formula_count: usize = data
        .rows
        .iter()
        .map(|row| row.iter().filter(|c| c.formula.is_some()).count())
        .sum();

    let used_range = if row_count > 0 && col_count > 0 {
        let end_col = cell_ref::index_to_col((col_count - 1) as u16);
        format!("A1:{}{}", end_col, row_count)
    } else {
        "A1".to_string()
    };

    Ok(SheetOverview {
        name: sheet.to_string(),
        used_range,
        row_count,
        col_count,
        has_formulas: sheet_formula_count > 0,
        has_data,
    })
}

pub fn get_column_summary(path: &str, sheet: &str, col: &str) -> Result<ColumnSummary> {
    let col_idx = cell_ref::col_to_index(col)?;
    let data = read_sheet_all(path, sheet)?;

    let sample_end = COLUMN_SAMPLE_ROWS.min(data.rows.len());
    let mut non_empty = 0usize;
    let mut string_count = 0usize;
    let mut number_count = 0usize;
    let mut date_count = 0usize;
    let mut first_value: Option<String> = None;

    for (row_idx, row) in data.rows.iter().enumerate() {
        if let Some(cell) = row.get(col_idx as usize) {
            if cell.value.is_some() {
                non_empty += 1;
                if first_value.is_none() {
                    first_value = cell.value.clone();
                }
                if row_idx < sample_end {
                    match cell.data_type {
                        crate::types::CellDataType::String => string_count += 1,
                        crate::types::CellDataType::Float
                        | crate::types::CellDataType::Int => number_count += 1,
                        crate::types::CellDataType::DateTime => date_count += 1,
                        _ => {}
                    }
                }
            }
        }
    }

    let total_typed = (sample_end.min(non_empty)).max(1);
    let string_ratio = string_count as f64 / total_typed as f64;
    let number_ratio = number_count as f64 / total_typed as f64;
    let date_ratio = date_count as f64 / total_typed as f64;

    let inferred_type = if number_ratio > 0.7 {
        "number"
    } else if date_ratio > 0.7 {
        "date"
    } else if string_ratio > 0.7 {
        "string"
    } else {
        "mixed"
    };

    Ok(ColumnSummary {
        column: col.to_string(),
        inferred_type: inferred_type.to_string(),
        non_empty_rows: non_empty,
        first_row_value: first_value,
    })
}

pub fn list_workbook_history(path: &str) -> Result<Vec<crate::types::WorkbookHistoryEntry>> {
    list_history_entries(path).map_err(crate::types::AppError::Io)
}

pub fn get_workbook_blueprint(path: &str) -> Result<WorkbookBlueprint> {
    let sheets = list_sheets(path)?;
    let mut blueprint_sheets = Vec::new();
    let mut data_flows = Vec::new();
    let mut key_cells = Vec::new();

    for sheet_name in &sheets {
        let data = read_sheet_all(path, sheet_name)?;
        let row_count = data.rows.len();
        let col_count = if row_count > 0 {
            data.rows.iter().map(|r| r.len()).max().unwrap_or(0)
        } else {
            0
        };

        // Identify formula cells for dependency analysis and key cells
        let mut sheet_formula_refs = Vec::new();
        for (row_idx, row) in data.rows.iter().enumerate() {
            for (col_idx, cell) in row.iter().enumerate() {
                if let Some(ref formula) = cell.formula {
                    let cell_ref =
                        cell_ref::format_cell_ref(row_idx as u32, col_idx as u16);
                    sheet_formula_refs.push((cell_ref, formula.clone()));
                }
            }
        }

        let formula_count_sheet = sheet_formula_refs.len();

        // Key cells: formula cells with high dependency potential
        // For now, mark cells with formulas containing cross-sheet references
        for (ref_, formula) in &sheet_formula_refs {
            // Check if formula references other sheets
            if let Some(sheet_ref) = extract_sheet_ref_from_formula(formula) {
                data_flows.push(DataFlow {
                    from: sheet_ref.to_string(),
                    to: format!("{}!{}", sheet_name, ref_),
                    via: "formula_reference".to_string(),
                });
            }
            // Mark as key cell if formula seems complex (has multiple references)
            if formula.matches(|c: char| c.is_ascii_uppercase()).count() > 2 {
                key_cells.push(format!("{}!{}", sheet_name, ref_));
            }
        }

        blueprint_sheets.push(BlueprintSheet {
            name: sheet_name.clone(),
            row_count,
            col_count,
            named_ranges: Vec::new(), // Named range parsing requires workbook.xml
            formula_count: formula_count_sheet,
        });
    }

    // Sort key cells for consistency
    key_cells.sort();

    Ok(WorkbookBlueprint {
        path: path.to_string(),
        sheets: blueprint_sheets,
        data_flows,
        key_cells,
    })
}

/// Extract a sheet name reference from a formula like "Sheet2!A1".
fn extract_sheet_ref_from_formula(formula: &str) -> Option<&str> {
    // Match patterns like SheetName!CellRef
    // Look for text before ! that starts with a letter
    let excl_pos = formula.find('!')?;
    let before = &formula[..excl_pos];
    // The sheet reference is the last identifier before !
    // It might contain spaces or be quoted, e.g., 'Sheet Name'!A1
    let sheet_ref = if before.starts_with('\'') {
        before.strip_prefix('\'')?.strip_suffix('\'')?
    } else {
        before
    };
    if sheet_ref.is_empty() || !sheet_ref.chars().next().unwrap_or(' ').is_alphabetic() {
        return None;
    }
    Some(sheet_ref)
}
