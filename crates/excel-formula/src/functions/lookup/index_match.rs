use excel_types::CellValue;
use crate::engine::DataProvider;
use crate::evaluator::to_number;

use super::*;

pub(crate) fn lookup_index(args: &[CellValue], _provider: &dyn DataProvider) -> CellValue {
    if args.len() < 2 {
        return CellValue::Error("#VALUE!".into());
    }

    let row_num = args.get(1).and_then(to_number);
    if row_num.is_none() {
        return CellValue::Error("#VALUE!".into());
    }
    let row_num = row_num.unwrap() as usize;
    if row_num == 0 {
        return CellValue::Error("#VALUE!".into());
    }

    let _row_idx = row_num - 1;

    // Check if args[0] is a range marker (2D table)
    if let Some((cols, rows, range_end)) = consume_range_marker(args, 0) {
        let table = build_2d_table(&args[2..range_end], cols, rows);
        let remaining = &args[range_end..];
        // Parse row_num and col_num from remaining args (they were passed after range)
        let row_num = remaining.first().and_then(to_number).unwrap_or(1.0) as usize;
        let col_num = remaining.get(1).and_then(to_number);

        if row_num == 0 || row_num > table.len() {
            return CellValue::Error("#REF!".into());
        }
        let row_idx = row_num - 1;
        if let Some(cn) = col_num {
            let cn = cn as usize;
            if cn == 0 || cn > table[row_idx].len() {
                return CellValue::Error("#REF!".into());
            }
            table[row_idx][cn - 1].clone()
        } else {
            // Return whole row: we can only return the first cell
            table[row_idx][0].clone()
        }
    } else if let Some(col_num) = args.get(2).and_then(to_number) {
        // Inline array with row_num and col_num
        let col_num = col_num as usize;
        if col_num == 0 {
            return CellValue::Error("#VALUE!".into());
        }
        // args[0] is the single array value; with a col_num we need 2D data.
        // Only row=1, col=1 is valid for a single-value inline array.
        if row_num == 1 && col_num == 1 {
            args[0].clone()
        } else {
            CellValue::Error("#REF!".into())
        }
    } else {
        // INDEX(array, row_num) — single column, inline/single-value array.
        // Without a range marker, args[0] is the only array element;
        // args[1] is row_num. Only row 1 is valid.
        if row_num == 1 {
            args[0].clone()
        } else {
            CellValue::Error("#REF!".into())
        }
    }
}

// ---------------------------------------------------------------------------
// MATCH(lookup_value, lookup_array, [match_type])
// ---------------------------------------------------------------------------


pub(crate) fn lookup_match(args: &[CellValue], _provider: &dyn DataProvider) -> CellValue {
    if args.len() < 2 {
        return CellValue::Error("#VALUE!".into());
    }

    let lookup_value = &args[0];

    // Consume range marker (if present) and determine match_type from remainder.
    let (array, remainder) = if let Some((_cols, _rows, end)) = consume_range_marker(args, 1) {
        let flat = &args[1 + 2..end];
        (flat.to_vec(), &args[end..])
    } else {
        // No range marker: args[1..] are the array values.
        // Determine where the array ends and match_type begins.
        let end = if args.len() > 2 && to_number(&args[args.len() - 1]).is_some() {
            args.len() - 1
        } else {
            args.len()
        };
        (args[1..end].to_vec(), &args[end..])
    };

    let match_type = remainder.first().and_then(to_number).unwrap_or(1.0) as i32;

    if array.is_empty() {
        return CellValue::Error("#N/A".into());
    }

    let refs: Vec<&CellValue> = array.iter().collect();

    match match_type {
        0 => {
            // Exact match
            match exact_match(lookup_value, &refs) {
                Some(idx) => CellValue::Number((idx + 1) as f64),
                None => CellValue::Error("#N/A".into()),
            }
        }
        -1 => {
            // Exact or next larger; array must be sorted descending
            match approximate_match(lookup_value, &refs, ApproxMode::SmallestGe) {
                Some(idx) => CellValue::Number((idx + 1) as f64),
                None => CellValue::Error("#N/A".into()),
            }
        }
        _ => {
            // match_type = 1: exact or next smaller; array must be sorted ascending
            match approximate_match(lookup_value, &refs, ApproxMode::LargestLe) {
                Some(idx) => CellValue::Number((idx + 1) as f64),
                None => CellValue::Error("#N/A".into()),
            }
        }
    }
}

// ---------------------------------------------------------------------------
// OFFSET(reference, rows, cols, [height], [width])
// ---------------------------------------------------------------------------

