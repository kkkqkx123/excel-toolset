use regex::Regex;

use crate::excel_write::{ensure_dimensions, modify_file};
use crate::types::*;
use crate::utils::cell_ref;

/// Fill a formula from `source` cell into all cells in `target_range`, offsetting
/// relative references according to A1 rules.
///
/// # Reference offset rules
/// - `$C$1`: both row and col absolute — no offset
/// - `$C1`: col absolute, row relative — offset row only
/// - `C$1`: row absolute, col relative — offset col only
/// - `C1`: both relative — offset both
/// - `A1:B5`: range reference — both ends offset independently
/// - `SheetName!A1`: cross-sheet reference — left unchanged
///
/// # Errors
/// Returns `CellNotFound` if `source` has no formula.
pub fn fill_formula(
    path: &str,
    sheet: &str,
    source: &str,
    target_range: &str,
    params: &SecurityParams,
) -> Result<WriteResult> {
    let (src_row, src_col) = cell_ref::parse_cell_ref(source)?;
    let (tgt_start_row, tgt_end_row, tgt_start_col, tgt_end_col) =
        cell_ref::parse_range_normalized(target_range)?;

    let mut result = modify_file(path, params, |old_data| {
        let mut new_data = old_data.clone();
        let sd = new_data
            .get_mut(sheet)
            .ok_or_else(|| AppError::SheetNotFound(sheet.into()))?;

        let source_formula = sd
            .rows
            .get(src_row as usize)
            .and_then(|row| row.get(src_col as usize))
            .and_then(|cell| cell.formula.clone())
            .ok_or_else(|| AppError::CellNotFound(src_row, src_col))?;

        for target_row in tgt_start_row..=tgt_end_row {
            let row_offset = target_row as i64 - src_row as i64;
            for target_col in tgt_start_col..=tgt_end_col {
                let col_offset = target_col as i64 - src_col as i64;

                let offset_formula =
                    offset_cell_references(&source_formula, row_offset, col_offset);
                let cleaned = offset_formula.strip_prefix('=').unwrap_or(&offset_formula);

                ensure_dimensions(sd, target_row as usize, target_col as usize);
                sd.rows[target_row as usize][target_col as usize] = CellData {
                    value: None,
                    data_type: CellDataType::String,
                    formula: Some(cleaned.to_string()),
                };
            }
        }

        Ok(new_data)
    })?;

    let total_cells =
        (tgt_end_row - tgt_start_row + 1) as usize * (tgt_end_col - tgt_start_col + 1) as usize;
    result.message = format!(
        "Filled formula from {}!{} into {} cells in {}!{}",
        sheet, source, total_cells, sheet, target_range
    );

    Ok(result)
}

/// Offset all cell references in a formula string by the given row/col deltas.
/// Cross-sheet references (`Sheet!Cell`) are left unchanged because sheet-qualified
/// references point to external data sources.
fn offset_cell_references(formula: &str, row_offset: i64, col_offset: i64) -> String {
    if row_offset == 0 && col_offset == 0 {
        return formula.to_string();
    }

    let sheet_cell_re =
        Regex::new(r"[A-Za-z_][A-Za-z0-9_]*!(\$?[A-Za-z]{1,3}\$?\d+)(:\$?[A-Za-z]{1,3}\$?\d+)?")
            .expect("valid regex");
    let simple_re =
        Regex::new(r"(\$?[A-Za-z]{1,3}\$?\d+)(:\$?[A-Za-z]{1,3}\$?\d+)?").expect("valid regex");

    // Step 1: Replace all cross-sheet references with unique placeholders
    let mut placeholder_map: Vec<(String, String)> = Vec::new();
    let mut counter = 0u32;

    let with_placeholders = sheet_cell_re
        .replace_all(formula, |caps: &regex::Captures| {
            let full = caps.get(0).expect("full match").as_str().to_string();
            let placeholder = format!("%%{}%%", counter);
            counter += 1;
            placeholder_map.push((placeholder.clone(), full));
            placeholder
        })
        .to_string();

    // Step 2: Find and offset regular (non-qualified) cell references
    let mut matches: Vec<(usize, usize, String)> = Vec::new();
    for caps in simple_re.captures_iter(&with_placeholders) {
        let full = caps.get(0).expect("full match");
        let start = full.start();
        let end = full.end();

        let replacement = if let Some(range_end_cap) = caps.get(2) {
            let start_ref = caps.get(1).expect("range start").as_str();
            let end_ref = range_end_cap
                .as_str()
                .strip_prefix(':')
                .unwrap_or(range_end_cap.as_str());
            format!(
                "{}:{}",
                offset_single_ref(start_ref, row_offset, col_offset),
                offset_single_ref(end_ref, row_offset, col_offset)
            )
        } else {
            offset_single_ref(
                caps.get(1).expect("single ref").as_str(),
                row_offset,
                col_offset,
            )
        };

        matches.push((start, end, replacement));
    }

    // Apply replacements in reverse to preserve positions
    let mut result = with_placeholders;
    for (start, end, replacement) in matches.into_iter().rev() {
        result.replace_range(start..end, &replacement);
    }

    // Step 3: Restore placeholders
    for (placeholder, original) in placeholder_map.iter().rev() {
        result = result.replace(placeholder, original);
    }

    result
}

/// Offset a single A1-style cell reference like "A1", "$C$3", "A$5", "$B2".
fn offset_single_ref(ref_str: &str, row_offset: i64, col_offset: i64) -> String {
    let re = Regex::new(r"^(\$?)([A-Za-z]{1,3})(\$?)(\d+)$").expect("valid regex");
    let Some(caps) = re.captures(ref_str) else {
        return ref_str.to_string();
    };

    let col_abs = !caps.get(1).expect("col abs marker").as_str().is_empty();
    let col_letters = caps.get(2).expect("col letters").as_str();
    let row_abs = !caps.get(3).expect("row abs marker").as_str().is_empty();
    let row_str = caps.get(4).expect("row digits").as_str();

    let col_idx = cell_ref::col_to_index(col_letters).unwrap_or(0) as i64;
    let row_num: i64 = row_str.parse().unwrap_or(0);

    let new_col_idx = if col_abs {
        col_idx
    } else {
        col_idx + col_offset
    };
    let new_row_num = if row_abs {
        row_num
    } else {
        (row_num + row_offset).max(1)
    };

    if new_col_idx < 0 || new_row_num < 1 {
        return ref_str.to_string();
    }

    let col_prefix = if col_abs { "$" } else { "" };
    let row_prefix = if row_abs { "$" } else { "" };
    format!(
        "{}{}{}{}",
        col_prefix,
        cell_ref::index_to_col(new_col_idx as u16),
        row_prefix,
        new_row_num
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_offset_fully_relative() {
        assert_eq!(offset_single_ref("A1", 1, 0), "A2");
        assert_eq!(offset_single_ref("A1", 0, 1), "B1");
        assert_eq!(offset_single_ref("A1", 1, 1), "B2");
    }

    #[test]
    fn test_offset_absolute_unchanged() {
        assert_eq!(offset_single_ref("$A$1", 1, 0), "$A$1");
        assert_eq!(offset_single_ref("$A$1", 0, 5), "$A$1");
        assert_eq!(offset_single_ref("$A$1", 3, 2), "$A$1");
    }

    #[test]
    fn test_offset_mixed_references() {
        // Col absolute, row relative
        assert_eq!(offset_single_ref("$A1", 1, 0), "$A2");
        assert_eq!(offset_single_ref("$A1", 0, 5), "$A1");
        // Row absolute, col relative
        assert_eq!(offset_single_ref("A$1", 1, 0), "A$1");
        assert_eq!(offset_single_ref("A$1", 0, 1), "B$1");
    }

    #[test]
    fn test_offset_boundary_clamp() {
        // Row cannot go below 1
        assert_eq!(offset_single_ref("A1", -5, 0), "A1");
        // Col cannot go below A
        assert_eq!(offset_single_ref("A5", 0, -5), "A5");
    }

    #[test]
    fn test_offset_multi_letter_cols() {
        assert_eq!(offset_single_ref("Z1", 0, 1), "AA1");
        assert_eq!(offset_single_ref("AA1", 0, 1), "AB1");
    }

    #[test]
    fn test_offset_formula_with_range() {
        let result = offset_cell_references("SUM(A1:B5)", 1, 0);
        assert_eq!(result, "SUM(A2:B6)");
    }

    #[test]
    fn test_offset_formula_mixed() {
        let result = offset_cell_references("IF(A1>10, $B$2, C$3)", 1, 1);
        assert_eq!(result, "IF(B2>10, $B$2, D$3)");
    }

    #[test]
    fn test_offset_formula_cross_sheet_unchanged() {
        let result = offset_cell_references("SUM(Sheet2!A1, Sheet2!B2)", 1, 0);
        // Sheet2! references should NOT be offset
        assert_eq!(result, "SUM(Sheet2!A1, Sheet2!B2)");
    }

    #[test]
    fn test_offset_formula_cross_sheet_plus_local() {
        let result = offset_cell_references("Sheet2!A1 + B1", 1, 0);
        assert_eq!(result, "Sheet2!A1 + B2");
    }
}
