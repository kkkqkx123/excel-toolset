use crate::types::*;

// --- Utilities ---

pub(crate) fn ensure_dimensions(sd: &mut SheetData, row: usize, col: usize) {
    while sd.rows.len() <= row {
        sd.rows.push(Vec::new());
    }
    while sd.rows[row].len() <= col {
        sd.rows[row].push(CellData {
            value: None,
            data_type: CellDataType::Empty,
            formula: None,
        });
    }
}

pub(crate) fn cell_value_to_data(val: &CellValue) -> CellData {
    match val {
        CellValue::String(s) => CellData {
            value: Some(s.clone()),
            data_type: CellDataType::String,
            formula: None,
        },
        CellValue::Number(n) => CellData {
            value: Some(n.to_string()),
            data_type: CellDataType::Float,
            formula: None,
        },
        CellValue::Bool(b) => CellData {
            value: Some(b.to_string()),
            data_type: CellDataType::Bool,
            formula: None,
        },
        CellValue::DateTime(dt) => CellData {
            value: Some(dt.to_string()),
            data_type: CellDataType::DateTime,
            formula: None,
        },
        CellValue::Error(e) => CellData {
            value: Some(e.clone()),
            data_type: CellDataType::Error,
            formula: None,
        },
        CellValue::Empty => CellData {
            value: None,
            data_type: CellDataType::Empty,
            formula: None,
        },
    }
}