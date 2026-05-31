use crate::types::*;

pub fn parse_cell_value(s: &str) -> CellValue {
    if let Ok(n) = s.parse::<f64>() {
        return CellValue::Number(n);
    }
    match s.to_lowercase().as_str() {
        "true" => return CellValue::Bool(true),
        "false" => return CellValue::Bool(false),
        "null" | "none" | "empty" => return CellValue::Empty,
        _ => {}
    }
    CellValue::String(s.to_string())
}

pub fn parse_cell_value_grid(s: &str) -> Result<Vec<Vec<CellValue>>> {
    let outer: Vec<Vec<serde_json::Value>> = serde_json::from_str(s)
        .map_err(|e| AppError::Serialize(format!("Invalid data JSON: {}", e)))?;
    let mut grid = Vec::new();
    for row in outer {
        let mut cells = Vec::new();
        for val in row {
            match val {
                serde_json::Value::Number(n) => {
                    cells.push(CellValue::Number(n.as_f64().unwrap_or(0.0)));
                }
                serde_json::Value::Bool(b) => cells.push(CellValue::Bool(b)),
                serde_json::Value::String(s) => cells.push(CellValue::String(s)),
                serde_json::Value::Null => cells.push(CellValue::Empty),
                _ => cells.push(CellValue::String(val.to_string())),
            }
        }
        grid.push(cells);
    }
    Ok(grid)
}

pub fn json_val_to_cell_value(v: &serde_json::Value) -> CellValue {
    match v {
        serde_json::Value::Number(n) => CellValue::Number(n.as_f64().unwrap_or(0.0)),
        serde_json::Value::Bool(b) => CellValue::Bool(*b),
        serde_json::Value::String(s) => CellValue::String(s.clone()),
        serde_json::Value::Null => CellValue::Empty,
        _ => CellValue::String(v.to_string()),
    }
}

pub fn parse_filter_op(s: &str) -> Result<FilterOp> {
    match s.to_lowercase().as_str() {
        "eq" | "=" | "==" => Ok(FilterOp::Eq),
        "ne" | "!=" => Ok(FilterOp::Ne),
        "gt" | ">" => Ok(FilterOp::Gt),
        "lt" | "<" => Ok(FilterOp::Lt),
        "ge" | ">=" => Ok(FilterOp::Ge),
        "le" | "<=" => Ok(FilterOp::Le),
        "contains" => Ok(FilterOp::Contains),
        "startswith" | "starts_with" => Ok(FilterOp::StartsWith),
        "endswith" | "ends_with" => Ok(FilterOp::EndsWith),
        _ => Err(AppError::InvalidFilterOp(s.into())),
    }
}

pub fn chart_type_from_str(s: &str) -> Result<ChartType> {
    match s.to_lowercase().as_str() {
        "column" => Ok(ChartType::Column),
        "line" => Ok(ChartType::Line),
        "pie" => Ok(ChartType::Pie),
        "bar" => Ok(ChartType::Bar),
        "area" => Ok(ChartType::Area),
        "scatter" => Ok(ChartType::Scatter),
        _ => Err(AppError::InvalidChartType(s.into())),
    }
}
