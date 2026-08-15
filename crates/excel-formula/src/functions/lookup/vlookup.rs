use excel_types::CellValue;
use crate::engine::DataProvider;
use crate::evaluator::to_number;

use super::*;

pub(crate) fn lookup_vlookup(args: &[CellValue], _provider: &dyn DataProvider) -> CellValue {
    if args.len() < 2 {
        return CellValue::Error("#VALUE!".into());
    }

    let lookup_value = &args[0];

    // --- Parse table and remaining params ---
    let (table, col_index, range_lookup) =
        if let Some((cols, rows, range_end)) = consume_range_marker(args, 1) {
            // Range marker present: remaining args start after the expanded range.
            let table = build_2d_table(&args[1 + 2..range_end], cols, rows);
            let remaining = &args[range_end..];
            let col_index = remaining.first().and_then(to_number).unwrap_or(1.0) as usize;
            let range_lookup = remaining
                .get(1)
                .is_none_or(|v| !matches!(v, CellValue::Bool(false)));
            (table, col_index, range_lookup)
        } else {
            // No range marker: use inline args as single-column table.
            match parse_inline_vlookup_args(args) {
                Some((table, ci, rl)) => (table, ci, rl),
                None => return CellValue::Error("#VALUE!".into()),
            }
        };

    if col_index == 0 {
        return CellValue::Error("#VALUE!".into());
    }
    if table.is_empty() {
        return CellValue::Error("#N/A".into());
    }
    let n_cols = table[0].len();
    if col_index > n_cols {
        return CellValue::Error("#REF!".into());
    }

    let col_idx = col_index - 1;
    let search_col: Vec<&CellValue> = extract_column(&table, 0);

    if range_lookup {
        // Approximate: data must be sorted ascending.
        match approximate_match(lookup_value, &search_col, ApproxMode::LargestLe) {
            Some(r) => {
                if r < table.len() {
                    table[r][col_idx].clone()
                } else {
                    CellValue::Error("#N/A".into())
                }
            }
            None => CellValue::Error("#N/A".into()),
        }
    } else {
        // Exact match
        match exact_match(lookup_value, &search_col) {
            Some(r) => table[r][col_idx].clone(),
            None => CellValue::Error("#N/A".into()),
        }
    }
}


/// Parse inline args for VLOOKUP when no range marker is present.
///
/// Inline pattern: `args[0]=lookup, args[1..n-2/1]=table cells, ..., col_index, [range_lookup]`.
pub(crate) fn parse_inline_vlookup_args(args: &[CellValue]) -> Option<(Vec<Vec<CellValue>>, usize, bool)> {
    if args.len() < 3 {
        return None;
    }

    // Last arg is range_lookup if it's a Bool
    let has_range_lookup = matches!(args.last()?, CellValue::Bool(_));
    let param_count = if has_range_lookup { 2 } else { 1 };

    if args.len() < 2 + param_count {
        return None;
    }

    let table_vals_end = args.len() - param_count;
    let col_index = args[table_vals_end].clone();
    let col_index = to_number(&col_index)? as usize;

    let range_lookup = if has_range_lookup {
        match args.last().expect("checked") {
            CellValue::Bool(b) => *b,
            _ => true,
        }
    } else {
        true
    };

    let table_data = &args[1..table_vals_end];
    let table: Vec<Vec<CellValue>> = table_data.iter().map(|v| vec![v.clone()]).collect();

    Some((table, col_index, range_lookup))
}

// ---------------------------------------------------------------------------
// XLOOKUP(lookup_value, lookup_array, return_array, [if_not_found], [match_mode], [search_mode])
// ---------------------------------------------------------------------------

