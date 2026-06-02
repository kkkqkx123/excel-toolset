use excel_types::{AppError, CellData, SheetData, SortColumn};

use crate::db::{create_conn, load_sheet_with_row_id};

fn sort_ids(
    db: &duckdb::Connection,
    sheet_name: &str,
    sort_columns: &[SortColumn],
) -> Result<Vec<usize>, AppError> {
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

    let mut stmt = db
        .prepare(&sql)
        .map_err(|e| AppError::DuckDb(format!("Failed to prepare sort query: {e}")))?;
    stmt.query_map([], |row| row.get::<_, i64>(0).map(|v| v as usize))
        .map_err(|e| AppError::DuckDb(format!("Failed to execute sort: {e}")))?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(|e| AppError::DuckDb(format!("Failed to collect sort results: {e}")))
}

fn dedup_ids(
    db: &duckdb::Connection,
    sheet_name: &str,
    columns: &[u16],
    max_cols: usize,
) -> Result<Vec<usize>, AppError> {
    let partition_cols: Vec<String> = if columns.is_empty() {
        (0..max_cols).map(|i| format!(r#""c{i}""#)).collect()
    } else {
        columns.iter().map(|c| format!(r#""c{c}""#)).collect()
    };

    let partition_spec = partition_cols.join(", ");
    let escaped_name = sheet_name.replace('"', "\"\"");
    let sql = format!(
        r#"SELECT row_id FROM (
            SELECT row_id, ROW_NUMBER() OVER (PARTITION BY {partition_spec} ORDER BY row_id) AS rn
            FROM "{escaped_name}"
        ) WHERE rn = 1"#
    );

    let mut stmt = db
        .prepare(&sql)
        .map_err(|e| AppError::DuckDb(format!("Failed to prepare dedup query: {e}")))?;
    stmt.query_map([], |row| row.get::<_, i64>(0).map(|v| v as usize))
        .map_err(|e| AppError::DuckDb(format!("Failed to execute dedup: {e}")))?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(|e| AppError::DuckDb(format!("Failed to collect dedup results: {e}")))
}

/// Internal: sort using an existing connection (data will be loaded into it).
pub fn sort_sheet_on_data_impl(
    db: &mut duckdb::Connection,
    data: &SheetData,
    sort_columns: &[SortColumn],
) -> Result<SheetData, AppError> {
    if data.rows.len() <= 1 {
        return Ok(data.clone());
    }

    let sheet_name = &data.name;
    let original_rows = data.rows.clone();

    let tx = db
        .transaction()
        .map_err(|e| AppError::DuckDb(format!("Failed to start transaction: {e}")))?;

    load_sheet_with_row_id(
        &tx,
        sheet_name,
        &SheetData {
            name: sheet_name.clone(),
            rows: original_rows.clone(),
        },
        false,
    )
    .map_err(|e| AppError::DuckDb(format!("Failed to load data for sort: {e}")))?;

    let ids = sort_ids(&tx, sheet_name, sort_columns)?;

    tx.commit()
        .map_err(|e| AppError::DuckDb(format!("Failed to commit transaction: {e}")))?;

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

pub fn sort_sheet_on_data(
    data: &SheetData,
    sort_columns: &[SortColumn],
) -> Result<SheetData, AppError> {
    let mut db =
        create_conn().map_err(|e| AppError::DuckDb(format!("Failed to create connection: {e}")))?;
    sort_sheet_on_data_impl(&mut db, data, sort_columns)
}

/// Internal: dedup using an existing connection (data will be loaded into it).
pub fn dedup_sheet_on_data_impl(
    db: &mut duckdb::Connection,
    data: &SheetData,
    columns: &[u16],
) -> Result<SheetData, AppError> {
    if data.rows.len() <= 1 {
        return Ok(data.clone());
    }

    let sheet_name = &data.name;
    let header = data.rows[0].clone();
    let data_rows: Vec<Vec<CellData>> = data.rows[1..].to_vec();

    let tx = db
        .transaction()
        .map_err(|e| AppError::DuckDb(format!("Failed to start transaction: {e}")))?;

    load_sheet_with_row_id(
        &tx,
        sheet_name,
        &SheetData {
            name: sheet_name.clone(),
            rows: data_rows.clone(),
        },
        false,
    )
    .map_err(|e| AppError::DuckDb(format!("Failed to load data for dedup: {e}")))?;

    let max_cols = data_rows.iter().map(|r| r.len()).max().unwrap_or(0);
    let deduped_ids = dedup_ids(&tx, sheet_name, columns, max_cols)?;

    tx.commit()
        .map_err(|e| AppError::DuckDb(format!("Failed to commit transaction: {e}")))?;

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

pub fn dedup_sheet_on_data(data: &SheetData, columns: &[u16]) -> Result<SheetData, AppError> {
    let mut db =
        create_conn().map_err(|e| AppError::DuckDb(format!("Failed to create connection: {e}")))?;
    dedup_sheet_on_data_impl(&mut db, data, columns)
}
