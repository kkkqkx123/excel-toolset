use rust_xlsxwriter::{Chart, Format, Table as XlsxTable, Workbook, Worksheet};

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
                        if let Ok((r_start, r_end, c_start, c_end)) =
                            cell_ref::parse_range_normalized(range_str)
                        {
                            if ri as u32 >= r_start
                                && ri as u32 <= r_end
                                && ci as u16 >= c_start
                                && ci as u16 <= c_end
                            {
                                let fmt = build_format(style);
                                write_cell_with_format(ws, ri as u32, ci as u16, cell, &fmt)?;
                                applied_format = true;
                                break;
                            }
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

        // Add tables for this worksheet
        for op in operations {
            if let BatchOperation::AddTable { config } = op {
                let (r1, c1, r2, c2) = cell_ref::parse_range(&config.range)?;
                let mut table = XlsxTable::new();
                table = table.set_name(&config.name);
                if config.has_header {
                    table = table.set_header_row(true);
                }
                if config.has_total {
                    table = table.set_total_row(true);
                }
                ws.add_table(r1, c1, r2, c2, &table)
                    .map_err(AppError::Xlsx)?;
            }
        }

        // Add data validations for this worksheet
        for op in operations {
            if let BatchOperation::AddDataValidation { sheet, config } = op {
                if sheet != *name {
                    continue;
                }
                let (r1, c1, r2, c2) = cell_ref::parse_range(&config.range)?;
                let dv = crate::features::data_validation::build_data_validation(&config)?;
                ws.add_data_validation(r1, c1, r2, c2, &dv)
                    .map_err(AppError::Xlsx)?;
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
        if let BatchOperation::AddPivotTable { config } = op {
            let target_sheet_idx = sheet_names
                .iter()
                .position(|n| *n == config.target_sheet)
                .ok_or_else(|| AppError::SheetNotFound(config.target_sheet.clone()))?;
            if let Ok(ws) = wb.worksheet_from_index(target_sheet_idx) {
                let (target_r, target_c) =
                    cell_ref::parse_cell_ref(&config.target_cell)?;
                // Write pivot table label as a placeholder
                ws.write(target_r, target_c, &format!("PivotTable: {}", config.name))
                    .map_err(AppError::Xlsx)?;
                // Note: Full pivot table rendering is handled by
                // features::pivot_table::create_pivot_table via modify_file_with_wb
            }
        }
    }

    Ok(wb)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{CellData, CellDataType, SheetData};
    use std::collections::HashMap;

    fn make_cell(value: &str, data_type: CellDataType) -> CellData {
        CellData {
            value: Some(value.to_string()),
            data_type,
            formula: None,
        }
    }

    fn make_sheet(name: &str, rows: Vec<Vec<CellData>>) -> SheetData {
        SheetData {
            name: name.to_string(),
            rows,
        }
    }

    #[test]
    fn test_cell_value_to_data_string() {
        let cv = CellValue::String("hello".to_string());
        let cd = cell_value_to_data(&cv);
        assert_eq!(cd.value, Some("hello".to_string()));
        assert_eq!(cd.data_type, CellDataType::String);
    }

    #[test]
    fn test_cell_value_to_data_number() {
        let cv = CellValue::Number(42.5);
        let cd = cell_value_to_data(&cv);
        assert_eq!(cd.value, Some("42.5".to_string()));
        assert_eq!(cd.data_type, CellDataType::Float);
    }

    #[test]
    fn test_cell_value_to_data_bool() {
        let cv = CellValue::Bool(true);
        let cd = cell_value_to_data(&cv);
        assert_eq!(cd.value, Some("true".to_string()));
        assert_eq!(cd.data_type, CellDataType::Bool);
    }

    #[test]
    fn test_cell_value_to_data_empty() {
        let cv = CellValue::Empty;
        let cd = cell_value_to_data(&cv);
        assert_eq!(cd.value, None);
        assert_eq!(cd.data_type, CellDataType::Empty);
    }

    #[test]
    fn test_cell_value_to_data_error() {
        let cv = CellValue::Error("#DIV/0!".to_string());
        let cd = cell_value_to_data(&cv);
        assert_eq!(cd.value, Some("#DIV/0!".to_string()));
        assert_eq!(cd.data_type, CellDataType::Error);
    }

    #[test]
    fn test_ensure_dimensions_expands_rows() {
        let mut sheet = SheetData {
            name: "Test".to_string(),
            rows: vec![vec![make_cell("a", CellDataType::String)]],
        };
        ensure_dimensions(&mut sheet, 3, 0);
        assert_eq!(sheet.rows.len(), 4);
    }

    #[test]
    fn test_ensure_dimensions_expands_cols() {
        let mut sheet = SheetData {
            name: "Test".to_string(),
            rows: vec![vec![make_cell("a", CellDataType::String)]],
        };
        ensure_dimensions(&mut sheet, 0, 3);
        assert_eq!(sheet.rows[0].len(), 4);
    }

    #[test]
    fn test_add_sheet_to_map() {
        let mut data = HashMap::new();
        add(&mut data, "Sheet1").unwrap();
        assert!(data.contains_key("Sheet1"));
        assert_eq!(data["Sheet1"].rows.len(), 0);
    }

    #[test]
    fn test_add_sheet_duplicate_error() {
        let mut data = HashMap::new();
        add(&mut data, "Sheet1").unwrap();
        let result = add(&mut data, "Sheet1");
        assert!(result.is_err());
    }

    #[test]
    fn test_delete_sheet_from_map() {
        let mut data = HashMap::new();
        add(&mut data, "Sheet1").unwrap();
        add(&mut data, "Sheet2").unwrap();
        delete(&mut data, "Sheet1").unwrap();
        assert!(!data.contains_key("Sheet1"));
        assert!(data.contains_key("Sheet2"));
    }

    #[test]
    fn test_delete_sheet_not_found_error() {
        let mut data = HashMap::new();
        let result = delete(&mut data, "NonExistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_rename_sheet() {
        let mut data = HashMap::new();
        add(&mut data, "OldName").unwrap();
        rename(&mut data, "OldName", "NewName").unwrap();
        assert!(!data.contains_key("OldName"));
        assert!(data.contains_key("NewName"));
        assert_eq!(data["NewName"].name, "NewName");
    }

    #[test]
    fn test_rename_sheet_source_not_found() {
        let mut data = HashMap::new();
        let result = rename(&mut data, "NonExistent", "NewName");
        assert!(result.is_err());
    }

    #[test]
    fn test_rename_sheet_target_exists() {
        let mut data = HashMap::new();
        add(&mut data, "Sheet1").unwrap();
        add(&mut data, "Sheet2").unwrap();
        let result = rename(&mut data, "Sheet1", "Sheet2");
        assert!(result.is_err());
    }

    #[test]
    fn test_sort_single_column_ascending() {
        let mut data = HashMap::new();
        let sheet = make_sheet(
            "Data",
            vec![
                vec![
                    make_cell("Name", CellDataType::String),
                    make_cell("Age", CellDataType::String),
                ],
                vec![
                    make_cell("Charlie", CellDataType::String),
                    make_cell("35", CellDataType::String),
                ],
                vec![
                    make_cell("Alice", CellDataType::String),
                    make_cell("25", CellDataType::String),
                ],
                vec![
                    make_cell("Bob", CellDataType::String),
                    make_cell("30", CellDataType::String),
                ],
            ],
        );
        data.insert("Data".to_string(), sheet);

        let sort_columns = vec![SortColumn {
            column: 0,
            descending: false,
        }];
        sort(&mut data, "Data", &sort_columns).unwrap();

        let sheet = &data["Data"];
        assert_eq!(sheet.rows[1][0].value, Some("Alice".to_string()));
        assert_eq!(sheet.rows[2][0].value, Some("Bob".to_string()));
        assert_eq!(sheet.rows[3][0].value, Some("Charlie".to_string()));
    }

    #[test]
    fn test_sort_single_column_descending() {
        let mut data = HashMap::new();
        let sheet = make_sheet(
            "Data",
            vec![
                vec![make_cell("Name", CellDataType::String)],
                vec![make_cell("Alice", CellDataType::String)],
                vec![make_cell("Bob", CellDataType::String)],
            ],
        );
        data.insert("Data".to_string(), sheet);

        let sort_columns = vec![SortColumn {
            column: 0,
            descending: true,
        }];
        sort(&mut data, "Data", &sort_columns).unwrap();

        let sheet = &data["Data"];
        assert_eq!(sheet.rows[1][0].value, Some("Bob".to_string()));
        assert_eq!(sheet.rows[2][0].value, Some("Alice".to_string()));
    }

    #[test]
    fn test_sort_empty_sheet() {
        let mut data = HashMap::new();
        let sheet = make_sheet("Data", vec![]);
        data.insert("Data".to_string(), sheet);

        let sort_columns = vec![SortColumn {
            column: 0,
            descending: false,
        }];
        let result = sort(&mut data, "Data", &sort_columns);
        assert!(result.is_ok());
    }

    #[test]
    fn test_sort_sheet_not_found() {
        let mut data = HashMap::new();
        let sort_columns = vec![SortColumn {
            column: 0,
            descending: false,
        }];
        let result = sort(&mut data, "NonExistent", &sort_columns);
        assert!(result.is_err());
    }

    #[test]
    fn test_dedup_all_columns() {
        let mut data = HashMap::new();
        let sheet = make_sheet(
            "Data",
            vec![
                vec![
                    make_cell("A", CellDataType::String),
                    make_cell("B", CellDataType::String),
                ],
                vec![
                    make_cell("1", CellDataType::String),
                    make_cell("2", CellDataType::String),
                ],
                vec![
                    make_cell("1", CellDataType::String),
                    make_cell("2", CellDataType::String),
                ],
                vec![
                    make_cell("3", CellDataType::String),
                    make_cell("4", CellDataType::String),
                ],
            ],
        );
        data.insert("Data".to_string(), sheet);

        dedup(&mut data, "Data", &[]).unwrap();

        let sheet = &data["Data"];
        assert_eq!(sheet.rows.len(), 3); // header + 2 unique rows
    }

    #[test]
    fn test_dedup_specific_columns() {
        let mut data = HashMap::new();
        let sheet = make_sheet(
            "Data",
            vec![
                vec![
                    make_cell("A", CellDataType::String),
                    make_cell("B", CellDataType::String),
                ],
                vec![
                    make_cell("1", CellDataType::String),
                    make_cell("2", CellDataType::String),
                ],
                vec![
                    make_cell("1", CellDataType::String),
                    make_cell("3", CellDataType::String),
                ],
                vec![
                    make_cell("2", CellDataType::String),
                    make_cell("2", CellDataType::String),
                ],
            ],
        );
        data.insert("Data".to_string(), sheet);

        // Dedup only on column 0
        dedup(&mut data, "Data", &[0]).unwrap();

        let sheet = &data["Data"];
        assert_eq!(sheet.rows.len(), 3); // header + 2 unique by col 0
    }

    #[test]
    fn test_error_to_formula() {
        assert_eq!(error_to_formula("#DIV/0!"), Some("1/0".to_string()));
        assert_eq!(error_to_formula("#N/A"), Some("NA()".to_string()));
        assert_eq!(error_to_formula("#NUM!"), Some("SQRT(-1)".to_string()));
        assert_eq!(error_to_formula("#VALUE!"), Some("\"TEXT\"+1".to_string()));
        assert_eq!(error_to_formula("#REF!"), None);
    }
}
