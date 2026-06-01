use crate::excel_data;
use crate::excel_read;
use crate::security;
use crate::types::*;
use rust_xlsxwriter::{Workbook, Worksheet};

fn compute_cell_diffs(old: &SheetData, new: &SheetData) -> Vec<CellDiff> {
    let max_rows = old.rows.len().max(new.rows.len());
    let mut diffs = Vec::new();
    for r in 0..max_rows {
        let old_row = old.rows.get(r);
        let new_row = new.rows.get(r);
        let max_cols = old_row
            .map(|r| r.len())
            .unwrap_or(0)
            .max(new_row.map(|r| r.len()).unwrap_or(0));
        for c in 0..max_cols {
            let old_cell = old_row.and_then(|r| r.get(c));
            let new_cell = new_row.and_then(|r| r.get(c));
            let old_val = old_cell.and_then(|c| c.value.as_deref());
            let new_val = new_cell.and_then(|c| c.value.as_deref());
            let old_fml = old_cell.and_then(|c| c.formula.as_deref());
            let new_fml = new_cell.and_then(|c| c.formula.as_deref());
            if old_val != new_val || old_fml != new_fml {
                diffs.push(CellDiff {
                    row: r as u32,
                    col: c as u16,
                    cell_ref: format!("R{}C{}", r + 1, c + 1),
                    diff_type: if old_cell.is_none() {
                        DiffType::Add
                    } else if new_cell.is_none() {
                        DiffType::Delete
                    } else {
                        DiffType::Modify
                    },
                    old_value: old_val.map(String::from),
                    new_value: new_val.map(String::from),
                    old_formula: old_fml.map(String::from),
                    new_formula: new_fml.map(String::from),
                });
            }
        }
    }
    diffs
}

fn make_diff_summary(diffs: &[CellDiff]) -> DiffSummary {
    let mut summary = DiffSummary {
        adds: 0,
        deletes: 0,
        modifies: 0,
        passives: 0,
        total_changes: diffs.len(),
    };
    for d in diffs {
        match d.diff_type {
            DiffType::Add => summary.adds += 1,
            DiffType::Delete => summary.deletes += 1,
            DiffType::Modify => summary.modifies += 1,
            DiffType::Passive => summary.passives += 1,
            DiffType::NoChange => {}
        }
    }
    summary
}

fn write_workbook(path: &str, data: &std::collections::HashMap<String, SheetData>) -> Result<()> {
    let mut wb = Workbook::new();
    for (name, sd) in data {
        let ws = wb.add_worksheet();
        ws.set_name(name).map_err(AppError::Xlsx)?;
        excel_data::write_sheet_data(ws, sd)?;
    }
    wb.save(path).map_err(AppError::Xlsx)?;
    Ok(())
}

#[cfg(not(feature = "sql"))]
pub fn filter_rows_dispatch(
    path: &str,
    sheet: &str,
    conditions: &[FilterCondition],
) -> Result<Vec<Vec<CellData>>> {
    excel_data::filter_rows(path, sheet, conditions)
}

#[cfg(feature = "sql")]
pub fn filter_rows_dispatch(
    path: &str,
    sheet: &str,
    conditions: &[FilterCondition],
) -> Result<Vec<Vec<CellData>>> {
    let data = excel_read::read_sheet_all(path, sheet)?;
    let result = excel_sql::filter_rows_on_data(&data, sheet, conditions, true)?;
    Ok(result.rows)
}

#[cfg(not(feature = "sql"))]
pub fn sort_sheet_dispatch(
    path: &str,
    params: &SecurityParams,
    sheet: &str,
    sort_columns: &[SortColumn],
) -> Result<WriteResult> {
    excel_data::sort_sheet(path, params, sheet, sort_columns)
}

#[cfg(feature = "sql")]
pub fn sort_sheet_dispatch(
    path: &str,
    params: &SecurityParams,
    sheet: &str,
    sort_columns: &[SortColumn],
) -> Result<WriteResult> {
    let old_data = excel_read::read_all_sheets_to_map(path)?;
    let old_sheet = old_data
        .get(sheet)
        .ok_or_else(|| AppError::SheetNotFound(sheet.into()))?
        .clone();

    let old_hash = security::compute_file_hash(path).map_err(AppError::Io)?;
    let backup_info = if params.create_backup {
        Some(security::create_backup(path, &old_hash).map_err(AppError::Io)?)
    } else {
        None
    };

    let new_sheet = excel_sql::sort_sheet_on_data(&old_sheet, sort_columns)?;

    let new_hash = if params.dry_run {
        old_hash.clone()
    } else {
        let mut new_data = old_data.clone();
        new_data.insert(sheet.to_string(), new_sheet.clone());
        write_workbook(path, &new_data)?;
        security::compute_file_hash(path).map_err(AppError::Io)?
    };

    let cell_diffs = compute_cell_diffs(&old_sheet, &new_sheet);
    let diff = if cell_diffs.is_empty() {
        None
    } else {
        let sheet_diff = SheetDiff {
            sheet_name: sheet.to_string(),
            row_count_diff: new_sheet.rows.len() as i32 - old_sheet.rows.len() as i32,
            col_count_diff: 0,
            cell_diffs,
        };
        let summary = make_diff_summary(&sheet_diff.cell_diffs);
        Some(FileDiff {
            file_hash_match: old_hash == new_hash,
            sheet_diffs: vec![sheet_diff],
            summary,
        })
    };

    Ok(WriteResult {
        success: true,
        message: String::new(),
        backup_info,
        old_hash,
        new_hash,
        diff,
    })
}

#[cfg(not(feature = "sql"))]
pub fn dedup_sheet_dispatch(
    path: &str,
    params: &SecurityParams,
    sheet: &str,
    columns: &[u16],
) -> Result<WriteResult> {
    excel_data::dedup_sheet(path, params, sheet, columns)
}

#[cfg(feature = "sql")]
pub fn dedup_sheet_dispatch(
    path: &str,
    params: &SecurityParams,
    sheet: &str,
    columns: &[u16],
) -> Result<WriteResult> {
    let old_data = excel_read::read_all_sheets_to_map(path)?;
    let old_sheet = old_data
        .get(sheet)
        .ok_or_else(|| AppError::SheetNotFound(sheet.into()))?
        .clone();

    let old_hash = security::compute_file_hash(path).map_err(AppError::Io)?;
    let backup_info = if params.create_backup {
        Some(security::create_backup(path, &old_hash).map_err(AppError::Io)?)
    } else {
        None
    };

    let new_sheet = excel_sql::dedup_sheet_on_data(&old_sheet, columns)?;

    let new_hash = if params.dry_run {
        old_hash.clone()
    } else {
        let mut new_data = old_data.clone();
        new_data.insert(sheet.to_string(), new_sheet.clone());
        write_workbook(path, &new_data)?;
        security::compute_file_hash(path).map_err(AppError::Io)?
    };

    let cell_diffs = compute_cell_diffs(&old_sheet, &new_sheet);
    let diff = if cell_diffs.is_empty() {
        None
    } else {
        let sheet_diff = SheetDiff {
            sheet_name: sheet.to_string(),
            row_count_diff: new_sheet.rows.len() as i32 - old_sheet.rows.len() as i32,
            col_count_diff: 0,
            cell_diffs,
        };
        let summary = make_diff_summary(&sheet_diff.cell_diffs);
        Some(FileDiff {
            file_hash_match: old_hash == new_hash,
            sheet_diffs: vec![sheet_diff],
            summary,
        })
    };

    Ok(WriteResult {
        success: true,
        message: String::new(),
        backup_info,
        old_hash,
        new_hash,
        diff,
    })
}

#[cfg(feature = "sql")]
pub fn sql_query_dispatch(path: &str, _sheet: &str, query: &str) -> Result<Vec<Vec<CellData>>> {
    let data = excel_read::read_all_sheets_to_map(path)?;
    let sheets: Vec<SheetData> = data.into_values().collect();
    let result = excel_sql::sql_query_on_data(&sheets, query, true)?;
    Ok(result.rows)
}

#[cfg(not(feature = "sql"))]
pub fn sql_query_dispatch(_path: &str, _sheet: &str, _query: &str) -> Result<Vec<Vec<CellData>>> {
    Err(AppError::FeatureNotEnabled(
        "SQL queries require the 'sql' feature (enable with --features sql)".into(),
    ))
}
