use excel_types::CellValue;
use crate::engine::DataProvider;
use crate::evaluator::{cell_value_to_string, to_number};

use super::*;

pub(crate) fn lookup_offset(args: &[CellValue], _provider: &dyn DataProvider) -> CellValue {
    // OFFSET requires knowledge of the original cell coordinates, which are
    // not available from evaluated CellValues alone. When a concrete cell-
    // coordinate-aware call site is added this function can be made to use
    // the DataProvider.
    let _ref = args.first();
    let _rows = args.get(1).and_then(to_number).unwrap_or(0.0);
    let _cols = args.get(2).and_then(to_number).unwrap_or(0.0);
    CellValue::Error("#REF!".into())
}

// ---------------------------------------------------------------------------
// INDIRECT(ref_text, [a1])
// ---------------------------------------------------------------------------


pub(crate) fn lookup_indirect(args: &[CellValue], provider: &dyn DataProvider) -> CellValue {
    let ref_text = args
        .first()
        .map(cell_value_to_string)
        .unwrap_or_default();
    // A1 flag: if FALSE, use R1C1; not yet supported.
    let _a1 = args
        .get(1)
        .is_none_or(|v| !matches!(v, CellValue::Bool(false)));

    if ref_text.is_empty() {
        return CellValue::Error("#REF!".into());
    }

    // Parse A1 notation
    let (sheet, row, col) = match parse_a1_ref(&ref_text) {
        Some(t) => t,
        None => return CellValue::Error("#REF!".into()),
    };

    let sheet_name = if sheet.is_empty() {
        // No sheet specified — the provider may or may not handle this.
        // We need a sheet name to call get_cell.
        return CellValue::Error("#REF!".into());
    } else {
        sheet
    };

    provider
        .get_cell(&sheet_name, row, col)
        .unwrap_or(CellValue::Error("#REF!".into()))
}

// ---------------------------------------------------------------------------
// ROW([reference])
// ---------------------------------------------------------------------------

