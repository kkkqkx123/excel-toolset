use excel_types::CellDataType;

pub fn cell_to_duckdb_type(dt: &CellDataType) -> &'static str {
    match dt {
        CellDataType::Int => "INTEGER",
        CellDataType::Float => "DOUBLE",
        CellDataType::Bool => "BOOLEAN",
        CellDataType::DateTime => "TIMESTAMP",
        CellDataType::String | CellDataType::Error | CellDataType::Empty => "VARCHAR",
    }
}

pub fn infer_column_types(data: &[Vec<CellDataType>]) -> Vec<CellDataType> {
    let max_cols = data.iter().map(|r| r.len()).max().unwrap_or(0);
    let mut col_types = vec![CellDataType::String; max_cols];

    for (col, col_type) in col_types.iter_mut().enumerate().take(max_cols) {
        for row in data {
            if let Some(dt) = row.get(col)
                && !matches!(dt, CellDataType::Empty) {
                    *col_type = dt.clone();
                    break;
                }
        }
    }

    col_types
}
