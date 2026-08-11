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

    // calamine returns Some("") for non-formula cells; treat empty as "no formula"
    // so we never emit an empty <f></f> element on write-back.
    let formula = ws_formulas.as_ref().and_then(|f| {
        f.get_value((row, col as u32))
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
    });

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
            // Empty string means "not a formula" in calamine.
            let formula = ws_formulas.as_ref().and_then(|f| {
                f.get_value((row, col as u32))
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string())
            });
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

    Ok(formulas.get_value((row, col as u32)).and_then(|s| {
        let formula = s.to_string();
        // calamine yields Some("") for plain cells - that is not a formula.
        if formula.is_empty() {
            return None;
        }
        // Add = prefix if not present, as calamine stores formulas without it
        if formula.starts_with('=') {
            Some(formula)
        } else {
            Some(format!("={}", formula))
        }
    }))
}

pub fn read_sheet_all(path: &str, sheet: &str) -> Result<SheetData> {
    let mut workbook: Xlsx<_> = open_workbook(path)?;

    let range = workbook.worksheet_range(sheet)?;
    let ws_formulas = workbook.worksheet_formula(sheet).ok();

    // `worksheet_range` returns the *value* grid, and calamine crops it to the
    // cells that actually hold values. A formula cell with **no cached value**
    // (very common: files written by openpyxl, LibreOffice with "never
    // calculate", or our own `formula set` without `--eval`) lives ONLY in the
    // formula grid. Iterating just the value grid therefore silently dropped
    // formula-only cells: `diff file` missed formula changes on single-row
    // sheets and `diff formula-deps` always returned an empty graph.
    //
    // Fix: iterate the UNION of both grids' bounds, reading each cell's value
    // from the value grid and its formula from the formula grid.
    let v_start = range.start().unwrap_or((0, 0));
    // An empty sheet has height == width == 0; `- 1` would underflow to
    // u32::MAX and spawn a ~4-billion-row loop that OOMs the process
    // (observed as rc=137 / SIGKILL on every write of a workbook containing an
    // empty sheet). saturating arithmetic keeps empty ranges harmless.
    let v_end = (
        v_start
            .0
            .saturating_add(range.height() as u32)
            .saturating_sub(1),
        v_start
            .1
            .saturating_add(range.width() as u32)
            .saturating_sub(1),
    );
    let (f_start, f_end) = match &ws_formulas {
        Some(f) => {
            let s = f.start().unwrap_or((0, 0));
            (
                s,
                (
                    s.0.saturating_add(f.height() as u32).saturating_sub(1),
                    s.1.saturating_add(f.width() as u32).saturating_sub(1),
                ),
            )
        }
        None => ((0, 0), (0, 0)),
    };
    let end_row = v_end.0.max(f_end.0);
    let end_col = v_end.1.max(f_end.1);

    // Absolute coordinates: `rows[r][c]` is the cell at row r / column c with
    // A1 == (0, 0). Consumers (write_sheet_data, data_mut::write, diff, formula
    // evaluation) all index `rows[row][col]` as if the grid started at A1, so
    // pushing any non-A1 origin into the grid keeps "index == coordinate" true
    // everywhere and prevents silent data shifting on write-back.
    let mut rows: Vec<Vec<CellData>> = Vec::with_capacity(end_row as usize + 1);
    for row in 0..=end_row {
        let mut cells: Vec<CellData> = Vec::with_capacity(end_col as usize + 1);
        for col in 0..=end_col {
            let value = range.get_value((row, col)).unwrap_or(&Data::Empty);
            let formula = ws_formulas.as_ref().and_then(|f| {
                f.get_value((row, col))
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string())
            });
            cells.push(data_to_cell_data(value, formula));
        }
        rows.push(cells);
    }

    Ok(SheetData {
        name: sheet.to_string(),
        rows,
        // The grid is absolute; consumers must not re-apply any origin.
        start_row: 0,
        start_col: 0,
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
