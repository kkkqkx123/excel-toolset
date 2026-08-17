use crate::cli::args::*;
use excel_core::features::workbook_overview;
use excel_core::types::*;

pub(crate) fn run_history(args: &HistoryArgs) -> Result<serde_json::Value> {
    let history = workbook_overview::list_workbook_history(&args.path)?;
    serde_json::to_value(history).map_err(|e| AppError::Serialize(e.to_string()))
}

// ── Freeze Pane ──
