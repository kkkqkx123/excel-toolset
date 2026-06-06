use crate::excel_read;
use crate::types::*;

use super::core::modify_data_file;

#[cfg(feature = "sql")]
use super::diff;
#[cfg(feature = "sql")]
use crate::excel_write::write_sheet_data;
#[cfg(feature = "sql")]
use crate::security;
#[cfg(feature = "sql")]
use rust_xlsxwriter::Workbook;

#[cfg(feature = "sql")]
fn write_workbook(path: &str, data: &std::collections::HashMap<String, SheetData>) -> Result<()> {
    let mut wb = Workbook::new();
    for (name, sd) in data {
        let ws = wb.add_worksheet();
        ws.set_name(name).map_err(AppError::Xlsx)?;
        write_sheet_data(ws, sd)?;
    }
    wb.save(path).map_err(AppError::Xlsx)?;
    Ok(())
}

#[cfg(not(feature = "sql"))]
pub fn filter_rows(
    path: &str,
    sheet: &str,
    conditions: &[FilterCondition],
) -> Result<Vec<Vec<CellData>>> {
    let data = excel_read::read_sheet_all(path, sheet)?;
    let header = data.rows.first().cloned().unwrap_or_default();
    let mut results = vec![header];

    for row in data.rows.iter().skip(1) {
        if matches_all(row, conditions) {
            results.push(row.clone());
        }
    }
    Ok(results)
}

#[cfg(feature = "sql")]
pub fn filter_rows(
    path: &str,
    sheet: &str,
    conditions: &[FilterCondition],
) -> Result<Vec<Vec<CellData>>> {
    let data = excel_read::read_sheet_all(path, sheet)?;
    let result = excel_sql::filter_rows_on_data(&data, sheet, conditions, true)?;
    Ok(result.rows)
}

#[cfg(not(feature = "sql"))]
pub fn sort_sheet(
    path: &str,
    params: &SecurityParams,
    sheet: &str,
    sort_columns: &[SortColumn],
) -> Result<WriteResult> {
    modify_data_file(path, params, |old_data| {
        let mut new_data = old_data.clone();
        let sd = new_data
            .get_mut(sheet)
            .ok_or_else(|| AppError::SheetNotFound(sheet.into()))?;

        if sd.rows.len() > 1 {
            let header = sd.rows[0].clone();
            let mut body: Vec<Vec<CellData>> = sd.rows.drain(1..).collect();

            body.sort_by(|a, b| {
                for sc in sort_columns {
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
        Ok(new_data)
    })
}

#[cfg(feature = "sql")]
pub fn sort_sheet(
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

    let cell_diffs = diff::compute_cell_diffs(&old_sheet, &new_sheet);
    let diff = if cell_diffs.is_empty() {
        None
    } else {
        let sheet_diff = SheetDiff {
            sheet_name: sheet.to_string(),
            row_count_diff: new_sheet.rows.len() as i32 - old_sheet.rows.len() as i32,
            col_count_diff: 0,
            cell_diffs,
        };
        let summary = diff::make_diff_summary(&sheet_diff.cell_diffs);
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
pub fn dedup_sheet(
    path: &str,
    params: &SecurityParams,
    sheet: &str,
    columns: &[u16],
) -> Result<WriteResult> {
    modify_data_file(path, params, |old_data| {
        let mut new_data = old_data.clone();
        let sd = new_data
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
        Ok(new_data)
    })
}

#[cfg(feature = "sql")]
pub fn dedup_sheet(
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

    let cell_diffs = diff::compute_cell_diffs(&old_sheet, &new_sheet);
    let diff = if cell_diffs.is_empty() {
        None
    } else {
        let sheet_diff = SheetDiff {
            sheet_name: sheet.to_string(),
            row_count_diff: new_sheet.rows.len() as i32 - old_sheet.rows.len() as i32,
            col_count_diff: 0,
            cell_diffs,
        };
        let summary = diff::make_diff_summary(&sheet_diff.cell_diffs);
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
pub fn sql_query(path: &str, _sheet: &str, query: &str) -> Result<Vec<Vec<CellData>>> {
    let data = excel_read::read_all_sheets_to_map(path)?;
    let sheets: Vec<SheetData> = data.into_values().collect();
    let result = excel_sql::sql_query_on_data(&sheets, query, true)?;
    Ok(result.rows)
}

#[cfg(not(feature = "sql"))]
pub fn sql_query(_path: &str, _sheet: &str, _query: &str) -> Result<Vec<Vec<CellData>>> {
    Err(AppError::FeatureNotEnabled(
        "SQL queries require the 'sql' feature (enable with --features sql)".into(),
    ))
}

#[cfg(not(feature = "sql"))]
fn matches_all(row: &[CellData], conditions: &[FilterCondition]) -> bool {
    conditions.iter().all(|c| matches_one(row, c))
}

#[cfg(not(feature = "sql"))]
fn matches_one(row: &[CellData], cond: &FilterCondition) -> bool {
    let cell_val = row
        .get(cond.column as usize)
        .and_then(|c| c.value.as_deref())
        .unwrap_or("");
    let lower_val = cell_val.to_lowercase();
    let lower_cond = cond.value.to_lowercase();

    match cond.operator {
        FilterOp::Eq => lower_val == lower_cond,
        FilterOp::Ne => lower_val != lower_cond,
        FilterOp::Gt => lower_val > lower_cond,
        FilterOp::Lt => lower_val < lower_cond,
        FilterOp::Ge => lower_val >= lower_cond,
        FilterOp::Le => lower_val <= lower_cond,
        FilterOp::Contains => lower_val.contains(&lower_cond),
        FilterOp::StartsWith => lower_val.starts_with(&lower_cond),
        FilterOp::EndsWith => lower_val.ends_with(&lower_cond),
    }
}
