use std::collections::HashMap;

use rust_xlsxwriter::{
    Chart, ChartType as XlsxChartType, Color, Format, FormatBorder, Workbook, Worksheet,
};

use crate::cell_ref;
use crate::excel_read::read_all_sheets_to_map;
use crate::security::{compute_file_hash, create_backup};
use crate::types::*;

// ---------------------------------------------------------------------------
// Public write API
// ---------------------------------------------------------------------------

pub fn create_file(path: &str, sheet_name: &str) -> Result<WriteResult> {
    let mut wb = Workbook::new();
    let ws = wb.add_worksheet();
    ws.set_name(sheet_name).map_err(AppError::Xlsx)?;
    wb.save(path).map_err(AppError::Xlsx)?;

    let new_hash = compute_file_hash(path).map_err(AppError::Io)?;
    Ok(WriteResult {
        success: true,
        message: format!("Created {}", path),
        backup_info: None,
        old_hash: String::new(),
        new_hash,
        diff: None,
    })
}

pub fn add_sheet(path: &str, params: &SecurityParams, sheet: &str) -> Result<WriteResult> {
    modify_file(path, params, |old_data, wb| {
        if old_data.contains_key(sheet) {
            return Err(AppError::Custom(format!(
                "Sheet '{}' already exists",
                sheet
            )));
        }
        wb.add_worksheet().set_name(sheet).map_err(AppError::Xlsx)?;

        let mut new_data = old_data.clone();
        new_data.insert(
            sheet.to_string(),
            SheetData {
                name: sheet.to_string(),
                rows: Vec::new(),
            },
        );
        Ok(new_data)
    })
}

pub fn delete_sheet(path: &str, params: &SecurityParams, sheet: &str) -> Result<WriteResult> {
    modify_file(path, params, |old_data, wb| {
        if !old_data.contains_key(sheet) {
            return Err(AppError::Custom(format!("Sheet '{}' not found", sheet)));
        }
        *wb = Workbook::new();
        for (name, data) in old_data.iter() {
            if name != sheet {
                let ws = wb.add_worksheet();
                ws.set_name(name).map_err(AppError::Xlsx)?;
                write_sheet_data(ws, data)?;
            }
        }
        let mut new_data = old_data.clone();
        new_data.remove(sheet);
        Ok(new_data)
    })
}

pub fn rename_sheet(
    path: &str,
    params: &SecurityParams,
    old_name: &str,
    new_name: &str,
) -> Result<WriteResult> {
    modify_file(path, params, |old_data, wb| {
        if !old_data.contains_key(old_name) {
            return Err(AppError::Custom(format!("Sheet '{}' not found", old_name)));
        }
        if old_data.contains_key(new_name) {
            return Err(AppError::Custom(format!(
                "Sheet '{}' already exists",
                new_name
            )));
        }
        *wb = Workbook::new();
        for (name, data) in old_data.iter() {
            let ws = wb.add_worksheet();
            let display_name = if name == old_name { new_name } else { name };
            ws.set_name(display_name).map_err(AppError::Xlsx)?;
            write_sheet_data(ws, data)?;
        }
        let mut new_data = old_data.clone();
        if let Some(data) = new_data.remove(old_name) {
            let mut renamed = data;
            renamed.name = new_name.to_string();
            new_data.insert(new_name.to_string(), renamed);
        }
        Ok(new_data)
    })
}

pub fn write_cell(
    path: &str,
    params: &SecurityParams,
    sheet: &str,
    row: u32,
    col: u16,
    value: &CellValue,
) -> Result<WriteResult> {
    modify_file(path, params, |old_data, wb| {
        let mut new_data = old_data.clone();
        if let Some(sd) = new_data.get_mut(sheet) {
            ensure_dimensions(sd, row as usize, col as usize);
            sd.rows[row as usize][col as usize] = cell_value_to_data(value);
        } else {
            return Err(AppError::Custom(format!("Sheet '{}' not found", sheet)));
        }
        *wb = Workbook::new();
        for (name, data) in new_data.iter() {
            let ws = wb.add_worksheet();
            ws.set_name(name).map_err(AppError::Xlsx)?;
            write_sheet_data(ws, data)?;
        }
        Ok(new_data)
    })
}

pub fn write_range(
    path: &str,
    params: &SecurityParams,
    sheet: &str,
    range_spec: &str,
    data: &[Vec<CellValue>],
) -> Result<WriteResult> {
    let (r1, c1, _, _) = cell_ref::parse_range(range_spec)?;

    modify_file(path, params, |old_data, wb| {
        let mut new_data = old_data.clone();
        let sd = new_data
            .get_mut(sheet)
            .ok_or_else(|| AppError::Custom(format!("Sheet '{}' not found", sheet)))?;

        for (ri, row) in data.iter().enumerate() {
            for (ci, val) in row.iter().enumerate() {
                let target_row = r1 as usize + ri;
                let target_col = c1 as usize + ci;
                ensure_dimensions(sd, target_row, target_col);
                sd.rows[target_row][target_col] = cell_value_to_data(val);
            }
        }

        *wb = Workbook::new();
        for (name, d) in new_data.iter() {
            let ws = wb.add_worksheet();
            ws.set_name(name).map_err(AppError::Xlsx)?;
            write_sheet_data(ws, d)?;
        }
        Ok(new_data)
    })
}

pub fn clear_range(
    path: &str,
    params: &SecurityParams,
    sheet: &str,
    range_spec: &str,
) -> Result<WriteResult> {
    let (r_start, r_end, c_start, c_end) = cell_ref::parse_range_normalized(range_spec)?;

    modify_file(path, params, |old_data, wb| {
        let mut new_data = old_data.clone();
        let sd = new_data
            .get_mut(sheet)
            .ok_or_else(|| AppError::Custom(format!("Sheet '{}' not found", sheet)))?;

        for ri in r_start..=r_end {
            for ci in c_start..=c_end {
                let row = ri as usize;
                let col = ci as usize;
                if row < sd.rows.len() && col < sd.rows[row].len() {
                    sd.rows[row][col] = CellData {
                        value: None,
                        data_type: CellDataType::Empty,
                        formula: None,
                    };
                }
            }
        }

        *wb = Workbook::new();
        for (name, d) in new_data.iter() {
            let ws = wb.add_worksheet();
            ws.set_name(name).map_err(AppError::Xlsx)?;
            write_sheet_data(ws, d)?;
        }
        Ok(new_data)
    })
}

pub fn set_formula(
    path: &str,
    params: &SecurityParams,
    sheet: &str,
    cell_spec: &str,
    formula: &str,
) -> Result<WriteResult> {
    let (row, col) = cell_ref::parse_cell_ref(cell_spec)?;

    modify_file(path, params, |old_data, wb| {
        let mut new_data = old_data.clone();
        let sd = new_data
            .get_mut(sheet)
            .ok_or_else(|| AppError::Custom(format!("Sheet '{}' not found", sheet)))?;

        ensure_dimensions(sd, row as usize, col as usize);
        sd.rows[row as usize][col as usize] = CellData {
            value: None,
            data_type: CellDataType::String,
            formula: Some(formula.to_string()),
        };

        *wb = Workbook::new();
        for (name, d) in new_data.iter() {
            let ws = wb.add_worksheet();
            ws.set_name(name).map_err(AppError::Xlsx)?;
            write_sheet_data(ws, d)?;
        }
        Ok(new_data)
    })
}

pub fn set_format(
    path: &str,
    params: &SecurityParams,
    sheet: &str,
    cell_spec: &str,
    style: &Style,
) -> Result<WriteResult> {
    let (row, col) = cell_ref::parse_cell_ref(cell_spec)?;

    modify_file(path, params, |old_data, wb| {
        *wb = Workbook::new();
        for (name, data) in old_data.iter() {
            let ws = wb.add_worksheet();
            ws.set_name(name).map_err(AppError::Xlsx)?;

            if name == sheet {
                for (ri, row_data) in data.rows.iter().enumerate() {
                    for (ci, cell) in row_data.iter().enumerate() {
                        let fmt = build_format(style);
                        if ri == row as usize && ci == col as usize {
                            write_cell_with_format(ws, ri as u32, ci as u16, cell, &fmt)?;
                        } else {
                            write_cell_data(ws, ri as u32, ci as u16, cell)?;
                        }
                    }
                }
            } else {
                write_sheet_data(ws, data)?;
            }
        }
        Ok(old_data.clone())
    })
}

pub fn merge_cells(
    path: &str,
    params: &SecurityParams,
    sheet: &str,
    range_spec: &str,
    value: &str,
) -> Result<WriteResult> {
    let (r1, c1, r2, c2) = cell_ref::parse_range(range_spec)?;

    modify_file(path, params, |old_data, wb| {
        *wb = Workbook::new();
        for (name, data) in old_data.iter() {
            let ws = wb.add_worksheet();
            ws.set_name(name).map_err(AppError::Xlsx)?;

            if name == sheet {
                write_sheet_data(ws, data)?;
                ws.merge_range(r1, c1, r2, c2, value, &Format::new())
                    .map_err(AppError::Xlsx)?;
            } else {
                write_sheet_data(ws, data)?;
            }
        }

        let mut new_data = old_data.clone();
        if let Some(sd) = new_data.get_mut(sheet) {
            ensure_dimensions(sd, r2 as usize, c2 as usize);
            sd.rows[r1 as usize][c1 as usize] = CellData {
                value: Some(value.to_string()),
                data_type: CellDataType::String,
                formula: None,
            };
            for ri in r1..=r2 {
                for ci in c1..=c2 {
                    let row = ri as usize;
                    let col = ci as usize;
                    if (ri != r1 || ci != c1) && row < sd.rows.len() && col < sd.rows[row].len() {
                        sd.rows[row][col] = CellData {
                            value: None,
                            data_type: CellDataType::Empty,
                            formula: None,
                        };
                    }
                }
            }
        }
        Ok(new_data)
    })
}

pub fn add_chart(path: &str, params: &SecurityParams, config: &ChartConfig) -> Result<WriteResult> {
    modify_file(path, params, |old_data, wb| {
        *wb = Workbook::new();
        for (name, data) in old_data.iter() {
            let ws = wb.add_worksheet();
            ws.set_name(name).map_err(AppError::Xlsx)?;
            write_sheet_data(ws, data)?;

            if name == &config.sheet {
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
        Ok(old_data.clone())
    })
}

pub fn refresh_formulas(path: &str, params: &SecurityParams, _sheet: &str) -> Result<WriteResult> {
    modify_file(path, params, |_old_data, _wb| Ok(_old_data.clone()))
}

pub fn set_calculation_mode(
    path: &str,
    params: &SecurityParams,
    _mode: &str,
) -> Result<WriteResult> {
    modify_file(path, params, |_old_data, _wb| Ok(_old_data.clone()))
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn modify_file<F>(path: &str, params: &SecurityParams, modifier: F) -> Result<WriteResult>
where
    F: FnOnce(&HashMap<String, SheetData>, &mut Workbook) -> Result<HashMap<String, SheetData>>,
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

    let _new_data = modifier(&old_data, &mut wb)?;

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

fn write_sheet_data(ws: &mut Worksheet, data: &SheetData) -> Result<()> {
    for (ri, row) in data.rows.iter().enumerate() {
        for (ci, cell) in row.iter().enumerate() {
            write_cell_data(ws, ri as u32, ci as u16, cell)?;
        }
    }
    Ok(())
}

fn write_cell_data(ws: &mut Worksheet, row: u32, col: u16, cell: &CellData) -> Result<()> {
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

fn write_cell_with_format(
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
            _ => {
                ws.write_string_with_format(row, col, val, fmt)
                    .map_err(AppError::Xlsx)?;
            }
        }
    }
    Ok(())
}

fn ensure_dimensions(sd: &mut SheetData, row: usize, col: usize) {
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

fn cell_value_to_data(val: &CellValue) -> CellData {
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
        CellValue::Empty => CellData {
            value: None,
            data_type: CellDataType::Empty,
            formula: None,
        },
    }
}

fn build_format(style: &Style) -> Format {
    let mut fmt = Format::new();
    if let Some(ref name) = style.font_name {
        fmt = fmt.set_font_name(name);
    }
    if let Some(size) = style.font_size {
        fmt = fmt.set_font_size(size);
    }
    if let Some(true) = style.bold {
        fmt = fmt.set_bold();
    }
    if let Some(true) = style.italic {
        fmt = fmt.set_italic();
    }
    if let Some(ref color) = style.font_color
        && let Some(c) = parse_color(color)
    {
        fmt = fmt.set_font_color(c);
    }
    if let Some(ref bg) = style.background_color
        && let Some(c) = parse_color(bg)
    {
        fmt = fmt.set_background_color(c);
    }
    if let Some(ref border) = style.border {
        let b = match border.to_lowercase().as_str() {
            "thin" => FormatBorder::Thin,
            "medium" => FormatBorder::Medium,
            "thick" => FormatBorder::Thick,
            "double" => FormatBorder::Double,
            "dotted" => FormatBorder::Dotted,
            "dashed" => FormatBorder::Dashed,
            _ => FormatBorder::Thin,
        };
        fmt = fmt.set_border(b);
    }
    fmt
}

fn parse_color(color: &str) -> Option<Color> {
    let s = color.trim_start_matches('#');
    if s.len() == 6 {
        u32::from_str_radix(s, 16)
            .ok()
            .map(|v| Color::RGB(v | 0xFF000000))
    } else if s.len() == 8 {
        u32::from_str_radix(s, 16).ok().map(Color::RGB)
    } else {
        match s.to_lowercase().as_str() {
            "red" => Some(Color::Red),
            "blue" => Some(Color::Blue),
            "green" => Some(Color::Green),
            "yellow" => Some(Color::Yellow),
            "white" => Some(Color::White),
            "black" => Some(Color::Black),
            "orange" => Some(Color::Orange),
            "purple" => Some(Color::Purple),
            "pink" => Some(Color::Pink),
            "cyan" => Some(Color::Cyan),
            "brown" => Some(Color::Brown),
            "magenta" => Some(Color::Magenta),
            "gray" => Some(Color::Gray),
            "lime" => Some(Color::Lime),
            "navy" => Some(Color::Navy),
            _ => None,
        }
    }
}

fn map_chart_type(ct: &ChartType) -> XlsxChartType {
    match ct {
        ChartType::Column => XlsxChartType::Column,
        ChartType::Line => XlsxChartType::Line,
        ChartType::Pie => XlsxChartType::Pie,
        ChartType::Bar => XlsxChartType::Bar,
        ChartType::Area => XlsxChartType::Area,
        ChartType::Scatter => XlsxChartType::Scatter,
    }
}
