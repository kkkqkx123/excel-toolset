use crate::converter::QueryResult;
use excel_types::{AppError, CellData, CellDataType};

/// Converting DuckDB values back to CellData
pub fn duckdb_to_cell(value: &duckdb::types::Value) -> CellData {
    match value {
        duckdb::types::Value::Null => CellData {
            value: None,
            data_type: CellDataType::Empty,
            formula: None,
        },
        duckdb::types::Value::Boolean(b) => CellData {
            value: Some(if *b { "true" } else { "false" }.to_string()),
            data_type: CellDataType::Bool,
            formula: None,
        },
        duckdb::types::Value::TinyInt(n) => CellData {
            value: Some(n.to_string()),
            data_type: CellDataType::Int,
            formula: None,
        },
        duckdb::types::Value::SmallInt(n) => CellData {
            value: Some(n.to_string()),
            data_type: CellDataType::Int,
            formula: None,
        },
        duckdb::types::Value::Int(n) => CellData {
            value: Some(n.to_string()),
            data_type: CellDataType::Int,
            formula: None,
        },
        duckdb::types::Value::BigInt(n) => CellData {
            value: Some(n.to_string()),
            data_type: CellDataType::Int,
            formula: None,
        },
        duckdb::types::Value::HugeInt(n) => CellData {
            value: Some(n.to_string()),
            data_type: CellDataType::Int,
            formula: None,
        },
        duckdb::types::Value::UTinyInt(n) => CellData {
            value: Some(n.to_string()),
            data_type: CellDataType::Int,
            formula: None,
        },
        duckdb::types::Value::USmallInt(n) => CellData {
            value: Some(n.to_string()),
            data_type: CellDataType::Int,
            formula: None,
        },
        duckdb::types::Value::UInt(n) => CellData {
            value: Some(n.to_string()),
            data_type: CellDataType::Int,
            formula: None,
        },
        duckdb::types::Value::UBigInt(n) => CellData {
            value: Some(n.to_string()),
            data_type: CellDataType::Int,
            formula: None,
        },
        duckdb::types::Value::Float(f) => CellData {
            value: Some(f.to_string()),
            data_type: CellDataType::Float,
            formula: None,
        },
        duckdb::types::Value::Double(f) => CellData {
            value: Some(f.to_string()),
            data_type: CellDataType::Float,
            formula: None,
        },
        duckdb::types::Value::Decimal(d) => CellData {
            value: Some(d.to_string()),
            data_type: CellDataType::Float,
            formula: None,
        },
        duckdb::types::Value::Date32(d) => CellData {
            value: Some(d.to_string()),
            data_type: CellDataType::DateTime,
            formula: None,
        },
        duckdb::types::Value::Time64(_, ts) => CellData {
            value: Some(ts.to_string()),
            data_type: CellDataType::DateTime,
            formula: None,
        },
        duckdb::types::Value::Timestamp(_, ts) => CellData {
            value: Some(ts.to_string()),
            data_type: CellDataType::DateTime,
            formula: None,
        },
        duckdb::types::Value::Interval {
            months,
            days,
            nanos,
        } => CellData {
            value: Some(format!("{months}m {days}d {nanos}ns")),
            data_type: CellDataType::String,
            formula: None,
        },
        duckdb::types::Value::Text(s) => CellData {
            value: Some(s.clone()),
            data_type: CellDataType::String,
            formula: None,
        },
        duckdb::types::Value::Blob(_) => CellData {
            value: Some("BLOB".to_string()),
            data_type: CellDataType::String,
            formula: None,
        },
        duckdb::types::Value::Enum(s) => CellData {
            value: Some(s.clone()),
            data_type: CellDataType::String,
            formula: None,
        },
        duckdb::types::Value::Array(_) => CellData {
            value: Some("ARRAY".to_string()),
            data_type: CellDataType::String,
            formula: None,
        },
        duckdb::types::Value::Struct(_) => CellData {
            value: Some("STRUCT".to_string()),
            data_type: CellDataType::String,
            formula: None,
        },
        duckdb::types::Value::Map(_) => CellData {
            value: Some("MAP".to_string()),
            data_type: CellDataType::String,
            formula: None,
        },
        duckdb::types::Value::Union(_) => CellData {
            value: Some("UNION".to_string()),
            data_type: CellDataType::String,
            formula: None,
        },
        duckdb::types::Value::List(_) => CellData {
            value: Some("LIST".to_string()),
            data_type: CellDataType::String,
            formula: None,
        },
    }
}

/// Execute a SQL query and return the results
pub fn query(db: &duckdb::Connection, sql: &str) -> Result<QueryResult, AppError> {
    let mut stmt = db
        .prepare(sql)
        .map_err(|e| AppError::DuckDb(e.to_string()))?;

    let col_count = stmt.column_count();
    let mut columns = Vec::with_capacity(col_count);
    for i in 0..col_count {
        columns.push(stmt.column_name(i).unwrap_or("").to_string());
    }

    let rows = stmt
        .query_map([], |row| {
            let mut cells = Vec::with_capacity(col_count);
            for i in 0..col_count {
                let value: duckdb::types::Value = row.get(i).unwrap_or(duckdb::types::Value::Null);
                cells.push(duckdb_to_cell(&value));
            }
            Ok(cells)
        })
        .map_err(|e| AppError::DuckDb(e.to_string()))?;

    let mut result_rows = Vec::new();
    for row in rows {
        result_rows.push(row.map_err(|e| AppError::DuckDb(e.to_string()))?);
    }

    let row_count = result_rows.len();
    Ok(QueryResult {
        columns,
        rows: result_rows,
        row_count,
    })
}

/// Executing SQL Queries with Parameters
pub fn query_with_params(
    db: &duckdb::Connection,
    sql: &str,
    params: &[duckdb::types::Value],
) -> Result<QueryResult, AppError> {
    let mut stmt = db
        .prepare(sql)
        .map_err(|e| AppError::DuckDb(e.to_string()))?;

    let col_count = stmt.column_count();
    let mut columns = Vec::with_capacity(col_count);
    for i in 0..col_count {
        columns.push(stmt.column_name(i).unwrap_or("").to_string());
    }

    let rows = stmt
        .query_map(duckdb::params_from_iter(params.iter()), |row| {
            let mut cells = Vec::with_capacity(col_count);
            for i in 0..col_count {
                let value: duckdb::types::Value = row.get(i).unwrap_or(duckdb::types::Value::Null);
                cells.push(duckdb_to_cell(&value));
            }
            Ok(cells)
        })
        .map_err(|e| AppError::DuckDb(e.to_string()))?;

    let mut result_rows = Vec::new();
    for row in rows {
        result_rows.push(row.map_err(|e| AppError::DuckDb(e.to_string()))?);
    }

    let row_count = result_rows.len();
    Ok(QueryResult {
        columns,
        rows: result_rows,
        row_count,
    })
}

/// Query and return the original string (simplified format)
pub fn query_to_strings(
    db: &duckdb::Connection,
    sql: &str,
) -> Result<(Vec<String>, Vec<Vec<String>>), AppError> {
    let result = query(db, sql)?;

    let column_names = result.columns;
    let string_rows: Vec<Vec<String>> = result
        .rows
        .iter()
        .map(|row| {
            row.iter()
                .map(|cell| cell.value.clone().unwrap_or_default())
                .collect()
        })
        .collect();

    Ok((column_names, string_rows))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::conn::create_conn;
    use crate::db::loader::create_table;

    fn make_cell(value: Option<&str>, dt: CellDataType) -> CellData {
        CellData {
            value: value.map(|s| s.to_string()),
            data_type: dt,
            formula: None,
        }
    }

    fn seed_db(conn: &duckdb::Connection) {
        create_table(conn, "t", &[CellDataType::Int, CellDataType::String]).unwrap();
        let rows = vec![
            vec![
                make_cell(Some("1"), CellDataType::Int),
                make_cell(Some("a"), CellDataType::String),
            ],
            vec![
                make_cell(Some("2"), CellDataType::Int),
                make_cell(Some("b"), CellDataType::String),
            ],
        ];
        crate::db::loader::batch_insert_rows(conn, "t", &rows).unwrap();
    }

    #[test]
    fn test_query_basic() {
        let conn = create_conn().unwrap();
        seed_db(&conn);
        let result = query(&conn, "SELECT * FROM \"t\" ORDER BY c0").unwrap();
        assert_eq!(result.columns, vec!["c0", "c1"]);
        assert_eq!(result.row_count, 2);
    }

    #[test]
    fn test_query_with_empty_result() {
        let conn = create_conn().unwrap();
        conn.execute_batch(r#"CREATE TABLE "e" (c0 INTEGER)"#)
            .unwrap();
        let result = query(&conn, "SELECT * FROM \"e\"").unwrap();
        assert_eq!(result.row_count, 0);
        assert_eq!(result.rows.len(), 0);
    }

    #[test]
    fn test_query_with_params() {
        let conn = create_conn().unwrap();
        seed_db(&conn);
        let params = [duckdb::types::Value::BigInt(1)];
        let result =
            query_with_params(&conn, "SELECT * FROM \"t\" WHERE c0 = ?1", &params).unwrap();
        assert_eq!(result.row_count, 1);
        assert_eq!(result.rows[0][0].value.as_deref(), Some("1"));
    }

    #[test]
    fn test_query_to_strings() {
        let conn = create_conn().unwrap();
        seed_db(&conn);
        let (cols, rows) = query_to_strings(&conn, "SELECT * FROM \"t\" ORDER BY c0").unwrap();
        assert_eq!(cols, vec!["c0", "c1"]);
        assert_eq!(rows, vec![vec!["1", "a"], vec!["2", "b"]]);
    }

    #[test]
    fn test_duckdb_to_cell_all_variants() {
        use duckdb::types::Value;
        // Null
        assert_eq!(duckdb_to_cell(&Value::Null).data_type, CellDataType::Empty);
        // Boolean
        assert_eq!(
            duckdb_to_cell(&Value::Boolean(true)).data_type,
            CellDataType::Bool
        );
        // Integer types
        assert_eq!(
            duckdb_to_cell(&Value::TinyInt(1)).data_type,
            CellDataType::Int
        );
        assert_eq!(
            duckdb_to_cell(&Value::SmallInt(1)).data_type,
            CellDataType::Int
        );
        assert_eq!(duckdb_to_cell(&Value::Int(1)).data_type, CellDataType::Int);
        assert_eq!(
            duckdb_to_cell(&Value::BigInt(1)).data_type,
            CellDataType::Int
        );
        assert_eq!(
            duckdb_to_cell(&Value::HugeInt(1i128.into())).data_type,
            CellDataType::Int
        );
        assert_eq!(
            duckdb_to_cell(&Value::UTinyInt(1)).data_type,
            CellDataType::Int
        );
        assert_eq!(
            duckdb_to_cell(&Value::USmallInt(1)).data_type,
            CellDataType::Int
        );
        assert_eq!(duckdb_to_cell(&Value::UInt(1)).data_type, CellDataType::Int);
        assert_eq!(
            duckdb_to_cell(&Value::UBigInt(1)).data_type,
            CellDataType::Int
        );
        // Float types
        assert_eq!(
            duckdb_to_cell(&Value::Float(1.5)).data_type,
            CellDataType::Float
        );
        assert_eq!(
            duckdb_to_cell(&Value::Double(2.5)).data_type,
            CellDataType::Float
        );
        // Decimal: skip - rust_decimal is not a direct dep; Float + Double cover float path
        // Text
        assert_eq!(
            duckdb_to_cell(&Value::Text("hi".into())).data_type,
            CellDataType::String
        );
        assert_eq!(
            duckdb_to_cell(&Value::Text("hi".into())).value,
            Some("hi".to_string())
        );
        // Complex types → String
        assert_eq!(
            duckdb_to_cell(&Value::Blob(vec![1])).data_type,
            CellDataType::String
        );
        assert_eq!(
            duckdb_to_cell(&Value::Enum("x".into())).data_type,
            CellDataType::String
        );
        assert_eq!(
            duckdb_to_cell(&Value::Array(vec![])).data_type,
            CellDataType::String
        );
    }
}
