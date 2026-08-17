use crate::cli::args::*;
use excel_core::excel_write;
use excel_core::types::*;
use excel_core::utils::helpers;

pub(crate) fn run_chart(args: &ChartArgs) -> Result<serde_json::Value> {
    match &args.command {
        ChartSub::Create {
            path,
            sheet,
            range,
            chart_type,
            title,
            position,
            dry_run,
            ..
        } => {
            let ct = helpers::chart_type_from_str(chart_type)?;
            let (r1, c1, r2, c2) = excel_core::utils::cell_ref::parse_range(range)?;
            let (chart_row, chart_col) = if let Some(pos) = position {
                let (pr, pc) = excel_core::utils::cell_ref::parse_cell_ref(pos)?;
                (pr, pc)
            } else {
                (r2 + 1, c1)
            };
            // Build sheet-qualified range strings for rust_xlsxwriter
            // Use first column as categories, remaining columns as values
            let categories_range = format!(
                "'{}'!${}${}:${}${}",
                sheet,
                excel_core::utils::cell_ref::index_to_col(c1),
                r1 + 1,
                excel_core::utils::cell_ref::index_to_col(c1),
                r2 + 1
            );
            let values_range = if c2 > c1 {
                format!(
                    "'{}'!${}${}:${}${}",
                    sheet,
                    excel_core::utils::cell_ref::index_to_col(c1 + 1),
                    r1 + 1,
                    excel_core::utils::cell_ref::index_to_col(c2),
                    r2 + 1
                )
            } else {
                categories_range.clone()
            };
            let config = ChartConfig {
                chart_type: ct,
                title: title.clone(),
                categories_range,
                values_range,
                sheet: sheet.clone(),
                row: chart_row,
                col: chart_col,
                trendline: None,
                y_error_bars: None,
                x_error_bars: None,
                log_base: None,
            };
            let params = SecurityParams {
                dry_run: *dry_run,
                create_backup: true,
                file_path: path.clone(),
            };
            let result = excel_write::add_chart(path, &params, &config)?;
            Ok(serde_json::to_value(result).map_err(|e| AppError::Serialize(e.to_string()))?)
        }
    }
}
