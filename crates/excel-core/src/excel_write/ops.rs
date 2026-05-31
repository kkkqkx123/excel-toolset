use rust_xlsxwriter::{Chart, Format, Workbook};

use crate::cell_ref;
use crate::types::*;

use super::chart::map_chart_type;
use super::data::{cell_value_to_data, ensure_dimensions};
use super::style::build_format;
use super::write::{modify_file, write_cell_data, write_cell_with_format, write_sheet_data};

pub fn add_sheet(path: &str, params: &SecurityParams, sheet: &str) -> Result<WriteResult> {
    modify_file(path, params, |old_data, wb| {
        let mut new_data = old_data.clone();
        if new_data.contains_key(sheet) {
            return Err(AppError::SheetAlreadyExists(sheet.into()));
        }
        new_data.insert(
            sheet.to_string(),
            SheetData {
                name: sheet.to_string(),
                rows: Vec::new(),
            },
        );
        wb.add_worksheet().set_name(sheet).map_err(AppError::Xlsx)?;
        Ok(new_data)
    })
}

pub fn delete_sheet(path: &str, params: &SecurityParams, sheet: &str) -> Result<WriteResult> {
    modify_file(path, params, |old_data, wb| {
        if !old_data.contains_key(sheet) {
            return Err(AppError::SheetNotFound(sheet.into()));
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
            return Err(AppError::SheetNotFound(old_name.into()));
        }
        if old_data.contains_key(new_name) {
            return Err(AppError::SheetAlreadyExists(new_name.into()));
        }
        *wb = Workbook::new();
        for (name, data) in old_data.iter() {
            let ws = wb.add_worksheet();
            let display_name = if name == old_name { new_name } else { name };
            ws.set_name(display_name).map_err(AppError::Xlsx)?;
            write_sheet_data(ws, data)?;
        }
        let mut new_data = old_data.clone();
        if let Some(mut data) = new_data.remove(old_name) {
            data.name = new_name.to_string();
            new_data.insert(new_name.to_string(), data);
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
            return Err(AppError::SheetNotFound(sheet.into()));
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
            .ok_or_else(|| AppError::SheetNotFound(sheet.into()))?;

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
            .ok_or_else(|| AppError::SheetNotFound(sheet.into()))?;

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
            .ok_or_else(|| AppError::SheetNotFound(sheet.into()))?;

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

pub fn refresh_formulas(path: &str, params: &SecurityParams, sheet: &str) -> Result<WriteResult> {
    modify_file(path, params, |old_data, wb| {
        let mut new_data = old_data.clone();

        for (name, data) in new_data.iter_mut() {
            if name == sheet || sheet == "*" {
                for row in data.rows.iter_mut() {
                    for cell in row.iter_mut() {
                        if cell.formula.is_some() {
                            cell.value = None;
                        }
                    }
                }
            }
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

pub fn set_calculation_mode(
    path: &str,
    params: &SecurityParams,
    _mode: &str,
) -> Result<WriteResult> {
    modify_file(path, params, |_old_data, _wb| Ok(_old_data.clone()))
}
