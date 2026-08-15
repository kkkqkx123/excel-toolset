use excel_types::CellValue;
use crate::evaluator::{cell_value_to_string, to_number};

use super::*;

pub(crate) fn lookup_choose(args: &[CellValue]) -> CellValue {
    if args.is_empty() {
        return CellValue::Error("#VALUE!".into());
    }

    let idx = match to_number(&args[0]) {
        Some(n) if n >= 1.0 => n as usize,
        _ => return CellValue::Error("#VALUE!".into()),
    };

    if idx >= args.len() {
        CellValue::Error("#VALUE!".into())
    } else {
        args[idx].clone()
    }
}

// ---------------------------------------------------------------------------
// ADDRESS(row_num, column_num, [abs_num], [a1], [sheet_text])
// ---------------------------------------------------------------------------


pub(crate) fn lookup_address(args: &[CellValue]) -> CellValue {
    let row = args.first().and_then(to_number).unwrap_or(1.0) as u32;
    let col = args.get(1).and_then(to_number).unwrap_or(1.0) as u32;
    let abs_num = args.get(2).and_then(to_number).unwrap_or(1.0) as u32;
    let _a1 = args
        .get(3)
        .is_none_or(|v| !matches!(v, CellValue::Bool(false)));
    let sheet = args.get(4).map(cell_value_to_string);

    if row == 0 || col == 0 {
        return CellValue::Error("#VALUE!".into());
    }

    let col_str = col_index_to_letters(col.saturating_sub(1));

    let cell = match abs_num {
        1 => format!("${}${}", col_str, row), // Absolute
        2 => format!("{}${}", col_str, row),  // Row absolute
        3 => format!("${}{}", col_str, row),  // Col absolute
        4 => format!("{}{}", col_str, row),   // Relative
        _ => format!("${}${}", col_str, row),
    };

    let result = if let Some(s) = sheet {
        format!("{}!{}", s, cell)
    } else {
        cell
    };

    CellValue::String(result)
}

