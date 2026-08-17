use crate::cli::args::*;
use excel_core::excel_read;
use excel_core::excel_write;
use excel_core::types::*;
use excel_core::utils::helpers;

pub(crate) fn run_cell(args: &CellArgs) -> Result<serde_json::Value> {
    match &args.command {
        CellSub::Read { path, sheet, cell } => {
            let (row, col) = excel_core::utils::cell_ref::parse_cell_ref(cell)?;
            let data = excel_read::read_cell(path, sheet, row, col)?;
            Ok(serde_json::to_value(data).map_err(|e| AppError::Serialize(e.to_string()))?)
        }
        CellSub::Write {
            path,
            sheet,
            cell,
            value,
            dry_run,
        } => {
            let (row, col) = excel_core::utils::cell_ref::parse_cell_ref(cell)?;
            let params = SecurityParams {
                dry_run: *dry_run,
                create_backup: true,
                file_path: path.clone(),
            };
            let cell_value = helpers::parse_cell_value(value);
            let result = excel_write::write_cell(path, &params, sheet, row, col, &cell_value)?;
            Ok(serde_json::to_value(result).map_err(|e| AppError::Serialize(e.to_string()))?)
        }
    }
}
