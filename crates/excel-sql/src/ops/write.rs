use std::cell::RefCell;

use excel_core::excel_data;
use excel_core::excel_read;
use excel_core::types::*;

use crate::db::{create_conn, load_sheet_with_row_id};

pub fn sort_sheet(
    path: &str,
    params: &SecurityParams,
    sheet: &str,
    sort_columns: &[SortColumn],
) -> Result<WriteResult> {
    let old_sheet_data = excel_read::read_sheet_all(path, sheet)?;
    let new_sheet_data: RefCell<Option<SheetData>> = RefCell::new(None);

    let mut result = excel_data::modify_data_file(path, params, |old_data| {
        let mut new_data = old_data.clone();
        let sd = new_data
            .get_mut(sheet)
            .ok_or_else(|| AppError::SheetNotFound(sheet.into()))?;

        if sd.rows.len() <= 1 {
            *new_sheet_data.borrow_mut() = Some(SheetData {
                name: sheet.to_string(),
                rows: sd.rows.clone(),
            });
            return Ok(new_data);
        }

        let db = create_conn().map_err(|e| AppError::DuckDb(e.to_string()))?;
        load_sheet_with_row_id(&db, sheet, sd, false).map_err(|e| AppError::DuckDb(e.to_string()))?;

        let order_clauses: Vec<String> = sort_columns
            .iter()
            .map(|sc| {
                let dir = if sc.descending { "DESC" } else { "ASC" };
                format!(r#""c{}" {}"#, sc.column, dir)
            })
            .collect();

        let sql = format!(
            r#"SELECT row_id FROM "{}" ORDER BY {}"#,
            sheet.replace('"', "\"\""),
            order_clauses.join(", ")
        );

        let mut stmt = db.prepare(&sql).map_err(|e| AppError::DuckDb(e.to_string()))?;
        let ids: Vec<usize> = stmt
            .query_map([], |row| row.get::<_, i64>(0).map(|v| v as usize))
            .map_err(|e| AppError::DuckDb(e.to_string()))?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| AppError::DuckDb(e.to_string()))?;

        let original_rows = std::mem::take(&mut sd.rows);
        for &id in &ids {
            if id < original_rows.len() {
                sd.rows.push(original_rows[id].clone());
            }
        }

        *new_sheet_data.borrow_mut() = Some(SheetData {
            name: sheet.to_string(),
            rows: sd.rows.clone(),
        });

        Ok(new_data)
    })?;

    if let Some(new_sd) = new_sheet_data.into_inner() {
        let cell_diffs = excel_diff::compute_diffs(&old_sheet_data, &new_sd);
        if !cell_diffs.is_empty() {
            let sheet_diff = SheetDiff {
                sheet_name: sheet.to_string(),
                row_count_diff: new_sd.rows.len() as i32 - old_sheet_data.rows.len() as i32,
                col_count_diff: 0,
                cell_diffs,
            };
            let summary = excel_diff::summarize::summarize(std::slice::from_ref(&sheet_diff));
            result.diff = Some(FileDiff {
                file_hash_match: result.old_hash == result.new_hash,
                sheet_diffs: vec![sheet_diff],
                summary,
            });
        }
    }

    Ok(result)
}

pub fn dedup_sheet(
    path: &str,
    params: &SecurityParams,
    sheet: &str,
    columns: &[u16],
) -> Result<WriteResult> {
    let old_sheet_data = excel_read::read_sheet_all(path, sheet)?;
    let new_sheet_data: RefCell<Option<SheetData>> = RefCell::new(None);

    let mut result = excel_data::modify_data_file(path, params, |old_data| {
        let mut new_data = old_data.clone();
        let sd = new_data
            .get_mut(sheet)
            .ok_or_else(|| AppError::SheetNotFound(sheet.into()))?;

        if sd.rows.len() <= 1 {
            *new_sheet_data.borrow_mut() = Some(SheetData {
                name: sheet.to_string(),
                rows: sd.rows.clone(),
            });
            return Ok(new_data);
        }

        let header = sd.rows[0].clone();
        let data_rows: Vec<Vec<CellData>> = sd.rows.drain(1..).collect();

        let db = create_conn().map_err(|e| AppError::DuckDb(e.to_string()))?;
        load_sheet_with_row_id(&db, sheet, &SheetData {
            name: sheet.to_string(),
            rows: data_rows.clone(),
        }, false).map_err(|e| AppError::DuckDb(e.to_string()))?;

        let partition_cols: Vec<String> = if columns.is_empty() {
            let max_cols = data_rows.iter().map(|r| r.len()).max().unwrap_or(0);
            (0..max_cols).map(|i| format!(r#""c{i}""#)).collect()
        } else {
            columns.iter().map(|c| format!(r#""c{c}""#)).collect()
        };

        let sub_sql = format!(
            r#"SELECT row_id FROM "{}" ORDER BY {}"#,
            sheet.replace('"', "\"\""),
            partition_cols.join(", ")
        );
        let partition_sql = format!(
            r#"SELECT DISTINCT ON ({}) row_id FROM ({}) sub ORDER BY {}, row_id"#,
            partition_cols.join(", "),
            sub_sql,
            partition_cols.join(", ")
        );

        let mut stmt = db.prepare(&partition_sql).map_err(|e| AppError::DuckDb(e.to_string()))?;
        let deduped_ids: Vec<usize> = stmt
            .query_map([], |row| row.get::<_, i64>(0).map(|v| v as usize))
            .map_err(|e| AppError::DuckDb(e.to_string()))?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| AppError::DuckDb(e.to_string()))?;

        let mut new_rows = vec![header];
        for &id in &deduped_ids {
            if id < data_rows.len() {
                new_rows.push(data_rows[id].clone());
            }
        }
        sd.rows = new_rows;

        *new_sheet_data.borrow_mut() = Some(SheetData {
            name: sheet.to_string(),
            rows: sd.rows.clone(),
        });

        Ok(new_data)
    })?;

    if let Some(new_sd) = new_sheet_data.into_inner() {
        let cell_diffs = excel_diff::compute_diffs(&old_sheet_data, &new_sd);
        if !cell_diffs.is_empty() {
            let sheet_diff = SheetDiff {
                sheet_name: sheet.to_string(),
                row_count_diff: new_sd.rows.len() as i32 - old_sheet_data.rows.len() as i32,
                col_count_diff: 0,
                cell_diffs,
            };
            let summary = excel_diff::summarize::summarize(std::slice::from_ref(&sheet_diff));
            result.diff = Some(FileDiff {
                file_hash_match: result.old_hash == result.new_hash,
                sheet_diffs: vec![sheet_diff],
                summary,
            });
        }
    }

    Ok(result)
}
