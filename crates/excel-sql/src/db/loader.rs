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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::conn::create_conn;
    use crate::db::tables::{get_table_schema, table_exists, table_row_count};

    fn make_cell(value: Option<&str>, dt: CellDataType) -> CellData {
        CellData { value: value.map(|s| s.to_string()), data_type: dt, formula: None }
    }

    #[test]
    fn test_create_table_no_columns() {
        let conn = create_conn().unwrap();
        create_table(&conn, "empty", &[]).unwrap();
        assert!(table_exists(&conn, "empty").unwrap());
        // Table should exist with zero columns
    }

    #[test]
    fn test_create_table_basic() {
        let conn = create_conn().unwrap();
        create_table(&conn, "nums", &[CellDataType::Int, CellDataType::Float]).unwrap();
        let schema = get_table_schema(&conn, "nums").unwrap();
        assert_eq!(schema.len(), 2);
        assert_eq!(schema[0].name, "c0");
        assert_eq!(schema[0].data_type, "INTEGER");
        assert_eq!(schema[1].name, "c1");
        assert_eq!(schema[1].data_type, "DOUBLE");
    }

    #[test]
    fn test_create_table_with_header() {
        let conn = create_conn().unwrap();
        let header = vec![
            make_cell(Some("Name"), CellDataType::String),
            make_cell(Some("Age"), CellDataType::Int),
        ];
        create_table_with_header(
            &conn,
            "people",
            &[CellDataType::String, CellDataType::Int],
            &header,
        )
        .unwrap();
        let schema = get_table_schema(&conn, "people").unwrap();
        assert_eq!(schema[0].name, "Name");
        assert_eq!(schema[1].name, "Age");
    }

    #[test]
    fn test_create_table_with_header_empty_cell_defaults_to_cN() {
        let conn = create_conn().unwrap();
        let header = vec![
            make_cell(Some("Name"), CellDataType::String),
            make_cell(None, CellDataType::Empty),
        ];
        create_table_with_header(
            &conn,
            "t",
            &[CellDataType::String, CellDataType::Int],
            &header,
        )
        .unwrap();
        let schema = get_table_schema(&conn, "t").unwrap();
        assert_eq!(schema[0].name, "Name");
        assert_eq!(schema[1].name, "c1");
    }

    #[test]
    fn test_batch_insert_rows() {
        let conn = create_conn().unwrap();
        create_table(&conn, "t", &[CellDataType::Int, CellDataType::String]).unwrap();
        let rows = vec![
            vec![make_cell(Some("1"), CellDataType::Int), make_cell(Some("a"), CellDataType::String)],
            vec![make_cell(Some("2"), CellDataType::Int), make_cell(Some("b"), CellDataType::String)],
        ];
        batch_insert_rows(&conn, "t", &rows).unwrap();
        assert_eq!(table_row_count(&conn, "t").unwrap(), 2);
    }

    #[test]
    fn test_batch_insert_rows_empty_yields_ok() {
        let conn = create_conn().unwrap();
        create_table(&conn, "t", &[CellDataType::Int]).unwrap();
        batch_insert_rows(&conn, "t", &[]).unwrap();
        assert_eq!(table_row_count(&conn, "t").unwrap(), 0);
    }

    #[test]
    fn test_batch_insert_rows_with_id() {
        let conn = create_conn().unwrap();
        conn.execute_batch(r#"CREATE TABLE "t" (row_id INTEGER, c0 VARCHAR)"#).unwrap();
        let rows = vec![
            vec![make_cell(Some("x"), CellDataType::String)],
            vec![make_cell(Some("y"), CellDataType::String)],
        ];
        batch_insert_rows_with_id(&conn, "t", &rows).unwrap();
        assert_eq!(table_row_count(&conn, "t").unwrap(), 2);
    }

    #[test]
    fn test_load_sheet_to_db_with_header() {
        let conn = create_conn().unwrap();
        let data = SheetData {
            name: "sheet1".to_string(),
            rows: vec![
                vec![make_cell(Some("ColA"), CellDataType::String), make_cell(Some("ColB"), CellDataType::String)],
                vec![make_cell(Some("v1"), CellDataType::String), make_cell(Some("v2"), CellDataType::String)],
            ],
        };
        load_sheet_to_db(&conn, "sheet1", &data, true).unwrap();
        assert_eq!(table_row_count(&conn, "sheet1").unwrap(), 1);
        let schema = get_table_schema(&conn, "sheet1").unwrap();
        assert_eq!(schema[0].name, "ColA");
        assert_eq!(schema[1].name, "ColB");
    }

    #[test]
    fn test_load_sheet_to_db_without_header() {
        let conn = create_conn().unwrap();
        let data = SheetData {
            name: "sheet1".to_string(),
            rows: vec![
                vec![make_cell(Some("1"), CellDataType::Int), make_cell(Some("x"), CellDataType::String)],
            ],
        };
        load_sheet_to_db(&conn, "sheet1", &data, false).unwrap();
        assert_eq!(table_row_count(&conn, "sheet1").unwrap(), 1);
        let schema = get_table_schema(&conn, "sheet1").unwrap();
        assert_eq!(schema[0].name, "c0");
    }

    #[test]
    fn test_load_sheet_to_db_empty_data() {
        let conn = create_conn().unwrap();
        let data = SheetData {
            name: "empty".to_string(),
            rows: vec![],
        };
        load_sheet_to_db(&conn, "empty", &data, true).unwrap();
        assert!(!table_exists(&conn, "empty").unwrap());
    }

    #[test]
    fn test_load_sheet_with_row_id() {
        let conn = create_conn().unwrap();
        let data = SheetData {
            name: "s".to_string(),
            rows: vec![
                vec![make_cell(Some("a"), CellDataType::String)],
                vec![make_cell(Some("b"), CellDataType::String)],
            ],
        };
        load_sheet_with_row_id(&conn, "s", &data, false).unwrap();
        assert_eq!(table_row_count(&conn, "s").unwrap(), 2);
    }
}
