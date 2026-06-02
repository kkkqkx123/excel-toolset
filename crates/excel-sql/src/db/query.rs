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
        columns.push(stmt.column_name(i).map_or("", |v| v).to_string());
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
        columns.push(stmt.column_name(i).map_or("", |v| v).to_string());
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
