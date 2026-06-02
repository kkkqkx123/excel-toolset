use excel_types::{AppError, CellData, CellDataType, SheetData};

use crate::converter::{
    cell_to_duckdb_type, cell_to_duckdb_value, collect_row_types, infer_column_types,
};
use crate::utils::sanitize_column_name;

pub fn create_table(
    db: &duckdb::Connection,
    name: &str,
    col_types: &[CellDataType],
) -> Result<(), AppError> {
    if col_types.is_empty() {
        return Ok(());
    }
    let col_defs: Vec<String> = col_types
        .iter()
        .enumerate()
        .map(|(i, dt)| format!("c{} {}", i, cell_to_duckdb_type(dt)))
        .collect();
    let escaped_name = name.replace('"', "\"\"");
    let sql = format!(
        r#"CREATE TABLE "{}" ({})"#,
        escaped_name,
        col_defs.join(", ")
    );
    db.execute_batch(&sql)
        .map_err(|e| AppError::DuckDb(e.to_string()))?;
    Ok(())
}

pub fn create_table_with_header(
    db: &duckdb::Connection,
    name: &str,
    col_types: &[CellDataType],
    header: &[CellData],
) -> Result<(), AppError> {
    if col_types.is_empty() {
        return Ok(());
    }
    let col_defs: Vec<String> = col_types
        .iter()
        .enumerate()
        .map(|(i, dt)| {
            let col_name = header
                .get(i)
                .and_then(|c| c.value.as_deref())
                .filter(|v| !v.is_empty())
                .map(sanitize_column_name)
                .unwrap_or_else(|| format!("c{}", i));
            format!(
                "\"{}\" {}",
                col_name.replace('"', "\"\""),
                cell_to_duckdb_type(dt)
            )
        })
        .collect();
    let escaped_name = name.replace('"', "\"\"");
    let sql = format!(
        r#"CREATE TABLE "{}" ({})"#,
        escaped_name,
        col_defs.join(", ")
    );
    db.execute_batch(&sql)
        .map_err(|e| AppError::DuckDb(e.to_string()))?;
    Ok(())
}

pub fn batch_insert_rows(
    db: &duckdb::Connection,
    name: &str,
    rows: &[Vec<CellData>],
) -> Result<(), AppError> {
    if rows.is_empty() {
        return Ok(());
    }

    let max_cols = rows.iter().map(|r| r.len()).max().unwrap_or(0);
    if max_cols == 0 {
        return Ok(());
    }

    let placeholders: Vec<String> = (0..max_cols).map(|i| format!("?{}", i + 1)).collect();
    let escaped_name = name.replace('"', "\"\"");
    let sql = format!(
        r#"INSERT INTO "{}" VALUES ({})"#,
        escaped_name,
        placeholders.join(", ")
    );

    let mut stmt = db
        .prepare(&sql)
        .map_err(|e| AppError::DuckDb(e.to_string()))?;

    for (row_idx, row) in rows.iter().enumerate() {
        let mut params: Vec<duckdb::types::Value> = Vec::with_capacity(max_cols);
        for i in 0..max_cols {
            let v = match row.get(i).map(cell_to_duckdb_value) {
                Some(Ok(v)) => v,
                Some(Err(msg)) => {
                    return Err(AppError::DuckDb(format!(
                        "Row {} col {}: {}",
                        row_idx, i, msg
                    )));
                }
                None => duckdb::types::Value::Null,
            };
            params.push(v);
        }
        stmt.execute(duckdb::params_from_iter(params.iter()))
            .map_err(|e| AppError::DuckDb(e.to_string()))?;
    }

    Ok(())
}

pub fn batch_insert_rows_with_id(
    db: &duckdb::Connection,
    name: &str,
    rows: &[Vec<CellData>],
) -> Result<(), AppError> {
    if rows.is_empty() {
        return Ok(());
    }

    let max_cols = rows.iter().map(|r| r.len()).max().unwrap_or(0);
    if max_cols == 0 {
        return Ok(());
    }

    let placeholders: Vec<String> = std::iter::once("?1".to_string())
        .chain((0..max_cols).map(|i| format!("?{}", i + 2)))
        .collect();
    let escaped_name = name.replace('"', "\"\"");
    let sql = format!(
        r#"INSERT INTO "{}" VALUES ({})"#,
        escaped_name,
        placeholders.join(", ")
    );

    let mut stmt = db
        .prepare(&sql)
        .map_err(|e| AppError::DuckDb(e.to_string()))?;

    for (idx, row) in rows.iter().enumerate() {
        let mut params: Vec<duckdb::types::Value> = vec![duckdb::types::Value::BigInt(idx as i64)];
        for i in 0..max_cols {
            let v = match row.get(i).map(cell_to_duckdb_value) {
                Some(Ok(v)) => v,
                Some(Err(msg)) => {
                    return Err(AppError::DuckDb(format!("Row {} col {}: {}", idx, i, msg)));
                }
                None => duckdb::types::Value::Null,
            };
            params.push(v);
        }
        stmt.execute(duckdb::params_from_iter(params.iter()))
            .map_err(|e| AppError::DuckDb(e.to_string()))?;
    }

    Ok(())
}

pub fn load_sheet_to_db(
    db: &duckdb::Connection,
    name: &str,
    data: &SheetData,
    has_header: bool,
) -> Result<(), AppError> {
    if data.rows.is_empty() {
        return Ok(());
    }

    let type_rows = collect_row_types(&data.rows);
    let col_types = infer_column_types(&type_rows);

    if has_header {
        let header = &data.rows[0];
        create_table_with_header(db, name, &col_types, header)?;
        let data_rows = &data.rows[1..];
        batch_insert_rows(db, name, data_rows)?;
    } else {
        create_table(db, name, &col_types)?;
        batch_insert_rows(db, name, &data.rows)?;
    }

    Ok(())
}

pub fn load_sheet_with_row_id(
    db: &duckdb::Connection,
    name: &str,
    data: &SheetData,
    has_header: bool,
) -> Result<(), AppError> {
    if data.rows.is_empty() {
        return Ok(());
    }

    let rows_to_load: &[Vec<CellData>] = if has_header && !data.rows.is_empty() {
        &data.rows[1..]
    } else {
        &data.rows
    };

    let max_cols = data.rows.iter().map(|r| r.len()).max().unwrap_or(0);
    if max_cols == 0 {
        return Ok(());
    }

    let type_rows = collect_row_types(rows_to_load);
    let col_types = infer_column_types(&type_rows);
    let col_defs: Vec<String> = std::iter::once("row_id INTEGER".to_string())
        .chain((0..max_cols).map(|i| {
            let t = col_types
                .get(i)
                .map(cell_to_duckdb_type)
                .unwrap_or("VARCHAR");
            format!("c{} {t}", i)
        }))
        .collect();
    let escaped_name = name.replace('"', "\"\"");
    let create_sql = format!(
        r#"CREATE TABLE "{}" ({})"#,
        escaped_name,
        col_defs.join(", ")
    );
    db.execute_batch(&create_sql)
        .map_err(|e| AppError::DuckDb(e.to_string()))?;

    batch_insert_rows_with_id(db, name, rows_to_load)?;

    Ok(())
}
