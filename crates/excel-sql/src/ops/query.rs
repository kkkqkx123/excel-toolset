use excel_types::{AppError, FilterCondition, FilterOp, SheetData};

use crate::converter::QueryResult;
use crate::db::query::{query as db_query, query_with_params};
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

pub fn sql_query_on_data(
    data: &[SheetData],
    sql: &str,
    has_header: bool,
) -> Result<QueryResult, AppError> {
    let db =
        create_conn().map_err(|e| AppError::DuckDb(format!("Failed to create connection: {e}")))?;

    for sheet in data {
        load_sheet_to_db(&db, &sheet.name, sheet, has_header)?;
    }

    db_query(&db, sql)
}

/// Internal impl that accepts an existing connection (used by QuerySession).
pub fn filter_rows_on_data_impl(
    db: &duckdb::Connection,
    sheet: &str,
    conditions: &[FilterCondition],
) -> Result<QueryResult, AppError> {
    if conditions.is_empty() {
        let sql = format!(r#"SELECT * FROM "{}""#, sheet.replace('"', "\"\""));
        return db_query(db, &sql);
    }

    let (where_clause, params) = build_param_conditions(conditions);
    let sql = format!(
        r#"SELECT * FROM "{}" WHERE {}"#,
        sheet.replace('"', "\"\""),
        where_clause
    );

    query_with_params(db, &sql, &params)
}

pub fn filter_rows_on_data(
    data: &SheetData,
    sheet: &str,
    conditions: &[FilterCondition],
    has_header: bool,
) -> Result<QueryResult, AppError> {
    let db =
        create_conn().map_err(|e| AppError::DuckDb(format!("Failed to create connection: {e}")))?;
    load_sheet_to_db(&db, sheet, data, has_header)?;
    filter_rows_on_data_impl(&db, sheet, conditions)
}
