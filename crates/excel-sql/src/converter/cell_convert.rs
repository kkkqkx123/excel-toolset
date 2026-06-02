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
