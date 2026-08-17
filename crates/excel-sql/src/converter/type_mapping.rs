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

    for (col, col_type) in col_types.iter_mut().enumerate() {
        // Count non-empty cells by kind, and track the highest numeric type
        // seen. A single stray text cell must NOT downgrade a mostly-numeric
        // column to VARCHAR (that used to break SUM()/> on otherwise-numeric
        // data); instead we pick the majority kind and let the insert step
        // coerce the minority dirty cells to NULL.
        let mut numeric = 0u32;
        let mut text = 0u32;
        let mut boolean = 0u32;
        let mut datetime = 0u32;
        let mut best_numeric: CellDataType = CellDataType::Int;

        for row in data {
            match row.get(col) {
                None | Some(CellDataType::Empty) => {}
                Some(CellDataType::Int) | Some(CellDataType::Float) => {
                    numeric += 1;
                    let ct = row.get(col).unwrap_or(&CellDataType::Empty);
                    best_numeric = combine_types(&best_numeric, ct);
                }
                Some(CellDataType::Bool) => boolean += 1,
                Some(CellDataType::DateTime) => datetime += 1,
                Some(_) => text += 1, // String / Error
            }
        }

        let total = numeric + text + boolean + datetime;
        if total == 0 {
            *col_type = CellDataType::String;
        } else if numeric >= boolean && numeric >= datetime && numeric >= text {
            *col_type = best_numeric;
        } else if boolean >= datetime && boolean >= text {
            *col_type = CellDataType::Bool;
        } else if datetime >= text {
            *col_type = CellDataType::DateTime;
        } else {
            *col_type = CellDataType::String;
        }
    }

    col_types
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cell_to_duckdb_type() {
        use excel_types::CellDataType::*;
        assert_eq!(cell_to_duckdb_type(&Int), "INTEGER");
        assert_eq!(cell_to_duckdb_type(&Float), "DOUBLE");
        assert_eq!(cell_to_duckdb_type(&Bool), "BOOLEAN");
        assert_eq!(cell_to_duckdb_type(&DateTime), "TIMESTAMP");
        assert_eq!(cell_to_duckdb_type(&String), "VARCHAR");
        assert_eq!(cell_to_duckdb_type(&Error), "VARCHAR");
        assert_eq!(cell_to_duckdb_type(&Empty), "VARCHAR");
    }

    #[test]
    fn test_combine_types_empty() {
        use excel_types::CellDataType::*;
        assert_eq!(combine_types(&Empty, &Int), Int);
        assert_eq!(combine_types(&String, &Empty), String);
        assert_eq!(combine_types(&Empty, &Empty), Empty);
    }

    #[test]
    fn test_combine_types_string_dominates() {
        use excel_types::CellDataType::*;
        assert_eq!(combine_types(&String, &Int), String);
        assert_eq!(combine_types(&Float, &String), String);
        assert_eq!(combine_types(&String, &Bool), String);
        assert_eq!(combine_types(&DateTime, &String), String);
    }

    #[test]
    fn test_combine_types_promotion_chain() {
        use excel_types::CellDataType::*;
        // Bool + Int → Int
        assert_eq!(combine_types(&Bool, &Int), Int);
        // Int + Float → Float
        assert_eq!(combine_types(&Int, &Float), Float);
        // Float + DateTime → DateTime
        assert_eq!(combine_types(&Float, &DateTime), DateTime);
        // Bool + DateTime → DateTime
        assert_eq!(combine_types(&Bool, &DateTime), DateTime);
        // Int + DateTime → DateTime
        assert_eq!(combine_types(&DateTime, &Int), DateTime);
    }

    #[test]
    fn test_combine_types_same_type() {
        use excel_types::CellDataType::*;
        assert_eq!(combine_types(&Int, &Int), Int);
        assert_eq!(combine_types(&Float, &Float), Float);
        assert_eq!(combine_types(&Bool, &Bool), Bool);
        assert_eq!(combine_types(&DateTime, &DateTime), DateTime);
    }

    #[test]
    fn test_infer_column_types_empty() {
        let result = infer_column_types(&[]);
        assert!(result.is_empty());
    }

    #[test]
    fn test_infer_column_types_single_row() {
        use excel_types::CellDataType::*;
        let data = vec![vec![Int, Float, Bool, String]];
        let result = infer_column_types(&data);
        assert_eq!(result, vec![Int, Float, Bool, String]);
    }

    #[test]
    fn test_infer_column_types_mixed_types() {
        use excel_types::CellDataType::*;
        let data = vec![vec![Int, Bool, Float], vec![Float, Int, Int]];
        let result = infer_column_types(&data);
        assert_eq!(result, vec![Float, Int, Float]);
    }

    #[test]
    fn test_infer_column_types_empty_cells_skipped() {
        use excel_types::CellDataType::*;
        let data = vec![vec![Int, Empty], vec![Empty, Float]];
        let result = infer_column_types(&data);
        assert_eq!(result, vec![Int, Float]);
    }

    #[test]
    fn test_infer_column_types_all_empty_defaults_to_string() {
        use excel_types::CellDataType::*;
        let data = vec![vec![Empty, Empty]];
        let result = infer_column_types(&data);
        assert_eq!(result, vec![String, String]);
    }

    #[test]
    fn test_infer_column_types_uneven_rows() {
        use excel_types::CellDataType::*;
        let data = vec![vec![Int, Float], vec![Int]];
        let result = infer_column_types(&data);
        assert_eq!(result, vec![Int, Float]);
    }
}
