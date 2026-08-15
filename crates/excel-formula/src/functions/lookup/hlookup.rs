use excel_types::CellValue;
use crate::engine::DataProvider;
use crate::evaluator::to_number;

use super::*;

pub(crate) fn lookup_hlookup(args: &[CellValue], _provider: &dyn DataProvider) -> CellValue {
    if args.len() < 2 {
        return CellValue::Error("#VALUE!".into());
    }

    let lookup_value = &args[0];

    let (table, row_index, range_lookup) =
        if let Some((cols, rows, range_end)) = consume_range_marker(args, 1) {
            let table = build_2d_table(&args[1 + 2..range_end], cols, rows);
            let remaining = &args[range_end..];
            let row_index = remaining.first().and_then(to_number).unwrap_or(1.0) as usize;
            let range_lookup = remaining
                .get(1)
                .is_none_or(|v| !matches!(v, CellValue::Bool(false)));
            (table, row_index, range_lookup)
        } else {
            // Inline args: treat as single-row table
            if args.len() < 3 {
                return CellValue::Error("#VALUE!".into());
            }
            let has_range_lookup = matches!(args.last(), Some(CellValue::Bool(_)));
            let param_count = if has_range_lookup { 2 } else { 1 };
            if args.len() < 2 + param_count {
                return CellValue::Error("#VALUE!".into());
            }
            let table_end = args.len() - param_count;
            let row_index = to_number(&args[table_end]).unwrap_or(1.0) as usize;
            let range_lookup = if has_range_lookup {
                match args.last().expect("checked") {
                    CellValue::Bool(b) => *b,
                    _ => true,
                }
            } else {
                true
            };
            // Treat as single-row table: each value is a column
            let table_data = &args[1..table_end];
            let table: Vec<Vec<CellValue>> = vec![table_data.to_vec()];
            (table, row_index, range_lookup)
        };

    if row_index == 0 {
        return CellValue::Error("#VALUE!".into());
    }
    if table.is_empty() || table[0].is_empty() {
        return CellValue::Error("#N/A".into());
    }
    if row_index > table.len() {
        return CellValue::Error("#REF!".into());
    }

    let row_idx = row_index - 1;
    // Search the first row (table[0]) horizontally
    let header: Vec<&CellValue> = table[0].iter().collect();

    if range_lookup {
        match approximate_match(lookup_value, &header, ApproxMode::LargestLe) {
            Some(col) => {
                if col < table[row_idx].len() {
                    table[row_idx][col].clone()
                } else {
                    CellValue::Error("#N/A".into())
                }
            }
            None => CellValue::Error("#N/A".into()),
        }
    } else {
        match exact_match(lookup_value, &header) {
            Some(col) => {
                if col < table[row_idx].len() {
                    table[row_idx][col].clone()
                } else {
                    CellValue::Error("#N/A".into())
                }
            }
            None => CellValue::Error("#N/A".into()),
        }
    }
}

// ---------------------------------------------------------------------------
// INDEX(array, row_num, [col_num])
// ---------------------------------------------------------------------------

