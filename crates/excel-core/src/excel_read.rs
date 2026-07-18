use std::collections::HashMap;

use calamine::{Data, Reader, Xlsx, open_workbook};

use crate::security::compute_file_hash;
use crate::types::{
    AppError, CellData, CellDataType, FileInfo, OutputMode, ReadRangeData, ReadRangeOptions,
    ReadRangeResult, Result, SheetData,
};
use crate::utils::cell_ref;

pub fn read_file_info(path: &str) -> Result<FileInfo> {
    let sheets = list_sheets(path)?;
    let hash = compute_file_hash(path).map_err(AppError::Io)?;
    let size = std::fs::metadata(path)
        .map(|m| m.len())
        .map_err(AppError::Io)?;

    Ok(FileInfo {
        path: path.to_string(),
        hash,
        size,
        sheets,
        created_at: chrono::Utc::now(),
    })
}

pub fn list_sheets(path: &str) -> Result<Vec<String>> {
    let workbook: Xlsx<_> = open_workbook(path)?;
    Ok(workbook.sheet_names().to_vec())
}

pub fn read_cell(path: &str, sheet: &str, row: u32, col: u16) -> Result<CellData> {
    let mut workbook: Xlsx<_> = open_workbook(path)?;

    let range = workbook.worksheet_range(sheet)?;

    let ws_formulas = workbook.worksheet_formula(sheet).ok();

    let cell = range
        .get_value((row, col as u32))
        .unwrap_or(&calamine::Data::Empty);

    let formula = ws_formulas
        .as_ref()
        .and_then(|f| f.get_value((row, col as u32)).map(|s| s.to_string()));

    Ok(data_to_cell_data(cell, formula))
}

/// Original read_range for backward compatibility.
pub fn read_range(path: &str, sheet: &str, range_spec: &str) -> Result<Vec<Vec<CellData>>> {
    let result = read_range_with_options(path, sheet, range_spec, &ReadRangeOptions::default())?;
    match result.data {
        ReadRangeData::Detailed(data) => Ok(data),
        _ => unreachable!("Detailed mode always returns Detailed variant"),
    }
}

/// Read range with advanced output mode options.
pub fn read_range_with_options(
    path: &str,
    sheet: &str,
    range_spec: &str,
    options: &ReadRangeOptions,
) -> Result<ReadRangeResult> {
    let (r_start, r_end, c_start, c_end) = cell_ref::parse_range_normalized(range_spec)?;
    let mut workbook: Xlsx<_> = open_workbook(path)?;

    let range = workbook.worksheet_range(sheet)?;
    let ws_formulas = workbook.worksheet_formula(sheet).ok();

    let total_rows = (r_end - r_start + 1) as usize;
    let total_cols = (c_end - c_start + 1) as usize;

    let effective_rows = match options.truncate {
        Some(trunc) if trunc < total_rows => trunc,
        _ => total_rows,
    };
    let truncated = effective_rows < total_rows;

    let mut raw_data = Vec::new();
    for row in r_start..r_start + effective_rows as u32 {
        let mut row_data = Vec::new();
        for col in c_start..=c_end {
            let cell = range.get_value((row, col as u32)).unwrap_or(&Data::Empty);
            let formula = ws_formulas
                .as_ref()
                .and_then(|f| f.get_value((row, col as u32)).map(|s| s.to_string()));
            row_data.push(data_to_cell_data(cell, formula));
        }
        raw_data.push(row_data);
    }

    let data = match options.mode {
        OutputMode::Detailed => {
            if truncated {
                let marker_row = vec![
                    CellData {
                        value: Some(format!("... ({} more rows)", total_rows - effective_rows)),
                        data_type: CellDataType::String,
                        formula: None,
                    };
                    total_cols
                ];
                raw_data.append(&mut vec![marker_row]);
            }
            ReadRangeData::Detailed(raw_data)
        }
        OutputMode::Compact => {
            let compact = format_compact(&raw_data, r_start, c_start, total_rows, truncated);
            ReadRangeData::Compact(compact)
        }
        OutputMode::Csv => {
            let csv = format_csv(&raw_data, total_rows, truncated);
            ReadRangeData::Csv(csv)
        }
    };

    Ok(ReadRangeResult {
        mode: options.mode.clone(),
        data,
        total_rows,
        total_cols,
        truncated,
    })
}

fn format_compact(
    data: &[Vec<CellData>],
    row_offset: u32,
    col_offset: u16,
    _total_rows: usize,
    _truncated: bool,
) -> Vec<String> {
    data.iter()
        .enumerate()
        .map(|(row_idx, row)| {
            let cells: Vec<String> = row
                .iter()
                .enumerate()
                .map(|(col_idx, cell)| {
                    let col = cell_ref::index_to_col(col_offset + col_idx as u16);
                    let col_ref = format!("{}{}", col, row_offset + row_idx as u32 + 1);
                    let val = match &cell.value {
                        Some(v) => v.clone(),
                        None => String::new(),
                    };
                    format!("{}: {}", col_ref, val)
                })
                .collect();
            cells.join("  ")
        })
        .collect()
}

fn format_csv(data: &[Vec<CellData>], _total_rows: usize, _truncated: bool) -> String {
    let mut wtr = csv::Writer::from_writer(Vec::new());
    for row in data {
        let record: Vec<String> = row
            .iter()
            .map(|cell| match &cell.value {
                Some(v) => v.clone(),
                None => String::new(),
            })
            .collect();
        let _ = wtr.write_record(&record);
    }
    let _ = wtr.flush();
    String::from_utf8(wtr.into_inner().expect("CSV writer should not fail"))
        .expect("CSV output should be valid UTF-8")
}

pub fn read_formula(path: &str, sheet: &str, cell_spec: &str) -> Result<Option<String>> {
    let (row, col) = cell_ref::parse_cell_ref(cell_spec)?;
    let mut workbook: Xlsx<_> = open_workbook(path)?;

    let formulas = workbook.worksheet_formula(sheet)?;

    Ok(formulas.get_value((row, col as u32)).map(|s| {
        let formula = s.to_string();
        // Add = prefix if not present, as calamine stores formulas without it
        if formula.starts_with('=') {
            formula
        } else {
            format!("={}", formula)
        }
    }))
}

pub fn read_sheet_all(path: &str, sheet: &str) -> Result<SheetData> {
    let mut workbook: Xlsx<_> = open_workbook(path)?;

    let range = workbook.worksheet_range(sheet)?;

    let ws_formulas = workbook.worksheet_formula(sheet).ok();

    let mut rows = Vec::new();
    for (row_idx, row) in range.rows().enumerate() {
        let mut cells = Vec::new();
        for (col_idx, cell) in row.iter().enumerate() {
            let formula = ws_formulas.as_ref().and_then(|f| {
                f.get_value((row_idx as u32, col_idx as u32))
                    .map(|s| s.to_string())
            });
            cells.push(data_to_cell_data(cell, formula));
        }
        rows.push(cells);
    }

    Ok(SheetData {
        name: sheet.to_string(),
        rows,
    })
}

pub(crate) fn read_all_sheets_to_map(path: &str) -> Result<HashMap<String, SheetData>> {
    let sheets = list_sheets(path)?;
    let mut map = HashMap::new();
    for name in sheets {
        let data = read_sheet_all(path, &name)?;
        map.insert(name, data);
    }
    Ok(map)
}

fn data_to_cell_data(cell: &Data, formula: Option<String>) -> CellData {
    match cell {
        Data::String(s) => CellData {
            value: Some(s.clone()),
            data_type: CellDataType::String,
            formula,
        },
        Data::Float(f) => CellData {
            value: Some(f.to_string()),
            data_type: CellDataType::Float,
            formula,
        },
        Data::Int(i) => CellData {
            value: Some(i.to_string()),
            data_type: CellDataType::Int,
            formula,
        },
        Data::Bool(b) => CellData {
            value: Some(b.to_string()),
            data_type: CellDataType::Bool,
            formula,
        },
        Data::DateTime(f) => CellData {
            value: Some(f.to_string()),
            data_type: CellDataType::DateTime,
            formula,
        },
        Data::DateTimeIso(s) => CellData {
            value: Some(s.clone()),
            data_type: CellDataType::DateTime,
            formula,
        },
        Data::DurationIso(s) => CellData {
            value: Some(s.clone()),
            data_type: CellDataType::String,
            formula,
        },
        Data::Error(e) => CellData {
            value: Some(format!("{}", e)),
            data_type: CellDataType::Error,
            formula,
        },
        Data::Empty => CellData {
            value: None,
            data_type: CellDataType::Empty,
            formula,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_sheets_nonexistent_file() {
        let result = list_sheets("_nonexistent_file.xlsx");
        assert!(result.is_err());
    }
}
