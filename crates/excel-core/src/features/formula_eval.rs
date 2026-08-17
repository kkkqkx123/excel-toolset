//! Formula evaluation integration with excel-formula engine.
//!
//! Bridges the formula evaluation engine to excel-core's write pipeline.
//! When a formula is set via the write pipeline, this module optionally
//! evaluates it and stores the computed result alongside the formula string.

use std::collections::HashMap;

use calamine::Reader;
use excel_formula::{DataProvider, FormulaEngine};
use excel_types::CellValue;

use crate::types::*;
use crate::utils::cell_ref;

/// Evaluate a formula for a single cell without modifying the file.
///
/// This reads data from the file for resolution of cell references,
/// evaluates the formula, and returns the computed result.
pub fn evaluate_formula(path: &str, sheet: &str, cell: &str, formula: &str) -> Result<CellValue> {
    let provider = FileDataProvider::open(path)?;
    let engine = FormulaEngine::new(provider);
    engine
        .evaluate(sheet, cell, formula)
        .map(|r| r.to_cell_value())
        .map_err(|e| AppError::Custom(format!("Formula evaluation error: {}", e)))
}

/// A DataProvider that reads from an xlsx file using calamine.
struct FileDataProvider {
    cells: HashMap<String, Vec<Vec<CellValue>>>,
}

impl FileDataProvider {
    fn open(path: &str) -> Result<Self> {
        let mut workbook: calamine::Xlsx<_> = calamine::open_workbook(path)
            .map_err(|e: calamine::XlsxError| AppError::Read(e.to_string()))?;

        let sheet_names = workbook.sheet_names().to_vec();
        let mut cells: HashMap<String, Vec<Vec<CellValue>>> = HashMap::new();

        for sheet_name in sheet_names {
            let range =
                workbook
                    .worksheet_range(&sheet_name)
                    .map_err(|e: calamine::XlsxError| {
                        AppError::Read(format!("Cannot read sheet '{}': {}", sheet_name, e))
                    })?;

            // `range.rows()` yields *relative* indices within the used range.
            // A sheet whose data starts at, say, C5 would otherwise make `A1`
            // resolve to C5's value and every formula reference would be
            // silently off by the origin. Pad back to absolute coordinates.
            let (origin_row, origin_col) = range.start().unwrap_or((0, 0));

            let mut sheet_cells: Vec<Vec<CellValue>> = vec![Vec::new(); origin_row as usize];
            for row in range.rows() {
                let mut row_values: Vec<CellValue> = vec![CellValue::Empty; origin_col as usize];
                row_values.extend(row.iter().map(|cell| match cell {
                    calamine::Data::Float(f) => CellValue::Number(*f),
                    calamine::Data::Int(i) => CellValue::Number(*i as f64),
                    calamine::Data::String(s) => CellValue::String(s.clone()),
                    calamine::Data::Bool(b) => CellValue::Bool(*b),
                    calamine::Data::DateTime(d) => CellValue::Number(d.as_f64()),
                    calamine::Data::Error(e) => CellValue::Error(e.to_string()),
                    calamine::Data::Empty => CellValue::Empty,
                    calamine::Data::DateTimeIso(_) => CellValue::Empty,
                    calamine::Data::DurationIso(_) => CellValue::Empty,
                }));
                sheet_cells.push(row_values);
            }
            cells.insert(sheet_name, sheet_cells);
        }

        Ok(Self { cells })
    }
}

impl DataProvider for FileDataProvider {
    fn get_cell(&self, sheet: &str, row: u32, col: u32) -> Option<CellValue> {
        self.cells
            .get(sheet)?
            .get(row as usize)?
            .get(col as usize)
            .cloned()
    }

    fn get_range(
        &self,
        sheet: &str,
        start_row: u32,
        start_col: u32,
        end_row: u32,
        end_col: u32,
    ) -> Vec<Vec<CellValue>> {
        let sheet_data = match self.cells.get(sheet) {
            Some(d) => d,
            None => return vec![],
        };

        let mut result = Vec::new();
        for r in start_row..=end_row {
            let mut row_data = Vec::new();
            for c in start_col..=end_col {
                let val = sheet_data
                    .get(r as usize)
                    .and_then(|row| row.get(c as usize))
                    .cloned()
                    .unwrap_or(CellValue::Empty);
                row_data.push(val);
            }
            result.push(row_data);
        }
        result
    }

    fn cell_exists(&self, sheet: &str, row: u32, col: u32) -> bool {
        self.cells
            .get(sheet)
            .and_then(|d| d.get(row as usize))
            .and_then(|r| r.get(col as usize))
            .is_some()
    }
}

/// Human-readable rendering of an evaluated value, for result messages.
fn cell_value_display(v: &CellValue) -> String {
    match v {
        CellValue::Number(n) => format!("{}", n),
        CellValue::String(s) => s.clone(),
        CellValue::Bool(b) => if *b { "TRUE" } else { "FALSE" }.to_string(),
        CellValue::Error(e) => e.clone(),
        _ => "(empty)".to_string(),
    }
}

/// Convert a CellData to CellValue.
///
/// A formula cell resolves to its **cached value**, not to the formula text.
/// Returning `CellValue::String("SUM(A1:A3)")` made every dependent formula
/// operate on a string, so `=B1*2` on a formula cell produced garbage.
fn cell_data_to_value(cd: &CellData) -> CellValue {
    match &cd.value {
        Some(val) => match cd.data_type {
            CellDataType::Float | CellDataType::Int => val
                .parse::<f64>()
                .map(CellValue::Number)
                .unwrap_or_else(|_| CellValue::String(val.clone())),
            CellDataType::Bool => CellValue::Bool(val.to_lowercase() == "true"),
            CellDataType::DateTime => CellValue::String(val.clone()),
            CellDataType::Error => CellValue::Error(val.clone()),
            _ => CellValue::String(val.clone()),
        },
        None => CellValue::Empty,
    }
}

/// Set a formula in a cell and optionally evaluate it.
///
/// If `evaluate` is true (default), the formula is evaluated and the
/// computed result is written to the cell alongside the formula string.
///
/// Returns the WriteResult with the formula evaluation result if available.
pub fn set_formula_with_eval(
    path: &str,
    sheet: &str,
    cell: &str,
    formula: &str,
    evaluate: bool,
    params: &SecurityParams,
) -> Result<WriteResult> {
    let formula_clean = formula.strip_prefix('=').unwrap_or(formula);

    // Pre-evaluate the formula if requested. A failure here used to be swallowed
    // by `.ok()`, so `--eval` silently degraded to "write the formula with no
    // cached value" while still reporting success. Surface it instead: the user
    // explicitly asked for evaluation.
    let mut eval_error: Option<String> = None;
    let eval_result = if evaluate {
        match evaluate_formula(path, sheet, cell, formula) {
            Ok(v) => Some(v),
            Err(e) => {
                eval_error = Some(e.to_string());
                None
            }
        }
    } else {
        None
    };

    let (row, col) = cell_ref::parse_cell_ref(cell)?;

    let result = {
        #[cfg(feature = "zip")]
        {
            match &eval_result {
                Some(CellValue::Number(n)) => {
                    crate::excel_write::patch::set_formula_with_value_preserving(
                        path,
                        params,
                        sheet,
                        row,
                        col,
                        formula_clean,
                        &format!("{}", n),
                        CellDataType::Float,
                    )
                }
                Some(CellValue::String(s)) => {
                    crate::excel_write::patch::set_formula_with_value_preserving(
                        path,
                        params,
                        sheet,
                        row,
                        col,
                        formula_clean,
                        s,
                        CellDataType::String,
                    )
                }
                Some(CellValue::Bool(b)) => {
                    crate::excel_write::patch::set_formula_with_value_preserving(
                        path,
                        params,
                        sheet,
                        row,
                        col,
                        formula_clean,
                        if *b { "TRUE" } else { "FALSE" },
                        CellDataType::Bool,
                    )
                }
                Some(CellValue::Error(e)) => {
                    crate::excel_write::patch::set_formula_with_value_preserving(
                        path,
                        params,
                        sheet,
                        row,
                        col,
                        formula_clean,
                        e,
                        CellDataType::Error,
                    )
                }
                _ => crate::excel_write::patch::set_formula_preserving(
                    path,
                    params,
                    sheet,
                    row,
                    col,
                    formula_clean,
                ),
            }
        }

        #[cfg(not(feature = "zip"))]
        modify_file(path, params, |old_data| {
            use crate::excel_write::ensure_dimensions;
            let mut new_data = old_data.clone();
            let sd = new_data
                .get_mut(sheet)
                .ok_or_else(|| AppError::SheetNotFound(sheet.into()))?;

            ensure_dimensions(sd, row as usize, col as usize);

            let cell_data = match &eval_result {
                Some(CellValue::Number(n)) => CellData {
                    value: Some(format!("{}", n)),
                    data_type: CellDataType::Float,
                    formula: Some(formula_clean.to_string()),
                },
                Some(CellValue::String(s)) => CellData {
                    value: Some(s.clone()),
                    data_type: CellDataType::String,
                    formula: Some(formula_clean.to_string()),
                },
                Some(CellValue::Bool(b)) => CellData {
                    value: Some(if *b { "TRUE".into() } else { "FALSE".into() }),
                    data_type: CellDataType::Bool,
                    formula: Some(formula_clean.to_string()),
                },
                Some(CellValue::Error(e)) => CellData {
                    value: Some(e.clone()),
                    data_type: CellDataType::Error,
                    formula: Some(formula_clean.to_string()),
                },
                _ => CellData {
                    value: None,
                    data_type: CellDataType::String,
                    formula: Some(formula_clean.to_string()),
                },
            };

            sd.rows[row as usize][col as usize] = cell_data;
            Ok(new_data)
        })
    };

    let msg = match (&eval_result, &eval_error) {
        (Some(v), _) => format!(
            "Set formula at {}!{}: {} (evaluated, cached value = {})",
            sheet,
            cell,
            formula,
            cell_value_display(v)
        ),
        (None, Some(err)) => format!(
            "Set formula at {}!{}: {} — WARNING: evaluation failed ({}), \
             no cached value was stored; Excel will compute it on open",
            sheet, cell, formula, err
        ),
        (None, None) => format!("Set formula at {}!{}: {}", sheet, cell, formula),
    };

    Ok(WriteResult {
        message: msg,
        ..result?
    })
}

/// Refresh all formulas in a worksheet by re-evaluating them.
pub fn refresh_formulas(path: &str, sheet: &str, params: &SecurityParams) -> Result<WriteResult> {
    use crate::excel_write::modify_file;

    let result = modify_file(path, params, |old_data| {
        let mut new_data = old_data.clone();
        let sd = new_data
            .get_mut(sheet)
            .ok_or_else(|| AppError::SheetNotFound(sheet.into()))?;

        let row_count = sd.rows.len();

        for row_idx in 0..row_count {
            let col_count = sd.rows[row_idx].len();
            for col_idx in 0..col_count {
                let formula_str = match &sd.rows[row_idx][col_idx].formula {
                    Some(f) => f.clone(),
                    None => continue,
                };

                let formula_full = if formula_str.starts_with('=') {
                    formula_str.clone()
                } else {
                    format!("={}", formula_str)
                };

                let cell_ref_str = cell_ref::format_cell_ref(row_idx as u32, col_idx as u16);

                // Rebuild the provider from the *current* sheet state so a
                // formula that depends on a cell refreshed earlier in this pass
                // sees the new value instead of the stale one.
                let provider = SheetDataProvider::from_sheet_data(sheet, sd);
                let engine = FormulaEngine::new(provider);

                let (value_str, data_type) =
                    match engine.evaluate(sheet, &cell_ref_str, &formula_full) {
                        Ok(result) => match result.to_cell_value() {
                            CellValue::Number(n) => (format!("{}", n), CellDataType::Float),
                            CellValue::String(s) => (s, CellDataType::String),
                            CellValue::Bool(b) => (
                                if b { "TRUE".into() } else { "FALSE".into() },
                                CellDataType::Bool,
                            ),
                            CellValue::Error(e) => (e, CellDataType::Error),
                            _ => (String::new(), CellDataType::Empty),
                        },
                        // Unsupported or erroneous formulas keep their previous
                        // cached value rather than being blanked out.
                        Err(_) => continue,
                    };

                sd.rows[row_idx][col_idx] = CellData {
                    value: Some(value_str),
                    data_type,
                    formula: Some(formula_str),
                };
            }
        }

        Ok(new_data)
    })?;

    Ok(WriteResult {
        message: format!("Refreshed formulas in sheet '{}'", sheet),
        ..result
    })
}

/// Simple in-memory DataProvider backed by SheetData.
struct SheetDataProvider {
    cells: Vec<Vec<CellValue>>,
}

impl SheetDataProvider {
    fn from_sheet_data(_sheet: &str, sd: &SheetData) -> Self {
        let cells: Vec<Vec<CellValue>> = sd
            .rows
            .iter()
            .map(|row| row.iter().map(cell_data_to_value).collect())
            .collect();
        Self { cells }
    }
}

impl DataProvider for SheetDataProvider {
    fn get_cell(&self, _sheet: &str, row: u32, col: u32) -> Option<CellValue> {
        self.cells.get(row as usize)?.get(col as usize).cloned()
    }

    fn get_range(
        &self,
        _sheet: &str,
        start_row: u32,
        start_col: u32,
        end_row: u32,
        end_col: u32,
    ) -> Vec<Vec<CellValue>> {
        let mut result = Vec::new();
        for r in start_row..=end_row {
            let mut row_data = Vec::new();
            for c in start_col..=end_col {
                let val = self
                    .cells
                    .get(r as usize)
                    .and_then(|row| row.get(c as usize))
                    .cloned()
                    .unwrap_or(CellValue::Empty);
                row_data.push(val);
            }
            result.push(row_data);
        }
        result
    }

    fn cell_exists(&self, _sheet: &str, row: u32, col: u32) -> bool {
        self.cells
            .get(row as usize)
            .and_then(|r| r.get(col as usize))
            .is_some()
    }
}
