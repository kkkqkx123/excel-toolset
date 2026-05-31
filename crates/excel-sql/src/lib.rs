use excel_core::excel_read;
use excel_core::types::*;

fn map_duckdb_err(e: duckdb::Error) -> AppError {
    AppError::DuckDb(e.to_string())
}

fn load_sheet_to_db(
    db: &duckdb::Connection,
    name: &str,
    data: &SheetData,
) -> std::result::Result<(), duckdb::Error> {
    let max_cols = data.rows.iter().map(|r| r.len()).max().unwrap_or(0);
    if max_cols == 0 {
        return Ok(());
    }

    let col_defs: Vec<String> = (0..max_cols).map(|i| format!("c{} VARCHAR", i)).collect();
    let escaped_name = name.replace('"', "\"\"");
    db.execute_batch(&format!(
        r#"CREATE TABLE "{}" ({})"#,
        escaped_name,
        col_defs.join(", ")
    ))?;

    for row in &data.rows {
        let values: Vec<String> = (0..max_cols)
            .map(|i| {
                row.get(i)
                    .and_then(|c| c.value.as_deref())
                    .map(|v| format!("'{}'", v.replace('\'', "''")))
                    .unwrap_or_else(|| "NULL".to_string())
            })
            .collect();
        let sql = format!(
            r#"INSERT INTO "{}" VALUES ({})"#,
            escaped_name,
            values.join(", ")
        );
        db.execute_batch(&sql)?;
    }
    Ok(())
}

fn load_sheet_with_row_id(
    db: &duckdb::Connection,
    name: &str,
    data: &SheetData,
) -> std::result::Result<(), duckdb::Error> {
    let max_cols = data.rows.iter().map(|r| r.len()).max().unwrap_or(0);
    if max_cols == 0 {
        return Ok(());
    }

    let col_defs: Vec<String> = std::iter::once("row_id INTEGER".to_string())
        .chain((0..max_cols).map(|i| format!("c{} VARCHAR", i)))
        .collect();
    let escaped_name = name.replace('"', "\"\"");
    db.execute_batch(&format!(
        r#"CREATE TABLE "{}" ({})"#,
        escaped_name,
        col_defs.join(", ")
    ))?;

    for (idx, row) in data.rows.iter().enumerate() {
        let values: Vec<String> = std::iter::once(idx.to_string())
            .chain((0..max_cols).map(|i| {
                row.get(i)
                    .and_then(|c| c.value.as_deref())
                    .map(|v| format!("'{}'", v.replace('\'', "''")))
                    .unwrap_or_else(|| "NULL".to_string())
            }))
            .collect();
        let sql = format!(
            r#"INSERT INTO "{}" VALUES ({})"#,
            escaped_name,
            values.join(", ")
        );
        db.execute_batch(&sql)?;
    }
    Ok(())
}

fn query_to_cell_data(
    stmt: &mut duckdb::Statement,
) -> std::result::Result<Vec<Vec<CellData>>, duckdb::Error> {
    let col_count = stmt.column_count();
    let rows = stmt.query_map([], |row| {
        let mut cells = Vec::with_capacity(col_count);
        for i in 0..col_count {
            let val: Option<String> = row.get(i).ok().flatten();
            cells.push(CellData {
                value: val,
                data_type: CellDataType::String,
                formula: None,
            });
        }
        Ok(cells)
    })?;

    let mut result = Vec::new();
    for row in rows {
        result.push(row?);
    }
    Ok(result)
}

fn condition_to_sql(c: &FilterCondition) -> String {
    let col = format!("c{}", c.column);
    let val = c.value.replace('\'', "''");
    match c.operator {
        FilterOp::Eq => format!(r#""{}" = '{}'"#, col, val),
        FilterOp::Ne => format!(r#""{}" != '{}'"#, col, val),
        FilterOp::Gt => format!(r#""{}" > '{}'"#, col, val),
        FilterOp::Lt => format!(r#""{}" < '{}'"#, col, val),
        FilterOp::Ge => format!(r#""{}" >= '{}'"#, col, val),
        FilterOp::Le => format!(r#""{}" <= '{}'"#, col, val),
        FilterOp::Contains => {
            format!(r#""{}" LIKE '%{}%'"#, col, val)
        }
        FilterOp::StartsWith => {
            format!(r#""{}" LIKE '{}%'"#, col, val)
        }
        FilterOp::EndsWith => {
            format!(r#""{}" LIKE '%{}'"#, col, val)
        }
    }
}

// ====== Query operations (read-only) ======

/// Execute an arbitrary SQL query on an Excel file.
/// Each sheet is loaded as a table named by the sheet name.
/// SQL can reference sheets as tables: `SELECT * FROM "Sheet1" WHERE "c0" > '100'`
pub fn sql_query(path: &str, sql: &str) -> Result<Vec<Vec<CellData>>> {
    let sheets = excel_read::list_sheets(path)?;
    let db = duckdb::Connection::open_in_memory().map_err(map_duckdb_err)?;

    for sheet in &sheets {
        let data = excel_read::read_sheet_all(path, sheet)?;
        load_sheet_to_db(&db, sheet, &data).map_err(map_duckdb_err)?;
    }

    let mut stmt = db.prepare(sql).map_err(map_duckdb_err)?;
    query_to_cell_data(&mut stmt).map_err(map_duckdb_err)
}

/// Filter rows using DuckDB-powered SQL WHERE clause.
pub fn filter_rows(
    path: &str,
    sheet: &str,
    conditions: &[FilterCondition],
) -> Result<Vec<Vec<CellData>>> {
    let data = excel_read::read_sheet_all(path, sheet)?;
    let db = duckdb::Connection::open_in_memory().map_err(map_duckdb_err)?;

    load_sheet_to_db(&db, sheet, &data).map_err(map_duckdb_err)?;

    let where_clauses: Vec<String> = conditions.iter().map(condition_to_sql).collect();
    let sql = format!(
        r#"SELECT * FROM "{}" WHERE {}"#,
        sheet,
        where_clauses.join(" AND ")
    );

    let mut stmt = db.prepare(&sql).map_err(map_duckdb_err)?;
    query_to_cell_data(&mut stmt).map_err(map_duckdb_err)
}

// ====== Write operations (via excel-core modify_data_file) ======

/// Sort sheet data using DuckDB ORDER BY internally.
/// Formulas are preserved by reordering original `CellData` rows based on DuckDB-sorted row ids.
pub fn sort_sheet(
    path: &str,
    params: &SecurityParams,
    sheet: &str,
    sort_columns: &[SortColumn],
) -> Result<WriteResult> {
    excel_core::excel_data::modify_data_file(path, params, |old_data| {
        let mut new_data = old_data.clone();
        let sd = new_data
            .get_mut(sheet)
            .ok_or_else(|| AppError::SheetNotFound(sheet.into()))?;

        if sd.rows.len() <= 1 {
            return Ok(new_data);
        }

        let db = duckdb::Connection::open_in_memory().map_err(map_duckdb_err)?;
        load_sheet_with_row_id(&db, sheet, sd).map_err(map_duckdb_err)?;

        let order_clauses: Vec<String> = sort_columns
            .iter()
            .map(|sc| {
                let dir = if sc.descending { "DESC" } else { "ASC" };
                format!(r#""c{}" {}"#, sc.column, dir)
            })
            .collect();

        let sql = format!(
            r#"SELECT row_id FROM "{}" ORDER BY {}"#,
            sheet,
            order_clauses.join(", ")
        );

        let mut stmt = db.prepare(&sql).map_err(map_duckdb_err)?;
        let sorted_ids: Vec<usize> = stmt
            .query_map([], |row| row.get::<_, i64>(0).map(|v| v as usize))
            .map_err(map_duckdb_err)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| AppError::DuckDb(e.to_string()))?;

        let original_rows = std::mem::take(&mut sd.rows);
        for &id in &sorted_ids {
            if id < original_rows.len() {
                sd.rows.push(original_rows[id].clone());
            }
        }

        Ok(new_data)
    })
}

/// Deduplicate sheet data using DuckDB DISTINCT ON internally.
/// Formulas are preserved by reordering original `CellData` rows based on DuckDB-deduplicated row ids.
pub fn dedup_sheet(
    path: &str,
    params: &SecurityParams,
    sheet: &str,
    columns: &[u16],
) -> Result<WriteResult> {
    excel_core::excel_data::modify_data_file(path, params, |old_data| {
        let mut new_data = old_data.clone();
        let sd = new_data
            .get_mut(sheet)
            .ok_or_else(|| AppError::SheetNotFound(sheet.into()))?;

        if sd.rows.len() <= 1 {
            return Ok(new_data);
        }

        let db = duckdb::Connection::open_in_memory().map_err(map_duckdb_err)?;
        load_sheet_with_row_id(&db, sheet, sd).map_err(map_duckdb_err)?;

        let partition_cols: Vec<String> = if columns.is_empty() {
            // Dedup across all value columns
            let max_cols = sd.rows.iter().map(|r| r.len()).max().unwrap_or(0);
            (0..max_cols).map(|i| format!(r#""c{}""#, i)).collect()
        } else {
            columns.iter().map(|c| format!(r#""c{}""#, c)).collect()
        };

        let sql = format!(
            r#"SELECT row_id FROM "{}" ORDER BY {}"#,
            sheet,
            partition_cols.join(", ")
        );

        let partition_sql = format!(
            r#"SELECT DISTINCT ON ({}) row_id FROM ({}) sub ORDER BY {}, row_id"#,
            partition_cols.join(", "),
            sql,
            partition_cols.join(", ")
        );

        let mut stmt = db.prepare(&partition_sql).map_err(map_duckdb_err)?;
        let deduped_ids: Vec<usize> = stmt
            .query_map([], |row| row.get::<_, i64>(0).map(|v| v as usize))
            .map_err(map_duckdb_err)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| AppError::DuckDb(e.to_string()))?;

        let header = sd.rows[0].clone();
        let original_body: Vec<Vec<CellData>> = sd.rows.drain(1..).collect();
        let mut deduped_body = Vec::new();
        for &id in &deduped_ids {
            if id > 0 && (id - 1) < original_body.len() {
                deduped_body.push(original_body[id - 1].clone());
            }
        }

        sd.rows.push(header);
        sd.rows.extend(deduped_body);

        Ok(new_data)
    })
}
