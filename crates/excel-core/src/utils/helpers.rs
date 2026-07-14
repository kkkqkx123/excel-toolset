use crate::types::*;

/// Known Excel error strings. Order by length descending so longer patterns
/// (e.g. "#GETTING_DATA") are matched before shorter prefixes.
const EXCEL_ERRORS: &[&str] = &[
    "#GETTING_DATA",
    "#DIV/0!",
    "#NAME?",
    "#NULL!",
    "#NUM!",
    "#REF!",
    "#VALUE!",
    "#N/A",
];

fn is_excel_error(s: &str) -> bool {
    let upper = s.to_uppercase();
    EXCEL_ERRORS.iter().any(|e| *e == upper)
}

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
    if is_excel_error(s) {
        return CellValue::Error(s.to_uppercase());
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
                serde_json::Value::String(s) => {
                    if is_excel_error(&s) {
                        cells.push(CellValue::Error(s.to_uppercase()));
                    } else {
                        cells.push(CellValue::String(s));
                    }
                }
                serde_json::Value::Null => cells.push(CellValue::Empty),
                _ => {
                    let s = val.to_string();
                    if is_excel_error(&s) {
                        cells.push(CellValue::Error(s.to_uppercase()));
                    } else {
                        cells.push(CellValue::String(s));
                    }
                }
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
        serde_json::Value::String(s) => {
            if is_excel_error(s) {
                CellValue::Error(s.to_uppercase())
            } else {
                CellValue::String(s.clone())
            }
        }
        serde_json::Value::Null => CellValue::Empty,
        _ => {
            let s = v.to_string();
            if is_excel_error(&s) {
                CellValue::Error(s.to_uppercase())
            } else {
                CellValue::String(s)
            }
        }
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
        "column_stacked" | "columnstacked" => Ok(ChartType::ColumnStacked),
        "column_percent_stacked" | "columnpercentstacked" => Ok(ChartType::ColumnPercentStacked),
        "line" => Ok(ChartType::Line),
        "line_stacked" | "linestacked" => Ok(ChartType::LineStacked),
        "line_percent_stacked" | "linepercentstacked" => Ok(ChartType::LinePercentStacked),
        "pie" => Ok(ChartType::Pie),
        "doughnut" => Ok(ChartType::Doughnut),
        "bar" => Ok(ChartType::Bar),
        "bar_stacked" | "barstacked" => Ok(ChartType::BarStacked),
        "bar_percent_stacked" | "barpercentstacked" => Ok(ChartType::BarPercentStacked),
        "area" => Ok(ChartType::Area),
        "area_stacked" | "areastacked" => Ok(ChartType::AreaStacked),
        "area_percent_stacked" | "areapercentstacked" => Ok(ChartType::AreaPercentStacked),
        "scatter" => Ok(ChartType::Scatter),
        "scatter_straight" | "scatterstraight" => Ok(ChartType::ScatterStraight),
        "scatter_straight_with_markers" | "scatterstraightwithmarkers" => {
            Ok(ChartType::ScatterStraightWithMarkers)
        }
        "scatter_smooth" | "scattersmooth" => Ok(ChartType::ScatterSmooth),
        "scatter_smooth_with_markers" | "scattersmoothwithmarkers" => {
            Ok(ChartType::ScatterSmoothWithMarkers)
        }
        "stock" | "candlestick" | "k_line" | "kline" => Ok(ChartType::Stock),
        "radar" => Ok(ChartType::Radar),
        "radar_with_markers" | "radarwithmarkers" => Ok(ChartType::RadarWithMarkers),
        _ => Err(AppError::InvalidChartType(s.into())),
    }
}

/// Predefined Excel number format constants.
pub mod number_formats {
    /// General (default)
    pub const GENERAL: &str = "General";
    /// 1000 -> "1,000"
    pub const NUMBER: &str = "#,##0";
    /// 1000.5 -> "1,000.50"
    pub const NUMBER_2D: &str = "#,##0.00";
    /// 1000 -> "$1,000"
    pub const CURRENCY: &str = "$#,##0";
    /// 1000.5 -> "$1,000.50"
    pub const CURRENCY_2D: &str = "$#,##0.00";
    /// Accounting format (aligns currency symbol)
    pub const ACCOUNTING: &str = "_($* #,##0_);_($* (#,##0);_($* \"-\"_);_(@_)";
    /// Accounting with 2 decimals
    pub const ACCOUNTING_2D: &str = "_($* #,##0.00_);_($* (#,##0.00);_($* \"-\"??_);_(@_)";
    /// 1 -> "100%"
    pub const PERCENTAGE: &str = "0%";
    /// 0.5 -> "50.00%"
    pub const PERCENTAGE_2D: &str = "0.00%";
    /// 1000000 -> "1,000,000"
    pub const THOUSANDS: &str = "#,##0,";
    /// 1000000 -> "1.00"
    pub const MILLIONS: &str = "#,##0.00,,\"M\"";
    /// 1.23e+05
    pub const SCIENTIFIC: &str = "0.00E+00";
    /// 2025-01-15
    pub const DATE: &str = "yyyy-mm-dd";
    /// 2025-01-15 13:30:00
    pub const DATETIME: &str = "yyyy-mm-dd hh:mm:ss";
    /// 13:30
    pub const TIME: &str = "hh:mm";
    /// 1.5 -> "1 1/2"
    pub const FRACTION: &str = "# ?/?";
    /// 1234 -> "00000"
    pub const ZIP_CODE: &str = "00000";
    /// 1234 -> "1,234.00" (text)
    pub const TEXT: &str = "@";
}

/// Resolve a named number format to its Excel format string.
/// Returns the original string if no match is found (pass-through for custom formats).
pub fn resolve_number_format(name: &str) -> &str {
    use number_formats::*;

    match name.to_lowercase().as_str() {
        "general" => GENERAL,
        "number" | "integer" => NUMBER,
        "number_2d" | "number2d" | "number.00" => NUMBER_2D,
        "currency" | "dollar" => CURRENCY,
        "currency_2d" | "currency2d" | "dollar.00" => CURRENCY_2D,
        "accounting" | "accountant" => ACCOUNTING,
        "accounting_2d" | "accounting2d" => ACCOUNTING_2D,
        "percentage" | "percent" | "pct" => PERCENTAGE,
        "percentage_2d" | "percent.00" | "pct.00" => PERCENTAGE_2D,
        "thousands" => THOUSANDS,
        "millions" => MILLIONS,
        "scientific" | "sci" => SCIENTIFIC,
        "date" => DATE,
        "datetime" => DATETIME,
        "time" => TIME,
        "fraction" => FRACTION,
        "zip" | "zipcode" | "zip_code" => ZIP_CODE,
        "text" | "string" | "@" => TEXT,
        // Pass through: treat as raw format string
        _ => name,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_cell_value_number() {
        assert_eq!(parse_cell_value("123"), CellValue::Number(123.0));
        assert_eq!(parse_cell_value("-45.67"), CellValue::Number(-45.67));
        assert_eq!(parse_cell_value("0"), CellValue::Number(0.0));
    }

    #[test]
    fn test_parse_cell_value_bool() {
        assert_eq!(parse_cell_value("true"), CellValue::Bool(true));
        assert_eq!(parse_cell_value("false"), CellValue::Bool(false));
        assert_eq!(parse_cell_value("TRUE"), CellValue::Bool(true));
        assert_eq!(parse_cell_value("False"), CellValue::Bool(false));
    }

    #[test]
    fn test_parse_cell_value_empty() {
        assert_eq!(parse_cell_value("null"), CellValue::Empty);
        assert_eq!(parse_cell_value("none"), CellValue::Empty);
        assert_eq!(parse_cell_value("empty"), CellValue::Empty);
    }

    #[test]
    fn test_parse_cell_value_error() {
        assert_eq!(
            parse_cell_value("#DIV/0!"),
            CellValue::Error("#DIV/0!".to_string())
        );
        assert_eq!(
            parse_cell_value("#N/A"),
            CellValue::Error("#N/A".to_string())
        );
        assert_eq!(
            parse_cell_value("#VALUE!"),
            CellValue::Error("#VALUE!".to_string())
        );
        assert_eq!(
            parse_cell_value("#REF!"),
            CellValue::Error("#REF!".to_string())
        );
        assert_eq!(
            parse_cell_value("#NAME?"),
            CellValue::Error("#NAME?".to_string())
        );
        assert_eq!(
            parse_cell_value("#NULL!"),
            CellValue::Error("#NULL!".to_string())
        );
        assert_eq!(
            parse_cell_value("#NUM!"),
            CellValue::Error("#NUM!".to_string())
        );
        assert_eq!(
            parse_cell_value("#GETTING_DATA"),
            CellValue::Error("#GETTING_DATA".to_string())
        );
    }

    #[test]
    fn test_parse_cell_value_string() {
        assert_eq!(
            parse_cell_value("hello"),
            CellValue::String("hello".to_string())
        );
        assert_eq!(
            parse_cell_value("Hello World"),
            CellValue::String("Hello World".to_string())
        );
    }

    #[test]
    fn test_parse_cell_value_grid() {
        let json = r#"[[1, "hello", true], [null, 3.14, false]]"#;
        let result: Vec<Vec<CellValue>> = parse_cell_value_grid(json).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0][0], CellValue::Number(1.0));
        assert_eq!(result[0][1], CellValue::String("hello".to_string()));
        assert_eq!(result[0][2], CellValue::Bool(true));
        assert_eq!(result[1][0], CellValue::Empty);
        assert_eq!(result[1][1], CellValue::Number(3.14));
        assert_eq!(result[1][2], CellValue::Bool(false));
    }

    #[test]
    fn test_parse_cell_value_grid_with_errors() {
        let json = "[[\"#DIV/0!\", \"#N/A\"]]";
        let result = parse_cell_value_grid(json).unwrap();
        assert_eq!(result[0][0], CellValue::Error("#DIV/0!".to_string()));
        assert_eq!(result[0][1], CellValue::Error("#N/A".to_string()));
    }

    #[test]
    fn test_parse_cell_value_grid_invalid_json() {
        let result = parse_cell_value_grid("invalid json");
        assert!(result.is_err());
    }

    #[test]
    fn test_json_val_to_cell_value() {
        use serde_json::json;

        assert_eq!(json_val_to_cell_value(&json!(42)), CellValue::Number(42.0));
        assert_eq!(
            json_val_to_cell_value(&json!(3.14)),
            CellValue::Number(3.14)
        );
        assert_eq!(json_val_to_cell_value(&json!(true)), CellValue::Bool(true));
        assert_eq!(
            json_val_to_cell_value(&json!(false)),
            CellValue::Bool(false)
        );
        assert_eq!(
            json_val_to_cell_value(&json!("hello")),
            CellValue::String("hello".to_string())
        );
        assert_eq!(json_val_to_cell_value(&json!(null)), CellValue::Empty);
        assert_eq!(
            json_val_to_cell_value(&json!("#DIV/0!")),
            CellValue::Error("#DIV/0!".to_string())
        );
    }

    #[test]
    fn test_parse_filter_op() {
        assert_eq!(parse_filter_op("eq").unwrap(), FilterOp::Eq);
        assert_eq!(parse_filter_op("=").unwrap(), FilterOp::Eq);
        assert_eq!(parse_filter_op("==").unwrap(), FilterOp::Eq);
        assert_eq!(parse_filter_op("ne").unwrap(), FilterOp::Ne);
        assert_eq!(parse_filter_op("!=").unwrap(), FilterOp::Ne);
        assert_eq!(parse_filter_op("gt").unwrap(), FilterOp::Gt);
        assert_eq!(parse_filter_op(">").unwrap(), FilterOp::Gt);
        assert_eq!(parse_filter_op("lt").unwrap(), FilterOp::Lt);
        assert_eq!(parse_filter_op("<").unwrap(), FilterOp::Lt);
        assert_eq!(parse_filter_op("ge").unwrap(), FilterOp::Ge);
        assert_eq!(parse_filter_op(">=").unwrap(), FilterOp::Ge);
        assert_eq!(parse_filter_op("le").unwrap(), FilterOp::Le);
        assert_eq!(parse_filter_op("<=").unwrap(), FilterOp::Le);
        assert_eq!(parse_filter_op("contains").unwrap(), FilterOp::Contains);
        assert_eq!(parse_filter_op("startswith").unwrap(), FilterOp::StartsWith);
        assert_eq!(
            parse_filter_op("starts_with").unwrap(),
            FilterOp::StartsWith
        );
        assert_eq!(parse_filter_op("endswith").unwrap(), FilterOp::EndsWith);
        assert_eq!(parse_filter_op("ends_with").unwrap(), FilterOp::EndsWith);
    }

    #[test]
    fn test_parse_filter_op_invalid() {
        assert!(parse_filter_op("invalid").is_err());
        assert!(parse_filter_op("").is_err());
    }

    #[test]
    fn test_chart_type_from_str() {
        assert_eq!(chart_type_from_str("column").unwrap(), ChartType::Column);
        assert_eq!(chart_type_from_str("COLUMN").unwrap(), ChartType::Column);
        assert_eq!(chart_type_from_str("line").unwrap(), ChartType::Line);
        assert_eq!(chart_type_from_str("pie").unwrap(), ChartType::Pie);
        assert_eq!(chart_type_from_str("bar").unwrap(), ChartType::Bar);
        assert_eq!(chart_type_from_str("area").unwrap(), ChartType::Area);
        assert_eq!(chart_type_from_str("scatter").unwrap(), ChartType::Scatter);
    }

    #[test]
    fn test_chart_type_from_str_invalid() {
        assert!(chart_type_from_str("invalid").is_err());
        assert!(chart_type_from_str("").is_err());
    }
}
