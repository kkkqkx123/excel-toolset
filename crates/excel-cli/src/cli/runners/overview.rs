use excel_core::features::workbook_overview;
use excel_core::types::*;
use crate::cli::args::*;

pub(crate) fn run_overview(args: &OverviewArgs) -> Result<serde_json::Value> {
    if args.blueprint {
        let bp = workbook_overview::get_workbook_blueprint(&args.path)?;
        Ok(serde_json::to_value(bp).map_err(|e| AppError::Serialize(e.to_string()))?)
    } else {
        let overview = workbook_overview::get_workbook_overview(&args.path)?;
        Ok(serde_json::to_value(overview).map_err(|e| AppError::Serialize(e.to_string()))?)
    }
}

