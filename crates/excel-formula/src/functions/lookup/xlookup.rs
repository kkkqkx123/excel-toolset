use excel_types::CellValue;
use crate::engine::DataProvider;
use crate::evaluator::to_number;

use super::*;

pub(crate) fn lookup_xlookup(args: &[CellValue], _provider: &dyn DataProvider) -> CellValue {
    if args.len() < 2 {
        return CellValue::Error("#VALUE!".into());
    }

    let lookup_value = &args[0];

    // Consume lookup_array range marker (if present)
    let (lookup_array, rest) = if let Some((_cols, _rows, end)) = consume_range_marker(args, 1) {
        let flat = &args[1 + 2..end];
        (flat.to_vec(), &args[end..])
    } else {
        // No range marker: args[1] is the only lookup_array element
        (args[1..].to_vec(), &[] as &[CellValue])
    };

    // Consume return_array range marker (if present)
    let rest = if rest.is_empty() {
        &args[1 + lookup_array.len()..]
    } else {
        rest
    };

    let (return_array, rest) = if !rest.is_empty() {
        if let Some((_cols, _rows, end)) = consume_range_marker(rest, 0) {
            let flat = &rest[2..end];
            (flat.to_vec(), &rest[end..])
        } else {
            (rest.to_vec(), &[] as &[CellValue])
        }
    } else {
        (vec![], rest)
    };

    // Remaining params
    let if_not_found = rest.first().cloned();
    let match_mode = rest
        .get(if if_not_found.is_some() { 1 } else { 0 })
        .and_then(to_number)
        .unwrap_or(0.0) as i32;
    // search_mode: 1=first-to-last, -1=last-to-first, 2=binary-asc, -2=binary-desc
    let search_mode = rest
        .get(if if_not_found.is_some() { 2 } else { 1 })
        .and_then(to_number)
        .unwrap_or(1.0) as i32;

    if lookup_array.is_empty() {
        return if_not_found.unwrap_or(CellValue::Error("#N/A".into()));
    }

    // Build the search order
    let indices = match search_mode {
        -1 => (0..lookup_array.len()).rev().collect::<Vec<_>>(),
        _ => (0..lookup_array.len()).collect::<Vec<_>>(),
    };

    let search_refs: Vec<&CellValue> = lookup_array.iter().collect();

    match match_mode {
        -1 => {
            // Exact match, or next smaller item
            match exact_match(lookup_value, &search_refs) {
                Some(idx) => {
                    if idx < return_array.len() {
                        return_array[idx].clone()
                    } else {
                        CellValue::Error("#N/A".into())
                    }
                }
                None => {
                    // Try approximate (largest <= lookup)
                    let sorted_refs: Vec<&CellValue> =
                        indices.iter().map(|&i| search_refs[i]).collect();
                    match approximate_match(lookup_value, &sorted_refs, ApproxMode::LargestLe) {
                        Some(idx) => {
                            if idx < return_array.len() {
                                return_array[indices[idx]].clone()
                            } else {
                                CellValue::Error("#N/A".into())
                            }
                        }
                        None => if_not_found.unwrap_or(CellValue::Error("#N/A".into())),
                    }
                }
            }
        }
        1 => {
            // Exact match, or next larger item
            match exact_match(lookup_value, &search_refs) {
                Some(idx) => {
                    if idx < return_array.len() {
                        return_array[idx].clone()
                    } else {
                        CellValue::Error("#N/A".into())
                    }
                }
                None => {
                    let sorted_refs: Vec<&CellValue> =
                        indices.iter().map(|&i| search_refs[i]).collect();
                    match approximate_match(lookup_value, &sorted_refs, ApproxMode::SmallestGe) {
                        Some(idx) => {
                            if idx < return_array.len() {
                                return_array[indices[idx]].clone()
                            } else {
                                CellValue::Error("#N/A".into())
                            }
                        }
                        None => if_not_found.unwrap_or(CellValue::Error("#N/A".into())),
                    }
                }
            }
        }
        2 => {
            // Wildcard match (simplified: treat as exact match)
            match exact_match(lookup_value, &search_refs) {
                Some(idx) if idx < return_array.len() => return_array[idx].clone(),
                _ => if_not_found.unwrap_or(CellValue::Error("#N/A".into())),
            }
        }
        _ => {
            // Default: exact match (match_mode = 0)
            for &i in &indices {
                if lookup_values_equal(&lookup_array[i], lookup_value) {
                    return if i < return_array.len() {
                        return_array[i].clone()
                    } else {
                        CellValue::Error("#N/A".into())
                    };
                }
            }
            if_not_found.unwrap_or(CellValue::Error("#N/A".into()))
        }
    }
}

// ---------------------------------------------------------------------------
// HLOOKUP(lookup_value, table_array, row_index_num, [range_lookup])
// ---------------------------------------------------------------------------

