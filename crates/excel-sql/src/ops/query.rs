use excel_types::{AppError, CellData, CellDataType, FilterCondition, FilterOp, SheetData};

use crate::converter::QueryResult;
use crate::db::{create_conn, load_sheet_to_db};

fn build_param_conditions(conditions: &[FilterCondition]) -> (String, Vec<duckdb::types::Value>) {
    let mut clauses = Vec::new();
    let mut params = Vec::new();

    for (i, c) in conditions.iter().enumerate() {
        let col = format!("c{}", c.column);
        let placeholder = format!("?{}", i + 1);
        let (clause, value) = match c.operator {
            FilterOp::Eq => (format!("\"{col}\" = {placeholder}"), c.value.clone()),
            FilterOp::Ne => (format!("\"{col}\" != {placeholder}"), c.value.clone()),
            FilterOp::Gt => (format!("\"{col}\" > {placeholder}"), c.value.clone()),
            FilterOp::Lt => (format!("\"{col}\" < {placeholder}"), c.value.clone()),
            FilterOp::Ge => (format!("\"{col}\" >= {placeholder}"), c.value.clone()),
            FilterOp::Le => (format!("\"{col}\" <= {placeholder}"), c.value.clone()),
            FilterOp::Contains => {
                let pattern = format!("%{}%", c.value);
                (format!("\"{col}\" LIKE {placeholder}"), pattern)
            }
            FilterOp::StartsWith => {
                let pattern = format!("{}%", c.value);
                (format!("\"{col}\" LIKE {placeholder}"), pattern)
            }
            FilterOp::EndsWith => {
                let pattern = format!("%{}", c.value);
                (format!("\"{col}\" LIKE {placeholder}"), pattern)
            }
        };
        clauses.push(clause);
        params.push(duckdb::types::Value::Text(value));
    }

    (clauses.join(" AND "), params)
}

fn query_to_query_result(
    stmt: &mut duckdb::Statement,
) -> Result<QueryResult, AppError> {
    let col_count = stmt.column_count();
    let mut columns = Vec::with_capacity(col_count);
    for i in 0..col_count {
        columns.push(stmt.column_name(i).map_or("", |v| v).to_string());
    }

    let rows = stmt
        .query_map([], |row| {
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

pub fn sql_query_on_data(
    data: &[SheetData],
    sql: &str,
    has_header: bool,
) -> Result<QueryResult, AppError> {
    let db = create_conn().map_err(|e| AppError::DuckDb(e.to_string()))?;

    for sheet in data {
        load_sheet_to_db(&db, &sheet.name, sheet, has_header)?;
    }

    let mut stmt = db.prepare(sql).map_err(|e| AppError::DuckDb(e.to_string()))?;
    query_to_query_result(&mut stmt)
}

pub fn filter_rows_on_data(
    data: &SheetData,
    sheet: &str,
    conditions: &[FilterCondition],
    has_header: bool,
) -> Result<QueryResult, AppError> {
    let db = create_conn().map_err(|e| AppError::DuckDb(e.to_string()))?;

    load_sheet_to_db(&db, sheet, data, has_header)?;

    if conditions.is_empty() {
        let sql = format!(
            r#"SELECT * FROM "{}""#,
            sheet.replace('"', "\"\"")
        );
        let mut stmt = db.prepare(&sql).map_err(|e| AppError::DuckDb(e.to_string()))?;
        return query_to_query_result(&mut stmt);
    }

    let (where_clause, params) = build_param_conditions(conditions);
    let sql = format!(
        r#"SELECT * FROM "{}" WHERE {}"#,
        sheet.replace('"', "\"\""),
        where_clause
    );

    let mut stmt = db.prepare(&sql).map_err(|e| AppError::DuckDb(e.to_string()))?;
    let col_count = stmt.column_count();
    let mut columns = Vec::with_capacity(col_count);
    for i in 0..col_count {
        columns.push(stmt.column_name(i).map_or("", |v| v).to_string());
    }

    let rows = stmt
        .query_map(duckdb::params_from_iter(params.iter()), |row| {
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