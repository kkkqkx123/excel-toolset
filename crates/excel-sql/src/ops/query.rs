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

        let param_value = match c.operator {
            FilterOp::Eq
            | FilterOp::Ne
            | FilterOp::Gt
            | FilterOp::Lt
            | FilterOp::Ge
            | FilterOp::Le => {
                if let Ok(num) = c.value.parse::<i64>() {
                    duckdb::types::Value::BigInt(num)
                } else if let Ok(num) = c.value.parse::<f64>() {
                    duckdb::types::Value::Double(num)
                } else {
                    duckdb::types::Value::Text(c.value.clone())
                }
            }
            _ => duckdb::types::Value::Text(value),
        };

        params.push(param_value);
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

#[cfg(test)]
mod tests {
    use super::*;
    use excel_types::CellData;
    use excel_types::CellDataType::*;

    fn make_cell(value: Option<&str>, dt: excel_types::CellDataType) -> CellData {
        CellData {
            value: value.map(|s| s.to_string()),
            data_type: dt,
            formula: None,
        }
    }

    fn sample_data() -> SheetData {
        SheetData {
            name: "t".to_string(),
            rows: vec![
                vec![make_cell(Some("1"), Int), make_cell(Some("a"), String)],
                vec![make_cell(Some("2"), Int), make_cell(Some("b"), String)],
                vec![make_cell(Some("3"), Int), make_cell(Some("c"), String)],
            ],
        }
    }

    mod build_param_conditions_tests {
        use super::*;

        #[test]
        fn test_eq() {
            let cond = FilterCondition {
                column: 0,
                operator: FilterOp::Eq,
                value: "10".into(),
            };
            let (clause, params) = build_param_conditions(&[cond]);
            assert_eq!(clause, r#""c0" = ?1"#);
            assert_eq!(params, vec![duckdb::types::Value::Text("10".into())]);
        }

        #[test]
        fn test_ne() {
            let cond = FilterCondition {
                column: 1,
                operator: FilterOp::Ne,
                value: "x".into(),
            };
            let (clause, _) = build_param_conditions(&[cond]);
            assert_eq!(clause, r#""c1" != ?1"#);
        }

        #[test]
        fn test_comparison_ops() {
            let ops = [
                (FilterOp::Gt, ">"),
                (FilterOp::Lt, "<"),
                (FilterOp::Ge, ">="),
                (FilterOp::Le, "<="),
            ];
            for (op, expected) in ops {
                let cond = FilterCondition {
                    column: 0,
                    operator: op,
                    value: "5".into(),
                };
                let (clause, _) = build_param_conditions(&[cond]);
                assert_eq!(clause, format!(r#""c0" {} ?1"#, expected));
            }
        }

        #[test]
        fn test_like_ops() {
            let cases = [
                (FilterOp::Contains, "abc", "%abc%"),
                (FilterOp::StartsWith, "abc", "abc%"),
                (FilterOp::EndsWith, "abc", "%abc"),
            ];
            for (op, val, expected_pat) in cases {
                let cond = FilterCondition {
                    column: 0,
                    operator: op,
                    value: val.into(),
                };
                let (clause, params) = build_param_conditions(&[cond]);
                assert_eq!(clause, r#""c0" LIKE ?1"#);
                assert_eq!(
                    params,
                    vec![duckdb::types::Value::Text(expected_pat.into())]
                );
            }
        }

        #[test]
        fn test_multiple_conditions() {
            let conds = vec![
                FilterCondition {
                    column: 0,
                    operator: FilterOp::Eq,
                    value: "1".into(),
                },
                FilterCondition {
                    column: 1,
                    operator: FilterOp::Contains,
                    value: "a".into(),
                },
            ];
            let (clause, params) = build_param_conditions(&conds);
            assert_eq!(clause, r#""c0" = ?1 AND "c1" LIKE ?2"#);
            assert_eq!(params.len(), 2);
        }
    }

    #[test]
    fn test_sql_query_on_data() {
        let data = vec![SheetData {
            name: "s1".to_string(),
            rows: vec![vec![
                make_cell(Some("k"), String),
                make_cell(Some("v"), String),
            ]],
        }];
        let result = sql_query_on_data(&data, "SELECT * FROM \"s1\"", false).unwrap();
        assert_eq!(result.row_count, 1);
        assert_eq!(result.rows[0][0].value.as_deref(), Some("k"));
    }

    #[test]
    fn test_filter_rows_on_data_empty_conditions() {
        let data = sample_data();
        let result = filter_rows_on_data(&data, "t", &[], false).unwrap();
        assert_eq!(result.row_count, 3);
    }

    #[test]
    fn test_filter_rows_on_data_with_condition() {
        let data = sample_data();
        let cond = FilterCondition {
            column: 0,
            operator: FilterOp::Gt,
            value: "1".into(),
        };
        let result = filter_rows_on_data(&data, "t", &[cond], false).unwrap();
        assert_eq!(result.row_count, 2);
    }
}
