use rust_xlsxwriter::{Chart, Format, Workbook, Worksheet};

use crate::excel_read::read_all_sheets_to_map;
use crate::security::{compute_file_hash, create_backup};
use crate::types::*;
use crate::utils::cell_ref;

use super::format::{build_format, map_chart_type};

pub fn modify_file<F>(path: &str, params: &SecurityParams, modifier: F) -> Result<WriteResult>
where
    F: FnOnce(
        &std::collections::HashMap<String, SheetData>,
    ) -> Result<std::collections::HashMap<String, SheetData>>,
{
    let old_hash = compute_file_hash(path).map_err(AppError::Io)?;

    let backup_info = if params.create_backup {
        Some(create_backup(path, &old_hash).map_err(AppError::Io)?)
    } else {
        None
    };

    let old_data = read_all_sheets_to_map(path)?;
    let new_data = modifier(&old_data)?;

    if new_data.is_empty() {
        return Err(AppError::Custom(
            "Cannot delete all sheets from a workbook".to_string(),
        ));
    }

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

pub fn modify_file_with_wb<F>(
    path: &str,
    params: &SecurityParams,
    modifier: F,
) -> Result<WriteResult>
where
    F: FnOnce(&std::collections::HashMap<String, SheetData>, &mut Workbook) -> Result<()>,
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

pub fn error_to_formula(val: &str) -> Option<String> {
    match val {
        "#DIV/0!" => Some("1/0".to_string()),
        "#N/A" => Some("NA()".to_string()),
        "#NUM!" => Some("SQRT(-1)".to_string()),
        "#VALUE!" => Some("\"TEXT\"+1".to_string()),
        _ => None,
    }
}

pub fn ensure_dimensions(sd: &mut SheetData, row: usize, col: usize) {
    while sd.rows.len() <= row {
        sd.rows.push(Vec::new());
    }
    while sd.rows[row].len() <= col {
        sd.rows[row].push(CellData {
            value: None,
            data_type: CellDataType::Empty,
            formula: None,
        });
    }
}

pub fn cell_value_to_data(val: &CellValue) -> CellData {
    match val {
        CellValue::String(s) => CellData {
            value: Some(s.clone()),
            data_type: CellDataType::String,
            formula: None,
        },
        CellValue::Number(n) => CellData {
            value: Some(n.to_string()),
            data_type: CellDataType::Float,
            formula: None,
        },
        CellValue::Bool(b) => CellData {
            value: Some(b.to_string()),
            data_type: CellDataType::Bool,
            formula: None,
        },
        CellValue::DateTime(dt) => CellData {
            value: Some(dt.to_string()),
            data_type: CellDataType::DateTime,
            formula: None,
        },
        CellValue::Error(e) => CellData {
            value: Some(e.clone()),
            data_type: CellDataType::Error,
            formula: None,
        },
        CellValue::Empty => CellData {
            value: None,
            data_type: CellDataType::Empty,
            formula: None,
        },
    }
}

pub fn add(data: &mut std::collections::HashMap<String, SheetData>, name: &str) -> Result<()> {
    if data.contains_key(name) {
        return Err(AppError::SheetAlreadyExists(name.into()));
    }
    data.insert(
        name.to_string(),
        SheetData {
            name: name.to_string(),
            rows: Vec::new(),
        },
    );
    Ok(())
}

pub fn delete(data: &mut std::collections::HashMap<String, SheetData>, name: &str) -> Result<()> {
    if !data.contains_key(name) {
        return Err(AppError::SheetNotFound(name.into()));
    }
    data.remove(name);
    Ok(())
}

pub fn rename(
    data: &mut std::collections::HashMap<String, SheetData>,
    old_name: &str,
    new_name: &str,
) -> Result<()> {
    if !data.contains_key(old_name) {
        return Err(AppError::SheetNotFound(old_name.into()));
    }
    if data.contains_key(new_name) {
        return Err(AppError::SheetAlreadyExists(new_name.into()));
    }
    if let Some(mut sd) = data.remove(old_name) {
        sd.name = new_name.to_string();
        data.insert(new_name.to_string(), sd);
    }
    Ok(())
}

pub fn sort(
    data: &mut std::collections::HashMap<String, SheetData>,
    sheet: &str,
    columns: &[SortColumn],
) -> Result<()> {
    let sd = data
        .get_mut(sheet)
        .ok_or_else(|| AppError::SheetNotFound(sheet.into()))?;
    if sd.rows.len() > 1 {
        let header = sd.rows[0].clone();
        let mut body: Vec<Vec<CellData>> = sd.rows.drain(1..).collect();
        body.sort_by(|a, b| {
            for sc in columns {
                let ca = a
                    .get(sc.column as usize)
                    .and_then(|c| c.value.as_deref())
                    .unwrap_or("");
                let cb = b
                    .get(sc.column as usize)
                    .and_then(|c| c.value.as_deref())
                    .unwrap_or("");
                let cmp = ca.to_lowercase().cmp(&cb.to_lowercase());
                if cmp != std::cmp::Ordering::Equal {
                    return if sc.descending { cmp.reverse() } else { cmp };
                }
            }
            std::cmp::Ordering::Equal
        });
        // Replace the rows with sorted data
        sd.rows = vec![header];
        sd.rows.extend(body);
    }
    Ok(())
}

pub fn dedup(
    data: &mut std::collections::HashMap<String, SheetData>,
    sheet: &str,
    columns: &[u16],
) -> Result<()> {
    let sd = data
        .get_mut(sheet)
        .ok_or_else(|| AppError::SheetNotFound(sheet.into()))?;
    if sd.rows.len() > 1 {
        let header = sd.rows[0].clone();
        let body: Vec<Vec<CellData>> = sd.rows.drain(1..).collect();
        let mut seen = std::collections::HashSet::new();
        let cols: Vec<usize> = if columns.is_empty() {
            (0..body.iter().map(|r| r.len()).max().unwrap_or(0)).collect()
        } else {
            columns.iter().map(|c| *c as usize).collect()
        };
        let mut deduped_body = Vec::new();
        for row in body {
            let key: Vec<String> = cols
                .iter()
                .map(|&ci| {
                    row.get(ci)
                        .and_then(|c| c.value.as_deref())
                        .unwrap_or("")
                        .to_string()
                })
                .collect();
            if seen.insert(key) {
                deduped_body.push(row);
            }
        }
        // Replace the rows with header and deduped body
        sd.rows = vec![header];
        sd.rows.extend(deduped_body);
    }
    Ok(())
}

pub fn write_sheet_data(ws: &mut Worksheet, data: &SheetData) -> Result<()> {
    for (ri, row) in data.rows.iter().enumerate() {
        for (ci, cell) in row.iter().enumerate() {
            write_cell_data(ws, ri as u32, ci as u16, cell)?;
        }
    }
    Ok(())
}

pub fn write_cell_data(ws: &mut Worksheet, row: u32, col: u16, cell: &CellData) -> Result<()> {
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

pub fn write_cell_with_format(
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

pub fn build_workbook_with_ops(
    data: &std::collections::HashMap<String, SheetData>,
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
