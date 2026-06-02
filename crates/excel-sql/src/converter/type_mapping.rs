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

/// Returns the "higher" type when combining two types.
/// The hierarchy is: Empty < String < Bool < Int < Float < DateTime
fn combine_types(a: &CellDataType, b: &CellDataType) -> CellDataType {
    use CellDataType::*;
    match (a, b) {
        // Any type combined with Empty returns the other type
        (Empty, other) | (other, Empty) => other.clone(),

        // String can absorb any type
        (String, _) | (_, String) => String,

        // Bool can be promoted to Int/Float/DateTime
        (Bool, Int | Float | DateTime) | (Int | Float | DateTime, Bool) => {
            if matches!(b, Int | Float | DateTime) {
                b.clone()
            } else {
                a.clone()
            }
        }
        (Bool, Bool) => Bool,

        // Int can be promoted to Float/DateTime
        (Int, Float | DateTime) | (Float | DateTime, Int) => {
            if matches!(b, Float | DateTime) {
                b.clone()
            } else {
                a.clone()
            }
        }
        (Int, Int) => Int,

        // Float can be promoted to DateTime
        (Float, DateTime) | (DateTime, Float) => DateTime,
        (Float, Float) => Float,

        // DateTime is highest
        (DateTime, DateTime) => DateTime,

        // All other combinations fall back to String
        _ => String,
    }
}

pub fn infer_column_types(data: &[Vec<CellDataType>]) -> Vec<CellDataType> {
    let max_cols = data.iter().map(|r| r.len()).max().unwrap_or(0);
    let mut col_types = vec![CellDataType::Empty; max_cols];

    #[allow(clippy::needless_range_loop)]
    for col in 0..max_cols {
        for row in data {
            if let Some(cell_type) = row.get(col) {
                // Skip empty cells
                if *cell_type == CellDataType::Empty {
                    continue;
                }

                // Combine with current column type using our type hierarchy
                col_types[col] = combine_types(&col_types[col], cell_type);

                // If we've reached String or DateTime (highest), no need to check further
                if matches!(
                    col_types[col],
                    CellDataType::String | CellDataType::DateTime
                ) {
                    break;
                }
            }
        }

        // If all values were Empty, default to String
        if col_types[col] == CellDataType::Empty {
            col_types[col] = CellDataType::String;
        }
    }

    col_types
}
