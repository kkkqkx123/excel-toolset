use rust_xlsxwriter::{Formula, Table, TableColumn, TableFunction as XlsxTableFunc, TableStyle as XlsxTableStyle};

use crate::excel_write;
use crate::security;
use crate::types::*;

/// Map our TableStylePreset to rust_xlsxwriter TableStyle.
fn map_table_style(preset: &TableStylePreset) -> XlsxTableStyle {
    match preset {
        TableStylePreset::None => XlsxTableStyle::None,
        TableStylePreset::Light1 => XlsxTableStyle::Light1,
        TableStylePreset::Light2 => XlsxTableStyle::Light2,
        TableStylePreset::Light3 => XlsxTableStyle::Light3,
        TableStylePreset::Light4 => XlsxTableStyle::Light4,
        TableStylePreset::Light5 => XlsxTableStyle::Light5,
        TableStylePreset::Light6 => XlsxTableStyle::Light6,
        TableStylePreset::Light7 => XlsxTableStyle::Light7,
        TableStylePreset::Light8 => XlsxTableStyle::Light8,
        TableStylePreset::Light9 => XlsxTableStyle::Light9,
        TableStylePreset::Light10 => XlsxTableStyle::Light10,
        TableStylePreset::Light11 => XlsxTableStyle::Light11,
        TableStylePreset::Light12 => XlsxTableStyle::Light12,
        TableStylePreset::Light13 => XlsxTableStyle::Light13,
        TableStylePreset::Light14 => XlsxTableStyle::Light14,
        TableStylePreset::Light15 => XlsxTableStyle::Light15,
        TableStylePreset::Light16 => XlsxTableStyle::Light16,
        TableStylePreset::Light17 => XlsxTableStyle::Light17,
        TableStylePreset::Light18 => XlsxTableStyle::Light18,
        TableStylePreset::Light19 => XlsxTableStyle::Light19,
        TableStylePreset::Light20 => XlsxTableStyle::Light20,
        TableStylePreset::Light21 => XlsxTableStyle::Light21,
        TableStylePreset::Medium1 => XlsxTableStyle::Medium1,
        TableStylePreset::Medium2 => XlsxTableStyle::Medium2,
        TableStylePreset::Medium3 => XlsxTableStyle::Medium3,
        TableStylePreset::Medium4 => XlsxTableStyle::Medium4,
        TableStylePreset::Medium5 => XlsxTableStyle::Medium5,
        TableStylePreset::Medium6 => XlsxTableStyle::Medium6,
        TableStylePreset::Medium7 => XlsxTableStyle::Medium7,
        TableStylePreset::Medium8 => XlsxTableStyle::Medium8,
        TableStylePreset::Medium9 => XlsxTableStyle::Medium9,
        TableStylePreset::Medium10 => XlsxTableStyle::Medium10,
        TableStylePreset::Medium11 => XlsxTableStyle::Medium11,
        TableStylePreset::Medium12 => XlsxTableStyle::Medium12,
        TableStylePreset::Medium13 => XlsxTableStyle::Medium13,
        TableStylePreset::Medium14 => XlsxTableStyle::Medium14,
        TableStylePreset::Medium15 => XlsxTableStyle::Medium15,
        TableStylePreset::Medium16 => XlsxTableStyle::Medium16,
        TableStylePreset::Medium17 => XlsxTableStyle::Medium17,
        TableStylePreset::Medium18 => XlsxTableStyle::Medium18,
        TableStylePreset::Medium19 => XlsxTableStyle::Medium19,
        TableStylePreset::Medium20 => XlsxTableStyle::Medium20,
        TableStylePreset::Medium21 => XlsxTableStyle::Medium21,
        TableStylePreset::Medium22 => XlsxTableStyle::Medium22,
        TableStylePreset::Medium23 => XlsxTableStyle::Medium23,
        TableStylePreset::Medium24 => XlsxTableStyle::Medium24,
        TableStylePreset::Medium25 => XlsxTableStyle::Medium25,
        TableStylePreset::Medium26 => XlsxTableStyle::Medium26,
        TableStylePreset::Medium27 => XlsxTableStyle::Medium27,
        TableStylePreset::Medium28 => XlsxTableStyle::Medium28,
        TableStylePreset::Dark1 => XlsxTableStyle::Dark1,
        TableStylePreset::Dark2 => XlsxTableStyle::Dark2,
        TableStylePreset::Dark3 => XlsxTableStyle::Dark3,
        TableStylePreset::Dark4 => XlsxTableStyle::Dark4,
        TableStylePreset::Dark5 => XlsxTableStyle::Dark5,
        TableStylePreset::Dark6 => XlsxTableStyle::Dark6,
        TableStylePreset::Dark7 => XlsxTableStyle::Dark7,
        TableStylePreset::Dark8 => XlsxTableStyle::Dark8,
        TableStylePreset::Dark9 => XlsxTableStyle::Dark9,
        TableStylePreset::Dark10 => XlsxTableStyle::Dark10,
        TableStylePreset::Dark11 => XlsxTableStyle::Dark11,
    }
}

/// Map our TotalFunction to rust_xlsxwriter TableFunction.
fn map_total_function(func: &TotalFunction) -> XlsxTableFunc {
    match func {
        TotalFunction::Sum => XlsxTableFunc::Sum,
        TotalFunction::Average => XlsxTableFunc::Average,
        TotalFunction::Count => XlsxTableFunc::Count,
        TotalFunction::CountNums => XlsxTableFunc::CountNumbers,
        TotalFunction::Max => XlsxTableFunc::Max,
        TotalFunction::Min => XlsxTableFunc::Min,
        TotalFunction::StdDev => XlsxTableFunc::StdDev,
        TotalFunction::Var => XlsxTableFunc::Var,
        TotalFunction::Custom(label) => XlsxTableFunc::Custom(Formula::new(label.as_str())),
    }
}

/// Create an Excel table (ListObject) on a worksheet.
pub fn create_table(
    path: &str,
    config: &TableConfig,
    params: &SecurityParams,
) -> Result<WriteResult> {
    if params.dry_run {
        return Ok(WriteResult::dry_run_success());
    }

    security::create_backup_if_needed(params)?;

    let (r1, c1, r2, c2) = crate::utils::cell_ref::parse_range(&config.range)?;

    excel_write::modify_file_with_wb(path, params, |_, wb| {
        let mut table = Table::new();
        table = table.set_name(&config.name);
        table = table.set_style(map_table_style(&config.style));

        if config.has_header {
            table = table.set_header_row(true);
        }
        if config.has_total {
            table = table.set_total_row(true);
        }

        // Set total row functions if specified
        if let Some(ref total_funcs) = config.total_row_functions {
            let columns: Vec<TableColumn> = total_funcs
                .iter()
                .map(|tf| {
                    let mut col = TableColumn::new();
                    col = col.set_total_function(map_total_function(&tf.function));
                    if let TotalFunction::Custom(ref label) = tf.function {
                        col = col.set_total_label(label);
                    }
                    col
                })
                .collect();
            table = table.set_columns(&columns);
        }

        // Find the target worksheet
        let worksheet = if let Some(ref sheet_name) = config.sheet {
            wb.worksheet_from_name(sheet_name)
                .map_err(|_e| AppError::SheetNotFound(sheet_name.clone()))?
        } else {
            // Use the first worksheet as default
            wb.worksheet_from_index(0)
                .map_err(|_e| AppError::Custom("No worksheets found".to_string()))?
        };

        worksheet
            .add_table(r1, c1, r2, c2, &table)
            .map_err(AppError::Xlsx)?;

        Ok(())
    })
}

/// Remove a table from the workbook.
/// Since rust_xlsxwriter rebuilds the workbook, the table is removed by not re-adding it.
pub fn remove_table(
    path: &str,
    _table_name: &str,
    params: &SecurityParams,
) -> Result<WriteResult> {
    if params.dry_run {
        return Ok(WriteResult::dry_run_success());
    }

    security::create_backup_if_needed(params)?;

    // Rebuilding the workbook without re-adding the table effectively removes it.
    excel_write::modify_file_with_wb(path, params, |_, _| Ok(()))
}
