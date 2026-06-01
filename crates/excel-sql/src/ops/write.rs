use excel_types::{AppError, CellData, SheetData, SortColumn};

use crate::db::{create_conn, load_sheet_with_row_id};

pub fn sort_sheet_on_data(
    data: &SheetData,
    sort_columns: &[SortColumn],
) -> Result<SheetData, AppError> {
    if data.rows.len() <= 1 {
        return Ok(data.clone());
    }

    let sheet_name = &data.name;
    let original_rows = data.rows.clone();
    let db = create_conn().map_err(|e| AppError::DuckDb(e.to_string()))?;
    load_sheet_with_row_id(
        &db,
        sheet_name,
        &SheetData {
            name: sheet_name.clone(),
            rows: original_rows.clone(),
        },
        false,
    )
    .map_err(|e| AppError::DuckDb(e.to_string()))?;

    let order_clauses: Vec<String> = sort_columns
        .iter()
        .map(|sc| {
            let dir = if sc.descending { "DESC" } else { "ASC" };
            format!(r#""c{}" {}"#, sc.column, dir)
        })
        .collect();

    let sql = format!(
        r#"SELECT row_id FROM "{}" ORDER BY {}"#,
        sheet_name.replace('"', "\"\""),
        order_clauses.join(", ")
    );

    let mut stmt = db.prepare(&sql).map_err(|e| AppError::DuckDb(e.to_string()))?;
    let ids: Vec<usize> = stmt
        .query_map([], |row| row.get::<_, i64>(0).map(|v| v as usize))
        .map_err(|e| AppError::DuckDb(e.to_string()))?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(|e| AppError::DuckDb(e.to_string()))?;

    let mut sorted_rows = Vec::with_capacity(ids.len());
    for &id in &ids {
        if id < original_rows.len() {
            sorted_rows.push(original_rows[id].clone());
        }
    }

    Ok(SheetData {
        name: sheet_name.clone(),
        rows: sorted_rows,
    })
}

pub fn dedup_sheet_on_data(
    data: &SheetData,
    columns: &[u16],
) -> Result<SheetData, AppError> {
    if data.rows.len() <= 1 {
        return Ok(data.clone());
    }

    let sheet_name = &data.name;
    let header = data.rows[0].clone();
    let data_rows: Vec<Vec<CellData>> = data.rows[1..].to_vec();

    let db = create_conn().map_err(|e| AppError::DuckDb(e.to_string()))?;
    load_sheet_with_row_id(
        &db,
        sheet_name,
        &SheetData {
            name: sheet_name.clone(),
            rows: data_rows.clone(),
        },
        false,
    )
    .map_err(|e| AppError::DuckDb(e.to_string()))?;

    let partition_cols: Vec<String> = if columns.is_empty() {
        let max_cols = data_rows.iter().map(|r| r.len()).max().unwrap_or(0);
        (0..max_cols).map(|i| format!(r#""c{i}""#)).collect()
    } else {
        columns.iter().map(|c| format!(r#""c{c}""#)).collect()
    };

    let sub_sql = format!(
        r#"SELECT row_id FROM "{}" ORDER BY {}"#,
        sheet_name.replace('"', "\"\""),
        partition_cols.join(", ")
    );
    let partition_sql = format!(
        r#"SELECT DISTINCT ON ({}) row_id FROM ({}) sub ORDER BY {}, row_id"#,
        partition_cols.join(", "),
        sub_sql,
        partition_cols.join(", ")
    );

    let mut stmt = db
        .prepare(&partition_sql)
        .map_err(|e| AppError::DuckDb(e.to_string()))?;
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

    Ok(SheetData {
        name: sheet_name.clone(),
        rows: new_rows,
    })
}
