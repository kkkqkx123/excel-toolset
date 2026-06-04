use excel_types::{CellData, CellDataType};

#[derive(Debug, Clone)]
pub struct QueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<CellData>>,
    pub row_count: usize,
}

pub fn cell_to_duckdb_value(cell: &CellData) -> Result<duckdb::types::Value, String> {
    match cell.data_type {
        CellDataType::Empty | CellDataType::Error => Ok(duckdb::types::Value::Null),

        CellDataType::Int => {
            if let Some(value_str) = cell.value.as_deref() {
                value_str
                    .parse::<i64>()
                    .map(duckdb::types::Value::BigInt)
                    .map_err(|e| format!("Failed to parse '{}' as Int: {}", value_str, e))
            } else {
                Ok(duckdb::types::Value::Null)
            }
        }

        CellDataType::Float => {
            if let Some(value_str) = cell.value.as_deref() {
                value_str
                    .parse::<f64>()
                    .map(duckdb::types::Value::Double)
                    .map_err(|e| format!("Failed to parse '{}' as Float: {}", value_str, e))
            } else {
                Ok(duckdb::types::Value::Null)
            }
        }

        CellDataType::Bool => {
            let b = cell
                .value
                .as_deref()
                .is_some_and(|v| matches!(v.to_lowercase().as_str(), "true" | "1" | "yes"));
            Ok(duckdb::types::Value::Boolean(b))
        }

        CellDataType::DateTime | CellDataType::String => Ok(cell
            .value
            .as_deref()
            .map(|v| duckdb::types::Value::Text(v.to_string()))
            .unwrap_or(duckdb::types::Value::Null)),
    }
}

pub fn collect_row_types(data: &[Vec<CellData>]) -> Vec<Vec<CellDataType>> {
    data.iter()
        .map(|row| row.iter().map(|c| c.data_type.clone()).collect())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_cell(value: Option<&str>, dt: CellDataType) -> CellData {
        CellData {
            value: value.map(|s| s.to_string()),
            data_type: dt,
            formula: None,
        }
    }

    mod cell_to_duckdb_value_tests {
        use super::*;

        #[test]
        fn test_empty_and_error_to_null() {
            let empty = make_cell(None, CellDataType::Empty);
            assert_eq!(cell_to_duckdb_value(&empty).unwrap(), duckdb::types::Value::Null);

            let err = make_cell(Some("#REF!"), CellDataType::Error);
            assert_eq!(cell_to_duckdb_value(&err).unwrap(), duckdb::types::Value::Null);
        }

        #[test]
        fn test_int_parsing() {
            let valid = make_cell(Some("42"), CellDataType::Int);
            assert_eq!(
                cell_to_duckdb_value(&valid).unwrap(),
                duckdb::types::Value::BigInt(42)
            );

            let null_val = make_cell(None, CellDataType::Int);
            assert_eq!(
                cell_to_duckdb_value(&null_val).unwrap(),
                duckdb::types::Value::Null
            );
        }

        #[test]
        fn test_int_parse_error() {
            let bad = make_cell(Some("not_a_number"), CellDataType::Int);
            assert!(cell_to_duckdb_value(&bad).is_err());
        }

        #[test]
        fn test_float_parsing() {
            let valid = make_cell(Some("3.14"), CellDataType::Float);
            assert!(
                matches!(cell_to_duckdb_value(&valid).unwrap(), duckdb::types::Value::Double(v) if (v - 3.14).abs() < 1e-10)
            );

            let null_val = make_cell(None, CellDataType::Float);
            assert_eq!(
                cell_to_duckdb_value(&null_val).unwrap(),
                duckdb::types::Value::Null
            );
        }

        #[test]
        fn test_bool_variants() {
            for true_val in &["true", "TRUE", "1", "yes"] {
                let cell = make_cell(Some(true_val), CellDataType::Bool);
                assert_eq!(
                    cell_to_duckdb_value(&cell).unwrap(),
                    duckdb::types::Value::Boolean(true),
                    "Expected true for '{}'",
                    true_val
                );
            }
            for false_val in &["false", "FALSE", "0", "no", "anything"] {
                let cell = make_cell(Some(false_val), CellDataType::Bool);
                assert_eq!(
                    cell_to_duckdb_value(&cell).unwrap(),
                    duckdb::types::Value::Boolean(false),
                    "Expected false for '{}'",
                    false_val
                );
            }
        }

        #[test]
        fn test_string_and_datetime() {
            let s = make_cell(Some("hello"), CellDataType::String);
            assert_eq!(
                cell_to_duckdb_value(&s).unwrap(),
                duckdb::types::Value::Text("hello".to_string())
            );

            let dt = make_cell(Some("2024-01-15"), CellDataType::DateTime);
            assert_eq!(
                cell_to_duckdb_value(&dt).unwrap(),
                duckdb::types::Value::Text("2024-01-15".to_string())
            );

            let null_val = make_cell(None, CellDataType::String);
            assert_eq!(
                cell_to_duckdb_value(&null_val).unwrap(),
                duckdb::types::Value::Null
            );
        }
    }

    mod collect_row_types_tests {
        use super::*;

        #[test]
        fn test_empty_input() {
            assert!(collect_row_types(&[]).is_empty());
        }

        #[test]
        fn test_collects_types() {
            let row = vec![
                make_cell(Some("1"), CellDataType::Int),
                make_cell(Some("x"), CellDataType::String),
                make_cell(Some("true"), CellDataType::Bool),
            ];
            let result = collect_row_types(&[row]);
            assert_eq!(result[0], vec![CellDataType::Int, CellDataType::String, CellDataType::Bool]);
        }
    }
}
