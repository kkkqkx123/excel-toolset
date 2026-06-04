use std::collections::HashMap;

use rust_xlsxwriter::{Chart, Format, Workbook, Worksheet};

use crate::cell_ref;
use crate::excel_read::read_all_sheets_to_map;
use crate::security::{compute_file_hash, create_backup};
use crate::types::*;

use super::chart::map_chart_type;
use super::style::build_format;

/// Map an Excel error string to a formula that produces the same error.
/// rust_xlsxwriter has no public API for writing error cells directly,
/// so we use formula workarounds where possible and fall back to string otherwise.
fn error_to_formula(val: &str) -> Option<String> {
    match val {
        "#DIV/0!" => Some("1/0".to_string()),
        "#N/A" => Some("NA()".to_string()),
        "#NUM!" => Some("SQRT(-1)".to_string()),
        "#VALUE!" => Some("\"TEXT\"+1".to_string()),
        // #NAME?, #NULL!, #REF!, #GETTING_DATA — no reliable formula → fallback to string
        _ => None,
    }
}

pub(crate) fn modify_file<F>(
    path: &str,
    params: &SecurityParams,
    modifier: F,
) -> Result<WriteResult>
where
    F: FnOnce(&HashMap<String, SheetData>) -> Result<HashMap<String, SheetData>>,
{
    let old_hash = compute_file_hash(path).map_err(AppError::Io)?;

    let backup_info = if params.create_backup {
        Some(create_backup(path, &old_hash).map_err(AppError::Io)?)
    } else {
        None
    };

    let old_data = read_all_sheets_to_map(path)?;
    let new_data = modifier(&old_data)?;

    // Build workbook ONCE from the final modified data
    let mut wb = Workbook::new();
    for (name, data) in &new_data {
        let ws = wb.add_worksheet();
        ws.set_name(name).map_err(AppError::Xlsx)?;
        write_sheet_data(ws, data)?;
    }

    let new_hash = if params.dry_run {
        old_hash.clone()
    } else {
        wb.save(path).map_err(AppError::Xlsx)?;
        compute_file_hash(path).map_err(AppError::Io)?
    };

    Ok(WriteResult {
        success: true,
        message: String::new(),
        backup_info,
        old_hash,
        new_hash,
        diff: None,
    })
}

/// Variant of `modify_file` for operations that need direct workbook-level access
/// (e.g., set_format, merge_cells, add_chart). Pre-builds the workbook from old data
/// and passes it to the modifier. The modifier returns () and mutates the workbook in place.
pub(crate) fn modify_file_with_wb<F>(
    path: &str,
    params: &SecurityParams,
    modifier: F,
) -> Result<WriteResult>
where
    F: FnOnce(&HashMap<String, SheetData>, &mut Workbook) -> Result<()>,
{
    let old_hash = compute_file_hash(path).map_err(AppError::Io)?;

    let backup_info = if params.create_backup {
        Some(create_backup(path, &old_hash).map_err(AppError::Io)?)
    } else {
        None
    };

    let old_data = read_all_sheets_to_map(path)?;
    let mut wb = Workbook::new();

    for (name, data) in &old_data {
        let ws = wb.add_worksheet();
        ws.set_name(name).map_err(AppError::Xlsx)?;
        write_sheet_data(ws, data)?;
    }

    modifier(&old_data, &mut wb)?;

    let new_hash = if params.dry_run {
        old_hash.clone()
    } else {
        wb.save(path).map_err(AppError::Xlsx)?;
        compute_file_hash(path).map_err(AppError::Io)?
    };

    Ok(WriteResult {
        success: true,
        message: String::new(),
        backup_info,
        old_hash,
        new_hash,
        diff: None,
    })
}

pub(crate) fn write_sheet_data(ws: &mut Worksheet, data: &SheetData) -> Result<()> {
    for (ri, row) in data.rows.iter().enumerate() {
        for (ci, cell) in row.iter().enumerate() {
            write_cell_data(ws, ri as u32, ci as u16, cell)?;
        }
    }
    Ok(())
}

pub(crate) fn write_cell_data(
    ws: &mut Worksheet,
    row: u32,
    col: u16,
    cell: &CellData,
) -> Result<()> {
    if let Some(ref formula) = cell.formula {
        ws.write_formula(row, col, formula.as_str())
            .map_err(AppError::Xlsx)?;
        return Ok(());
    }
    if let Some(ref val) = cell.value {
        match cell.data_type {
            CellDataType::Float | CellDataType::Int | CellDataType::DateTime => {
                if let Ok(n) = val.parse::<f64>() {
                    ws.write_number(row, col, n).map_err(AppError::Xlsx)?;
                } else {
                    ws.write_string(row, col, val).map_err(AppError::Xlsx)?;
                }
            }
            CellDataType::Bool => {
                let b = val == "true" || val == "1" || val == "True";
                ws.write_boolean(row, col, b).map_err(AppError::Xlsx)?;
            }
            CellDataType::Error => {
                // Write error values as formulas where possible; fall back to string.
                if let Some(formula) = error_to_formula(val) {
                    ws.write_formula(row, col, formula.as_str())
                        .map_err(AppError::Xlsx)?;
                } else {
                    ws.write_string(row, col, val).map_err(AppError::Xlsx)?;
                }
            }
            _ => {
                ws.write_string(row, col, val).map_err(AppError::Xlsx)?;
            }
        }
    } else {
        ws.write_blank(row, col, &Format::new())
            .map_err(AppError::Xlsx)?;
    }
    Ok(())
}

pub(crate) fn write_cell_with_format(
    ws: &mut Worksheet,
    row: u32,
    col: u16,
    cell: &CellData,
    fmt: &Format,
) -> Result<()> {
    if let Some(ref val) = cell.value {
        match cell.data_type {
            CellDataType::Float | CellDataType::Int | CellDataType::DateTime => {
                if let Ok(n) = val.parse::<f64>() {
                    ws.write_number_with_format(row, col, n, fmt)
                        .map_err(AppError::Xlsx)?;
                } else {
                    ws.write_string_with_format(row, col, val, fmt)
                        .map_err(AppError::Xlsx)?;
                }
            }
            CellDataType::Error => {
                ws.write_string_with_format(row, col, val, fmt)
                    .map_err(AppError::Xlsx)?;
            }
            _ => {
                ws.write_string_with_format(row, col, val, fmt)
                    .map_err(AppError::Xlsx)?;
            }
        }
    }
    Ok(())
}

pub(crate) fn build_workbook_with_ops(
    data: &HashMap<String, SheetData>,
    operations: &[BatchOperation],
) -> Result<Workbook> {
    let mut wb = Workbook::new();
    let sheet_names: Vec<&str> = data.keys().map(|s| s.as_str()).collect();

    for name in &sheet_names {
        let sd = &data[*name];
        let ws = wb.add_worksheet();
        ws.set_name(*name).map_err(AppError::Xlsx)?;

        let formats: Vec<(&str, &Style)> = operations
            .iter()
            .filter_map(|op| {
                if let BatchOperation::SetFormat {
                    sheet,
                    range,
                    style,
                } = op
                {
                    if sheet == *name {
                        Some((range.as_str(), style))
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect();

        let merges: Vec<(&str, &Option<String>)> = operations
            .iter()
            .filter_map(|op| {
                if let BatchOperation::MergeCells {
                    sheet,
                    range,
                    value,
                } = op
                {
                    if sheet == *name {
                        Some((range.as_str(), value))
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect();

        if formats.is_empty() && merges.is_empty() {
            write_sheet_data(ws, sd)?;
        } else {
            for (ri, row) in sd.rows.iter().enumerate() {
                for (ci, cell) in row.iter().enumerate() {
                    let mut applied_format = false;
                    for (range_str, style) in &formats {
                        if let Ok((fr, fc)) = cell_ref::parse_cell_ref(range_str)
                            && ri as u32 == fr
                            && ci as u16 == fc
                        {
                            let fmt = build_format(style);
                            write_cell_with_format(ws, ri as u32, ci as u16, cell, &fmt)?;
                            applied_format = true;
                            break;
                        }
                    }
                    if !applied_format {
                        write_cell_data(ws, ri as u32, ci as u16, cell)?;
                    }
                }
            }
            for (range_str, value) in &merges {
                if let Ok((r1, c1, r2, c2)) = cell_ref::parse_range(range_str) {
                    ws.merge_range(
                        r1,
                        c1,
                        r2,
                        c2,
                        value.as_deref().unwrap_or(""),
                        &Format::new(),
                    )
                    .map_err(AppError::Xlsx)?;
                }
            }
        }
    }

    for op in operations {
        if let BatchOperation::AddChart { config } = op {
            let sheet_idx = sheet_names
                .iter()
                .position(|n| *n == config.sheet)
                .ok_or_else(|| AppError::SheetNotFound(config.sheet.clone()))?;
            if let Ok(ws) = wb.worksheet_from_index(sheet_idx) {
                let mut chart = Chart::new(map_chart_type(&config.chart_type));
                chart
                    .add_series()
                    .set_categories(config.categories_range.as_str())
                    .set_values(config.values_range.as_str());
                if let Some(ref title) = config.title {
                    chart.title().set_name(title);
                }
                ws.insert_chart(config.row, config.col, &chart)
                    .map_err(AppError::Xlsx)?;
            }
        }
    }

    Ok(wb)
}
