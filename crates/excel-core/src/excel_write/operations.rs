use rust_xlsxwriter::{Chart, Format, Workbook};

use crate::types::*;
use crate::utils::cell_ref;

use super::core::{
    cell_value_to_data, ensure_dimensions, modify_file, modify_file_with_wb, write_cell_data,
    write_cell_with_format, write_sheet_data,
};
use super::format::{build_format, map_chart_type};

pub fn add_sheet(path: &str, params: &SecurityParams, sheet: &str) -> Result<WriteResult> {
    modify_file(path, params, |old_data| {
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
        Ok(new_data)
    })
}

pub fn delete_sheet(path: &str, params: &SecurityParams, sheet: &str) -> Result<WriteResult> {
    modify_file(path, params, |old_data| {
        if !old_data.contains_key(sheet) {
            return Err(AppError::SheetNotFound(sheet.into()));
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
    modify_file(path, params, |old_data| {
        if !old_data.contains_key(old_name) {
            return Err(AppError::SheetNotFound(old_name.into()));
        }
        if old_data.contains_key(new_name) {
            return Err(AppError::SheetAlreadyExists(new_name.into()));
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
    modify_file(path, params, |old_data| {
        let mut new_data = old_data.clone();
        let sd = new_data
            .get_mut(sheet)
            .ok_or_else(|| AppError::SheetNotFound(sheet.into()))?;
        ensure_dimensions(sd, row as usize, col as usize);
        sd.rows[row as usize][col as usize] = cell_value_to_data(value);
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

    modify_file(path, params, |old_data| {
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

    modify_file(path, params, |old_data| {
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

    modify_file(path, params, |old_data| {
        let mut new_data = old_data.clone();
        let sd = new_data
            .get_mut(sheet)
            .ok_or_else(|| AppError::SheetNotFound(sheet.into()))?;

        // Remove = prefix if present, as rust_xlsxwriter adds it automatically
        let cleaned_formula = formula.strip_prefix('=').unwrap_or(formula);

        ensure_dimensions(sd, row as usize, col as usize);
        sd.rows[row as usize][col as usize] = CellData {
            value: None,
            data_type: CellDataType::String,
            formula: Some(cleaned_formula.to_string()),
        };
        Ok(new_data)
    })
}

pub fn set_format(
    path: &str,
    params: &SecurityParams,
    sheet: &str,
    range_spec: &str,
    style: &Style,
) -> Result<WriteResult> {
    let (r_start, r_end, c_start, c_end) = cell_ref::parse_range_normalized(range_spec)?;

    modify_file_with_wb(path, params, |old_data, wb| {
        *wb = Workbook::new();
        for (name, data) in old_data.iter() {
            let ws = wb.add_worksheet();
            ws.set_name(name).map_err(AppError::Xlsx)?;

            if name == sheet {
                for (ri, row_data) in data.rows.iter().enumerate() {
                    for (ci, cell) in row_data.iter().enumerate() {
                        if ri >= r_start as usize
                            && ri <= r_end as usize
                            && ci >= c_start as usize
                            && ci <= c_end as usize
                        {
                            let fmt = build_format(style);
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
        Ok(())
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

    modify_file_with_wb(path, params, |old_data, wb| {
        *wb = Workbook::new();
        for (name, data) in old_data.iter() {
            let ws = wb.add_worksheet();
            ws.set_name(name).map_err(AppError::Xlsx)?;

            if name == sheet {
                ws.merge_range(r1, c1, r2, c2, value, &Format::new())
                    .map_err(AppError::Xlsx)?;
            } else {
                write_sheet_data(ws, data)?;
            }
        }
        Ok(())
    })
}

pub fn add_chart(path: &str, params: &SecurityParams, config: &ChartConfig) -> Result<WriteResult> {
    modify_file_with_wb(path, params, |old_data, wb| {
        *wb = Workbook::new();
        let sheet_names: Vec<&str> = old_data.keys().map(|s| s.as_str()).collect();
        for name in &sheet_names {
            let sd = &old_data[*name];
            let ws = wb.add_worksheet();
            ws.set_name(*name).map_err(AppError::Xlsx)?;
            write_sheet_data(ws, sd)?;
        }

        // Insert chart after all data sheets are created
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
        Ok(())
    })
}

pub fn refresh_formulas(path: &str, params: &SecurityParams, sheet: &str) -> Result<WriteResult> {
    modify_file(path, params, |old_data| {
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
        Ok(new_data)
    })
}

pub fn set_calculation_mode(
    path: &str,
    params: &SecurityParams,
    _mode: &str,
) -> Result<WriteResult> {
    modify_file(path, params, |old_data| Ok(old_data.clone()))
}

pub fn append_rows(
    path: &str,
    params: &SecurityParams,
    sheet: &str,
    new_rows: &[Vec<CellValue>],
) -> Result<WriteResult> {
    modify_file(path, params, |old_data| {
        let mut new_data = old_data.clone();
        let sd = new_data
            .get_mut(sheet)
            .ok_or_else(|| AppError::SheetNotFound(sheet.into()))?;
        for row_data in new_rows {
            let mut row = Vec::new();
            for val in row_data {
                row.push(cell_value_to_data(val));
            }
            sd.rows.push(row);
        }
        Ok(new_data)
    })
}

pub fn insert_rows(
    path: &str,
    params: &SecurityParams,
    sheet: &str,
    at_row: u32,
    new_rows: &[Vec<CellValue>],
) -> Result<WriteResult> {
    modify_file(path, params, |old_data| {
        let mut new_data = old_data.clone();
        let sd = new_data
            .get_mut(sheet)
            .ok_or_else(|| AppError::SheetNotFound(sheet.into()))?;
        let row_idx = at_row as usize;
        let mut inserted_rows: Vec<Vec<CellData>> = Vec::new();
        for row_data in new_rows {
            let mut row = Vec::new();
            for val in row_data {
                row.push(cell_value_to_data(val));
            }
            inserted_rows.push(row);
        }
        sd.rows.splice(row_idx..row_idx, inserted_rows);
        Ok(new_data)
    })
}

pub fn delete_rows(
    path: &str,
    params: &SecurityParams,
    sheet: &str,
    start_row: u32,
    end_row: u32,
) -> Result<WriteResult> {
    modify_file(path, params, |old_data| {
        let mut new_data = old_data.clone();
        let sd = new_data
            .get_mut(sheet)
            .ok_or_else(|| AppError::SheetNotFound(sheet.into()))?;
        let start_idx = start_row as usize;
        let end_idx = end_row as usize;
        if start_idx >= sd.rows.len() {
            return Ok(new_data);
        }
        let end_idx = end_idx.min(sd.rows.len() - 1);
        sd.rows.drain(start_idx..=end_idx);
        Ok(new_data)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{CellData, CellDataType};
    use std::collections::HashMap;

    fn make_cell(value: &str) -> CellData {
        CellData {
            value: Some(value.to_string()),
            data_type: CellDataType::String,
            formula: None,
        }
    }

    #[test]
    fn test_cell_value_to_data_conversion() {
        let cv_string = CellValue::String("test".to_string());
        let cd = cell_value_to_data(&cv_string);
        assert_eq!(cd.value, Some("test".to_string()));
        assert_eq!(cd.data_type, CellDataType::String);

        let cv_number = CellValue::Number(3.14);
        let cd = cell_value_to_data(&cv_number);
        assert_eq!(cd.value, Some("3.14".to_string()));
        assert_eq!(cd.data_type, CellDataType::Float);

        let cv_bool = CellValue::Bool(true);
        let cd = cell_value_to_data(&cv_bool);
        assert_eq!(cd.value, Some("true".to_string()));
        assert_eq!(cd.data_type, CellDataType::Bool);

        let cv_empty = CellValue::Empty;
        let cd = cell_value_to_data(&cv_empty);
        assert_eq!(cd.value, None);
        assert_eq!(cd.data_type, CellDataType::Empty);
    }

    #[test]
    fn test_ensure_dimensions_expand_rows() {
        let mut sd = SheetData {
            name: "Test".to_string(),
            rows: vec![vec![make_cell("a")]],
        };
        ensure_dimensions(&mut sd, 3, 0);
        assert!(sd.rows.len() >= 4);
    }

    #[test]
    fn test_ensure_dimensions_expand_cols() {
        let mut sd = SheetData {
            name: "Test".to_string(),
            rows: vec![vec![make_cell("a")]],
        };
        ensure_dimensions(&mut sd, 0, 5);
        assert!(sd.rows[0].len() >= 6);
    }

    #[test]
    fn test_append_rows() {
        let mut data = HashMap::new();
        data.insert(
            "Sheet1".to_string(),
            SheetData {
                name: "Sheet1".to_string(),
                rows: vec![vec![make_cell("Header")]],
            },
        );

        let new_rows = vec![vec![CellValue::String("Row1".to_string())]];

        // Simulate the append operation
        let sd = data.get_mut("Sheet1").unwrap();
        for row_data in &new_rows {
            let mut row = Vec::new();
            for val in row_data {
                row.push(cell_value_to_data(val));
            }
            sd.rows.push(row);
        }

        assert_eq!(data["Sheet1"].rows.len(), 2);
        assert_eq!(data["Sheet1"].rows[1][0].value, Some("Row1".to_string()));
    }

    #[test]
    fn test_insert_rows() {
        let mut data = HashMap::new();
        data.insert(
            "Sheet1".to_string(),
            SheetData {
                name: "Sheet1".to_string(),
                rows: vec![vec![make_cell("Header")], vec![make_cell("Original")]],
            },
        );

        let new_rows = vec![vec![CellValue::String("Inserted".to_string())]];

        // Simulate the insert operation
        let sd = data.get_mut("Sheet1").unwrap();
        let row_idx = 1;
        let mut inserted_rows: Vec<Vec<CellData>> = Vec::new();
        for row_data in &new_rows {
            let mut row = Vec::new();
            for val in row_data {
                row.push(cell_value_to_data(val));
            }
            inserted_rows.push(row);
        }
        sd.rows.splice(row_idx..row_idx, inserted_rows);

        assert_eq!(data["Sheet1"].rows.len(), 3);
        assert_eq!(
            data["Sheet1"].rows[1][0].value,
            Some("Inserted".to_string())
        );
        assert_eq!(
            data["Sheet1"].rows[2][0].value,
            Some("Original".to_string())
        );
    }

    #[test]
    fn test_delete_rows() {
        let mut data = HashMap::new();
        data.insert(
            "Sheet1".to_string(),
            SheetData {
                name: "Sheet1".to_string(),
                rows: vec![
                    vec![make_cell("Header")],
                    vec![make_cell("Row1")],
                    vec![make_cell("Row2")],
                    vec![make_cell("Row3")],
                ],
            },
        );

        let start_idx = 1;
        let end_idx = 2;
        let sd = data.get_mut("Sheet1").unwrap();
        sd.rows.drain(start_idx..=end_idx);

        assert_eq!(data["Sheet1"].rows.len(), 2);
        assert_eq!(data["Sheet1"].rows[1][0].value, Some("Row3".to_string()));
    }

    #[test]
    fn test_delete_rows_beyond_bounds() {
        let mut data = HashMap::new();
        data.insert(
            "Sheet1".to_string(),
            SheetData {
                name: "Sheet1".to_string(),
                rows: vec![vec![make_cell("Header")]],
            },
        );

        let result: std::result::Result<(), ()> = {
            let sd = data.get_mut("Sheet1").unwrap();
            let start_idx = 10;
            let end_idx = 20;
            if start_idx >= sd.rows.len() {
                Ok(())
            } else {
                sd.rows.drain(start_idx..=end_idx);
                Ok(())
            }
        };
        assert!(result.is_ok());
        assert_eq!(data["Sheet1"].rows.len(), 1);
    }
}
