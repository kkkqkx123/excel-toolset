use excel_types::{CellData, CellDataType};

#[derive(Debug, Clone)]
pub struct QueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<CellData>>,
    pub row_count: usize,
}

pub fn cell_to_duckdb_value(cell: &CellData) -> duckdb::types::Value {
    match cell.data_type {
        CellDataType::Empty | CellDataType::Error => duckdb::types::Value::Null,
        CellDataType::Int => cell
            .value
            .as_deref()
            .and_then(|v| v.parse::<i64>().ok())
            .map(duckdb::types::Value::BigInt)
            .unwrap_or(duckdb::types::Value::Null),
        CellDataType::Float => cell
            .value
            .as_deref()
            .and_then(|v| v.parse::<f64>().ok())
            .map(duckdb::types::Value::Double)
            .unwrap_or(duckdb::types::Value::Null),
        CellDataType::Bool => {
            let b = cell
                .value
                .as_deref()
                .is_some_and(|v| v == "true" || v == "1" || v == "True" || v == "TRUE");
            duckdb::types::Value::Boolean(b)
        }
        CellDataType::DateTime | CellDataType::String => cell
            .value
            .as_deref()
            .map(|v| duckdb::types::Value::Text(v.to_string()))
            .unwrap_or(duckdb::types::Value::Null),
    }
}

pub fn collect_row_types(data: &[Vec<CellData>]) -> Vec<Vec<CellDataType>> {
    data.iter()
        .map(|row| row.iter().map(|c| c.data_type.clone()).collect())
        .collect()
}
