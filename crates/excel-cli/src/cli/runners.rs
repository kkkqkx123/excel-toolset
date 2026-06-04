use chrono::Utc;

use excel_core::api;
use excel_core::excel_data;
use excel_core::excel_read;
use excel_core::excel_write;
use excel_core::helpers;
use excel_core::security;
use excel_core::types::*;
use excel_core::vba_util;
use excel_diff::diff_files;
use excel_diff::diff_range;
use excel_diff::diff_sheets;
use excel_diff::git_driver;
use excel_diff::semantic::{self, Verbosity};
use excel_diff::summarize;

use super::args::*;

pub fn execute(cli: &Cli) {
    let result = run_command(cli);
    match result {
        Ok(json) => {
            if cli.format == "text" {
                if let Some(text) = json.get("raw_text").and_then(|v| v.as_str()) {
                    println!("{}", text);
                    return;
                }
                eprintln!(
                    "Warning: --format text is only supported for diff commands. \
                     Showing JSON output."
                );
            }
            if cli.pretty {
                println!("{}", serde_json::to_string_pretty(&json).unwrap());
            } else {
                println!("{}", serde_json::to_string(&json).unwrap());
            }
        }
        Err(e) => {
            let err_json = serde_json::json!({
                "success": false,
                "message": e.to_string()
            });
            if cli.pretty {
                println!("{}", serde_json::to_string_pretty(&err_json).unwrap());
            } else {
                println!("{}", serde_json::to_string(&err_json).unwrap());
            }
        }
    }
}

fn run_command(cli: &Cli) -> Result<serde_json::Value> {
    match &cli.command {
        Commands::File(args) => run_file(args),
        Commands::Sheet(args) => run_sheet(args),
        Commands::Cell(args) => run_cell(args),
        Commands::Range(args) => run_range(args),
        Commands::Data(args) => run_data(args),
        Commands::Formula(args) => run_formula(args),
        Commands::Format(args) => run_format(args),
        Commands::Chart(args) => run_chart(args),
        Commands::Vba(args) => run_vba(args),
        Commands::Diff(args) => run_diff(args, &cli.format),
        Commands::Batch(args) => run_batch(args, &cli.format),
        Commands::Rollback(args) => run_rollback(args),
    }
}

fn run_file(args: &FileArgs) -> Result<serde_json::Value> {
    match &args.command {
        FileSub::Create { path, sheet } => {
            let result = excel_write::create_file(path, sheet)?;
            Ok(serde_json::to_value(result).map_err(|e| AppError::Serialize(e.to_string()))?)
        }
        FileSub::Info { path } => {
            let info = excel_read::read_file_info(path)?;
            Ok(serde_json::to_value(info).map_err(|e| AppError::Serialize(e.to_string()))?)
        }
        FileSub::Backup { path, output } => {
            let hash = security::compute_file_hash(path)?;
            let backup = security::create_backup(path, &hash)?;
            if let Some(out) = output {
                std::fs::copy(&backup.backup_path, out)?;
            }
            Ok(serde_json::json!({
                "success": true,
                "backup_path": backup.backup_path,
                "timestamp": backup.timestamp,
                "file_hash": backup.file_hash
            }))
        }
    }
}

fn run_sheet(args: &SheetArgs) -> Result<serde_json::Value> {
    let params = SecurityParams {
        dry_run: false,
        create_backup: true,
        file_path: String::new(),
    };
    match &args.command {
        SheetSub::List { path } => {
            let sheets = excel_read::list_sheets(path)?;
            Ok(serde_json::json!({ "success": true, "sheets": sheets }))
        }
        SheetSub::Add { path, name } => {
            let result = excel_write::add_sheet(path, &params, name)?;
            Ok(serde_json::to_value(result).map_err(|e| AppError::Serialize(e.to_string()))?)
        }
        SheetSub::Delete { path, name } => {
            let result = excel_write::delete_sheet(path, &params, name)?;
            Ok(serde_json::to_value(result).map_err(|e| AppError::Serialize(e.to_string()))?)
        }
        SheetSub::Rename { path, old, new } => {
            let result = excel_write::rename_sheet(path, &params, old, new)?;
            Ok(serde_json::to_value(result).map_err(|e| AppError::Serialize(e.to_string()))?)
        }
    }
}

fn run_cell(args: &CellArgs) -> Result<serde_json::Value> {
    match &args.command {
        CellSub::Read { path, sheet, cell } => {
            let (row, col) = excel_core::cell_ref::parse_cell_ref(cell)?;
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
            let (row, col) = excel_core::cell_ref::parse_cell_ref(cell)?;
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

fn run_range(args: &RangeArgs) -> Result<serde_json::Value> {
    match &args.command {
        RangeSub::Read { path, sheet, range } => {
            let data = excel_read::read_range(path, sheet, range)?;
            Ok(serde_json::to_value(data).map_err(|e| AppError::Serialize(e.to_string()))?)
        }
        RangeSub::Write {
            path,
            sheet,
            range,
            data,
            dry_run,
        } => {
            let values: Vec<Vec<CellValue>> = helpers::parse_cell_value_grid(data)?;
            let params = SecurityParams {
                dry_run: *dry_run,
                create_backup: true,
                file_path: path.clone(),
            };
            let result = excel_write::write_range(path, &params, sheet, range, &values)?;
            Ok(serde_json::to_value(result).map_err(|e| AppError::Serialize(e.to_string()))?)
        }
        RangeSub::Clear {
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
            let result = excel_write::clear_range(path, &params, sheet, range)?;
            Ok(serde_json::to_value(result).map_err(|e| AppError::Serialize(e.to_string()))?)
        }
        RangeSub::WriteCsv {
            path,
            sheet,
            range,
            csv,
            dry_run,
        } => {
            let params = SecurityParams {
                dry_run: *dry_run,
                create_backup: true,
                file_path: path.clone(),
            };
            let result = excel_write::write_range_from_csv(path, &params, sheet, range, csv)?;
            Ok(serde_json::to_value(result).map_err(|e| AppError::Serialize(e.to_string()))?)
        }
    }
}

fn run_data(args: &DataArgs) -> Result<serde_json::Value> {
    match &args.command {
        DataSub::AppendRow {
            path,
            sheet,
            values,
            dry_run,
        } => {
            let cell_values: Vec<Vec<CellValue>> = vec![
                values
                    .iter()
                    .map(|v| helpers::parse_cell_value(v))
                    .collect(),
            ];
            let params = SecurityParams {
                dry_run: *dry_run,
                create_backup: true,
                file_path: path.clone(),
            };
            let result = excel_data::append_rows(path, &params, sheet, &cell_values)?;
            Ok(serde_json::to_value(result).map_err(|e| AppError::Serialize(e.to_string()))?)
        }
        DataSub::InsertRow {
            path,
            sheet,
            row,
            values,
            dry_run,
        } => {
            let cell_values: Vec<Vec<CellValue>> = vec![
                values
                    .iter()
                    .map(|v| helpers::parse_cell_value(v))
                    .collect(),
            ];
            let params = SecurityParams {
                dry_run: *dry_run,
                create_backup: true,
                file_path: path.clone(),
            };
            let result = excel_data::insert_rows(path, &params, sheet, *row, &cell_values)?;
            Ok(serde_json::to_value(result).map_err(|e| AppError::Serialize(e.to_string()))?)
        }
        DataSub::DeleteRow {
            path,
            sheet,
            row,
            dry_run,
        } => {
            let params = SecurityParams {
                dry_run: *dry_run,
                create_backup: true,
                file_path: path.clone(),
            };
            let result = excel_data::delete_rows(path, &params, sheet, *row, *row)?;
            Ok(serde_json::to_value(result).map_err(|e| AppError::Serialize(e.to_string()))?)
        }
        DataSub::Filter {
            path,
            sheet,
            column,
            op,
            value,
        } => {
            let filter_op = helpers::parse_filter_op(op)?;
            let conditions = vec![FilterCondition {
                column: *column,
                operator: filter_op,
                value: value.clone(),
            }];
            let result = api::filter_rows(path, sheet, &conditions)?;
            Ok(serde_json::to_value(result).map_err(|e| AppError::Serialize(e.to_string()))?)
        }
        DataSub::Sort {
            path,
            sheet,
            column,
            desc,
            dry_run,
        } => {
            let sort_cols = vec![SortColumn {
                column: *column,
                descending: *desc,
            }];
            let params = SecurityParams {
                dry_run: *dry_run,
                create_backup: true,
                file_path: path.clone(),
            };
            let result = api::sort_sheet(path, &params, sheet, &sort_cols)?;
            Ok(serde_json::to_value(result).map_err(|e| AppError::Serialize(e.to_string()))?)
        }
        DataSub::Dedup {
            path,
            sheet,
            column,
            dry_run,
        } => {
            let cols: Vec<u16> = column.map(|c| vec![c]).unwrap_or_default();
            let params = SecurityParams {
                dry_run: *dry_run,
                create_backup: true,
                file_path: path.clone(),
            };
            let result = api::dedup_sheet(path, &params, sheet, &cols)?;
            Ok(serde_json::to_value(result).map_err(|e| AppError::Serialize(e.to_string()))?)
        }
        DataSub::Sql { path, sheet, query } => {
            let result = api::sql_query(path, sheet, query)?;
            Ok(serde_json::to_value(result).map_err(|e| AppError::Serialize(e.to_string()))?)
        }
    }
}

fn run_formula(args: &FormulaArgs) -> Result<serde_json::Value> {
    match &args.command {
        FormulaSub::Set {
            path,
            sheet,
            cell,
            formula,
            dry_run,
        } => {
            let params = SecurityParams {
                dry_run: *dry_run,
                create_backup: true,
                file_path: path.clone(),
            };
            let result = excel_write::set_formula(path, &params, sheet, cell, formula)?;
            Ok(serde_json::to_value(result).map_err(|e| AppError::Serialize(e.to_string()))?)
        }
        FormulaSub::Refresh {
            path,
            sheet,
            dry_run,
        } => {
            let params = SecurityParams {
                dry_run: *dry_run,
                create_backup: true,
                file_path: path.clone(),
            };
            let result = excel_write::refresh_formulas(path, &params, sheet)?;
            Ok(serde_json::to_value(result).map_err(|e| AppError::Serialize(e.to_string()))?)
        }
    }
}

fn run_format(args: &FormatArgs) -> Result<serde_json::Value> {
    match &args.command {
        FormatSub::Set {
            path,
            sheet,
            range,
            style,
            dry_run,
        } => {
            let style_val: Style = serde_json::from_str(style)
                .map_err(|e| AppError::Serialize(format!("Invalid style JSON: {}", e)))?;
            let params = SecurityParams {
                dry_run: *dry_run,
                create_backup: true,
                file_path: path.clone(),
            };
            let result = excel_write::set_format(path, &params, sheet, range, &style_val)?;
            Ok(serde_json::to_value(result).map_err(|e| AppError::Serialize(e.to_string()))?)
        }
        FormatSub::Merge {
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
            let result = excel_write::merge_cells(path, &params, sheet, range, "")?;
            Ok(serde_json::to_value(result).map_err(|e| AppError::Serialize(e.to_string()))?)
        }
    }
}

fn run_chart(args: &ChartArgs) -> Result<serde_json::Value> {
    match &args.command {
        ChartSub::Create {
            path,
            sheet,
            range,
            chart_type,
            title,
            dry_run,
        } => {
            let ct = helpers::chart_type_from_str(chart_type)?;
            let (r1, c1, _, _) = excel_core::cell_ref::parse_range(range)?;
            let config = ChartConfig {
                chart_type: ct,
                title: title.clone(),
                categories_range: range.clone(),
                values_range: range.clone(),
                sheet: sheet.clone(),
                row: r1,
                col: c1,
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

fn run_vba(args: &VbaArgs) -> Result<serde_json::Value> {
    match &args.command {
        VbaSub::Export { path, output } => {
            let data = vba_util::export_vba(path)?;
            std::fs::write(output, &data)?;
            Ok(serde_json::json!({
                "success": true,
                "message": format!("VBA exported to {}", output)
            }))
        }
        VbaSub::Import {
            path,
            vba_file,
            dry_run,
        } => {
            let data = std::fs::read(vba_file)?;
            let params = SecurityParams {
                dry_run: *dry_run,
                create_backup: true,
                file_path: path.clone(),
            };
            let result = vba_util::import_vba(path, &params, &data)?;
            Ok(serde_json::to_value(result).map_err(|e| AppError::Serialize(e.to_string()))?)
        }
    }
}

fn run_batch(args: &BatchArgs, format: &str) -> Result<serde_json::Value> {
    match &args.command {
        BatchSub::Modify {
            path,
            operations,
            dry_run,
        } => {
            let ops: Vec<BatchOperation> = serde_json::from_str(operations)
                .map_err(|e| AppError::Serialize(format!("Invalid operations JSON: {}", e)))?;
            let params = SecurityParams {
                dry_run: *dry_run,
                create_backup: true,
                file_path: path.clone(),
            };
            let mut result = excel_write::execute_batch_operations(path, &params, &ops)?;
            if let Some(ref backup) = result.backup_info
                && let Ok(diff) = excel_diff::diff_files(&backup.backup_path, path)
            {
                result.diff = Some(diff);
            }
            if format == "text" {
                let mut parts = Vec::new();
                if !result.message.is_empty() {
                    parts.push(result.message.clone());
                }
                if let Some(ref diff) = result.diff {
                    parts.push(semantic::to_natural_text(diff, None, Verbosity::Detail));
                }
                let text = if parts.is_empty() {
                    "Batch modify completed.".to_string()
                } else {
                    parts.join("\n")
                };
                Ok(serde_json::json!({"raw_text": text}))
            } else {
                Ok(serde_json::to_value(result).map_err(|e| AppError::Serialize(e.to_string()))?)
            }
        }
    }
}

fn run_diff(args: &DiffArgs, format: &str) -> Result<serde_json::Value> {
    match &args.command {
        DiffSub::File {
            old_path,
            new_path,
            sheet,
        } => {
            let diff = match sheet {
                Some(s) => {
                    let sd = diff_sheets(old_path, new_path, s)?;
                    let summary = summarize::summarize(std::slice::from_ref(&sd));
                    FileDiff {
                        file_hash_match: false,
                        sheet_diffs: vec![sd],
                        summary,
                    }
                }
                None => diff_files(old_path, new_path)?,
            };

            if format == "text" {
                let text = semantic::to_natural_text(&diff, None, Verbosity::Detail);
                Ok(serde_json::json!({"raw_text": text}))
            } else {
                Ok(serde_json::to_value(diff).map_err(|e| AppError::Serialize(e.to_string()))?)
            }
        }
        DiffSub::Range {
            old_path,
            new_path,
            sheet,
            range,
        } => {
            let diff = diff_range(old_path, new_path, sheet, range)?;
            if format == "text" {
                let sd = SheetDiff {
                    sheet_name: sheet.clone(),
                    row_count_diff: 0,
                    col_count_diff: 0,
                    cell_diffs: diff.cell_diffs.clone(),
                };
                let summary = summarize::summarize(std::slice::from_ref(&sd));
                let fd = FileDiff {
                    file_hash_match: false,
                    sheet_diffs: vec![sd],
                    summary,
                };
                let text = semantic::to_natural_text(&fd, None, Verbosity::Detail);
                Ok(serde_json::json!({"raw_text": text}))
            } else {
                Ok(serde_json::to_value(diff).map_err(|e| AppError::Serialize(e.to_string()))?)
            }
        }
        DiffSub::InstallGitDriver {} => {
            git_driver::install_git_driver()?;
            Ok(serde_json::json!({ "success": true, "message": "Git diff driver installed" }))
        }
        DiffSub::UninstallGitDriver {} => {
            git_driver::uninstall_git_driver()?;
            Ok(serde_json::json!({ "success": true, "message": "Git diff driver uninstalled" }))
        }
    }
}

fn run_rollback(args: &RollbackArgs) -> Result<serde_json::Value> {
    let backup = BackupInfo {
        backup_path: args.backup_path.clone(),
        timestamp: Utc::now(),
        operation: "rollback".into(),
        file_hash: String::new(),
    };
    security::rollback(&backup, &args.path)?;
    Ok(serde_json::json!({
        "success": true,
        "message": format!("Rolled back {} from {}", args.path, args.backup_path)
    }))
}
