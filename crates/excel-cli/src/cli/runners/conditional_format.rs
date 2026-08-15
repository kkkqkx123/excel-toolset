use excel_core::features::conditional_format;
use excel_core::types::*;
use crate::cli::args::*;

pub(crate) fn run_conditional_format(args: &ConditionalFormatArgs) -> Result<serde_json::Value> {
    match &args.command {
        ConditionalFormatSub::Add {
            path,
            sheet,
            range,
            rule_type,
            condition,
            style,
            config,
            dry_run,
        } => {
            let params = SecurityParams {
                dry_run: *dry_run,
                create_backup: true,
                file_path: path.clone(),
            };

            let rt = conditional_format::parse_rule_type(rule_type);

            let parsed_style: Option<Style> = if let Some(s) = style {
                Some(
                    serde_json::from_str(s)
                        .map_err(|e| AppError::Serialize(format!("Invalid style JSON: {}", e)))?,
                )
            } else {
                None
            };

            let parsed_config: Option<conditional_format::ConditionalFormatConfig> =
                if let Some(c) = config {
                    Some(
                        serde_json::from_str(c).map_err(|e| {
                            AppError::Serialize(format!("Invalid config JSON: {}", e))
                        })?,
                    )
                } else {
                    None
                };

            let rule = conditional_format::ConditionalFormatRule {
                rule_type: rt,
                condition: condition.clone(),
                format: parsed_style,
                config: parsed_config,
            };

            let result =
                conditional_format::add_conditional_format(path, sheet, range, &rule, &params)?;
            Ok(serde_json::to_value(result).map_err(|e| AppError::Serialize(e.to_string()))?)
        }
        ConditionalFormatSub::Remove {
            path,
            sheet,
            range,
            dry_run,
        } => {
            let params = SecurityParams {
                dry_run: *dry_run,
                create_backup: true,
                file_path: path.clone(),
            };
            let result =
                conditional_format::remove_conditional_format(path, sheet, range, &params)?;
            Ok(serde_json::to_value(result).map_err(|e| AppError::Serialize(e.to_string()))?)
        }
    }
}

// ── Table ──

