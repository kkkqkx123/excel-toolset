use chrono::Utc;

use excel_core::excel_read;
use excel_core::excel_write;
use excel_core::features::comments;
use excel_core::features::conditional_format;
use excel_core::features::formula_analysis;
use excel_core::features::named_ranges;
use excel_core::features::search;
use excel_core::features::vba_util;
use excel_core::operations;
use excel_core::security;
use excel_core::types::*;
use excel_core::utils::helpers;
use excel_diff::diff_files;
use excel_diff::diff_range;
use excel_diff::diff_sheets;
use excel_diff::get_git_diff_file_paths;
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
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json)
                        .expect("JSON serialization of Value should never fail")
                );
            } else {
                println!(
                    "{}",
                    serde_json::to_string(&json)
                        .expect("JSON serialization of Value should never fail")
                );
            }
        }
        Err(e) => {
            let err_json = serde_json::json!({
                "success": false,
                "message": e.to_string()
            });
            if cli.pretty {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&err_json)
                        .expect("JSON serialization of Value should never fail")
                );
            } else {
                println!(
                    "{}",
                    serde_json::to_string(&err_json)
                        .expect("JSON serialization of Value should never fail")
                );
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
        Commands::Comments(args) => run_comments(args),
        Commands::NamedRange(args) => run_named_range(args),
        Commands::Search(args) => run_search(args),
        Commands::ConditionalFormat(args) => run_conditional_format(args),
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
    match &args.command {
        SheetSub::List { path } => {
            let sheets = excel_read::list_sheets(path)?;
            Ok(serde_json::json!({ "success": true, "sheets": sheets }))
        }
        SheetSub::Add { path, name } => {
            let params = SecurityParams {
                dry_run: false,
                create_backup: true,
                file_path: path.clone(),
            };
            let result = excel_write::add_sheet(path, &params, name)?;
            Ok(serde_json::to_value(result).map_err(|e| AppError::Serialize(e.to_string()))?)
        }
        SheetSub::Delete { path, name } => {
            let params = SecurityParams {
                dry_run: false,
                create_backup: true,
                file_path: path.clone(),
            };
            let result = excel_write::delete_sheet(path, &params, name)?;
            Ok(serde_json::to_value(result).map_err(|e| AppError::Serialize(e.to_string()))?)
        }
        SheetSub::Rename { path, old, new } => {
            let params = SecurityParams {
                dry_run: false,
                create_backup: true,
                file_path: path.clone(),
            };
            let result = excel_write::rename_sheet(path, &params, old, new)?;
            Ok(serde_json::to_value(result).map_err(|e| AppError::Serialize(e.to_string()))?)
        }
    }
}

fn run_cell(args: &CellArgs) -> Result<serde_json::Value> {
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
            let result = excel_write::append_rows(path, &params, sheet, &cell_values)?;
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
            // CLI row numbers are 1-indexed, internal functions use 0-indexed
            let row_idx = row.saturating_sub(1);
            let result = excel_write::insert_rows(path, &params, sheet, row_idx, &cell_values)?;
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
            // CLI row numbers are 1-indexed, internal functions use 0-indexed
            let row_idx = row.saturating_sub(1);
            let result = excel_write::delete_rows(path, &params, sheet, row_idx, row_idx)?;
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
            let col_idx = column.saturating_sub(1);
            let conditions = vec![FilterCondition {
                column: col_idx,
                operator: filter_op,
                value: value.clone(),
            }];
            let result = operations::filter_rows(path, sheet, &conditions)?;
            Ok(serde_json::json!({
                "success": true,
                "rows": result
            }))
        }
        DataSub::Sort {
            path,
            sheet,
            column,
            desc,
            dry_run,
        } => {
            let col_idx = column.saturating_sub(1);
            let sort_cols = vec![SortColumn {
                column: col_idx,
                descending: *desc,
            }];
            let params = SecurityParams {
                dry_run: *dry_run,
                create_backup: true,
                file_path: path.clone(),
            };
            let result = operations::sort_sheet(path, &params, sheet, &sort_cols)?;
            Ok(serde_json::to_value(result).map_err(|e| AppError::Serialize(e.to_string()))?)
        }
        DataSub::Dedup {
            path,
            sheet,
            column,
            dry_run,
        } => {
            let cols: Vec<u16> = column
                .map(|c| vec![c.saturating_sub(1)])
                .unwrap_or_default();
            let params = SecurityParams {
                dry_run: *dry_run,
                create_backup: true,
                file_path: path.clone(),
            };
            let result = operations::dedup_sheet(path, &params, sheet, &cols)?;
            Ok(serde_json::to_value(result).map_err(|e| AppError::Serialize(e.to_string()))?)
        }
        DataSub::Sql { path, sheet, query } => {
            let result = operations::sql_query(path, sheet, query)?;
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
        FormulaSub::Read { path, sheet, cell } => {
            let formula = excel_read::read_formula(path, sheet, cell)?;
            Ok(serde_json::json!({
                "success": true,
                "formula": formula
            }))
        }
        FormulaSub::CalcMode {
            path,
            mode,
            dry_run,
        } => {
            let params = SecurityParams {
                dry_run: *dry_run,
                create_backup: true,
                file_path: path.clone(),
            };
            let result = excel_write::set_calculation_mode(path, &params, mode)?;
            Ok(serde_json::to_value(result).map_err(|e| AppError::Serialize(e.to_string()))?)
        }
        FormulaSub::Trace { path, sheet, cell } => {
            let trace = formula_analysis::trace_dependencies(path, sheet, cell)?;
            Ok(serde_json::to_value(trace).map_err(|e| AppError::Serialize(e.to_string()))?)
        }
        FormulaSub::Explain {
            path,
            sheet,
            cell,
            language,
        } => {
            let explanation = formula_analysis::explain_formula(path, sheet, cell, language)?;
            Ok(
                serde_json::to_value(explanation)
                    .map_err(|e| AppError::Serialize(e.to_string()))?,
            )
        }
        FormulaSub::ExplainLogic {
            path,
            sheet,
            cell,
            language,
        } => {
            let logic = formula_analysis::explain_formula_logic(path, sheet, cell, language)?;
            Ok(serde_json::to_value(logic).map_err(|e| AppError::Serialize(e.to_string()))?)
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
            value,
            dry_run,
        } => {
            let params = SecurityParams {
                dry_run: *dry_run,
                create_backup: true,
                file_path: path.clone(),
            };
            let merge_value = value.as_deref().unwrap_or("");
            let result = excel_write::merge_cells(path, &params, sheet, range, merge_value)?;
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
            position,
            dry_run,
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
        VbaSub::Has { path } => {
            let has = vba_util::has_vba(path)?;
            Ok(serde_json::json!({
                "success": true,
                "has_vba": has
            }))
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
        DiffSub::GitDriver => {
            // Get file paths from environment variables or command line arguments
            let (old_path, new_path) = get_git_diff_file_paths()?;

            // Perform diff and output in text format (required by git diff driver)
            let diff = diff_files(&old_path, &new_path)?;

            // Git diff driver expects text output, not JSON
            let text = semantic::to_natural_text(&diff, None, Verbosity::Detail);
            println!("{}", text);

            // Return empty JSON since we already printed the text
            Ok(serde_json::json!({}))
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
    let hash = security::compute_file_hash(&args.backup_path).map_err(AppError::Io)?;
    let backup = BackupInfo {
        backup_path: args.backup_path.clone(),
        timestamp: Utc::now(),
        operation: "rollback".into(),
        file_hash: hash,
    };
    security::rollback(&backup, &args.path)?;
    Ok(serde_json::json!({
        "success": true,
        "message": format!("Rolled back {} from {}", args.path, args.backup_path)
    }))
}

// ── Comments ──

fn run_comments(args: &CommentsArgs) -> Result<serde_json::Value> {
    match &args.command {
        CommentsSub::Get { path, sheet, cell } => {
            let comment = comments::get_comment(path, sheet, cell)?;
            Ok(serde_json::to_value(comment).map_err(|e| AppError::Serialize(e.to_string()))?)
        }
        CommentsSub::Add {
            path,
            sheet,
            cell,
            text,
            dry_run,
        } => {
            let params = SecurityParams {
                dry_run: *dry_run,
                create_backup: true,
                file_path: path.clone(),
            };
            let result = comments::add_comment(path, sheet, cell, text, &params)?;
            Ok(serde_json::to_value(result).map_err(|e| AppError::Serialize(e.to_string()))?)
        }
        CommentsSub::Update {
            path,
            sheet,
            cell,
            text,
            dry_run,
        } => {
            let params = SecurityParams {
                dry_run: *dry_run,
                create_backup: true,
                file_path: path.clone(),
            };
            let result = comments::update_comment(path, sheet, cell, text, &params)?;
            Ok(serde_json::to_value(result).map_err(|e| AppError::Serialize(e.to_string()))?)
        }
        CommentsSub::Delete {
            path,
            sheet,
            cell,
            dry_run,
        } => {
            let params = SecurityParams {
                dry_run: *dry_run,
                create_backup: true,
                file_path: path.clone(),
            };
            let result = comments::delete_comment(path, sheet, cell, &params)?;
            Ok(serde_json::to_value(result).map_err(|e| AppError::Serialize(e.to_string()))?)
        }
    }
}

// ── Named Range ──

fn run_named_range(args: &NamedRangeArgs) -> Result<serde_json::Value> {
    match &args.command {
        NamedRangeSub::List { path } => {
            let ranges = named_ranges::list_named_ranges(path)?;
            Ok(serde_json::to_value(ranges).map_err(|e| AppError::Serialize(e.to_string()))?)
        }
        NamedRangeSub::Get { path, name } => {
            let value = named_ranges::get_named_range_value(path, name)?;
            Ok(serde_json::to_value(value).map_err(|e| AppError::Serialize(e.to_string()))?)
        }
        NamedRangeSub::Create {
            path,
            name,
            range,
            sheet,
            dry_run,
        } => {
            let params = SecurityParams {
                dry_run: *dry_run,
                create_backup: true,
                file_path: path.clone(),
            };
            let result =
                named_ranges::create_named_range(path, name, range, sheet.as_deref(), &params)?;
            Ok(serde_json::to_value(result).map_err(|e| AppError::Serialize(e.to_string()))?)
        }
        NamedRangeSub::Delete {
            path,
            name,
            dry_run,
        } => {
            let params = SecurityParams {
                dry_run: *dry_run,
                create_backup: true,
                file_path: path.clone(),
            };
            let result = named_ranges::delete_named_range(path, name, &params)?;
            Ok(serde_json::to_value(result).map_err(|e| AppError::Serialize(e.to_string()))?)
        }
    }
}

// ── Search ──

fn run_search(args: &SearchArgs) -> Result<serde_json::Value> {
    match &args.command {
        SearchSub::Workbook {
            path,
            pattern,
            match_type,
            search_type,
            case_sensitive,
            sheets,
        } => {
            let query =
                build_search_query(pattern, match_type, search_type, *case_sensitive, sheets)?;
            let results = search::search_workbook(path, &query)?;
            Ok(serde_json::to_value(results).map_err(|e| AppError::Serialize(e.to_string()))?)
        }
        SearchSub::Sheet {
            path,
            sheet,
            pattern,
            match_type,
            search_type,
            case_sensitive,
        } => {
            let query =
                build_search_query(pattern, match_type, search_type, *case_sensitive, &None)?;
            let results = search::search_sheet(path, sheet, &query)?;
            Ok(serde_json::to_value(results).map_err(|e| AppError::Serialize(e.to_string()))?)
        }
    }
}

fn build_search_query(
    pattern: &str,
    match_type: &str,
    search_type: &str,
    case_sensitive: bool,
    sheets: &Option<Vec<String>>,
) -> Result<search::SearchQuery> {
    let st = match search_type {
        "value" => search::SearchType::Value,
        "formula" => search::SearchType::Formula,
        _ => search::SearchType::Both,
    };
    let mt = match match_type {
        "exact" => search::MatchType::Exact,
        "regex" => search::MatchType::Regex,
        _ => search::MatchType::Contains,
    };
    Ok(search::SearchQuery {
        pattern: pattern.to_string(),
        search_type: st,
        match_type: mt,
        case_sensitive,
        sheets: sheets.clone(),
    })
}

// ── Conditional Format ──

fn run_conditional_format(args: &ConditionalFormatArgs) -> Result<serde_json::Value> {
    match &args.command {
        ConditionalFormatSub::Add {
            path,
            sheet,
            range,
            rule_type,
            condition,
            style,
            dry_run,
        } => {
            let params = SecurityParams {
                dry_run: *dry_run,
                create_backup: true,
                file_path: path.clone(),
            };

            let rt = match rule_type.to_lowercase().as_str() {
                "cell_value" | "cellvalue" => conditional_format::ConditionalFormatType::CellValue,
                "formula" => conditional_format::ConditionalFormatType::Formula,
                "above_average" | "aboveaverage" => {
                    conditional_format::ConditionalFormatType::AboveAverage
                }
                "top10" | "top_10" => conditional_format::ConditionalFormatType::Top10,
                "duplicate" => conditional_format::ConditionalFormatType::Duplicate,
                "text_contains" | "textcontains" => {
                    conditional_format::ConditionalFormatType::TextContains
                }
                "date_occurring" | "dateoccurring" => {
                    conditional_format::ConditionalFormatType::DateOccurring
                }
                _ => {
                    return Err(AppError::InvalidInput(format!(
                        "Unknown conditional format rule type: {}",
                        rule_type
                    )));
                }
            };

            let parsed_style: Option<Style> = if let Some(s) = style {
                Some(
                    serde_json::from_str(s)
                        .map_err(|e| AppError::Serialize(format!("Invalid style JSON: {}", e)))?,
                )
            } else {
                None
            };

            let rule = conditional_format::ConditionalFormatRule {
                rule_type: rt,
                condition: condition.clone(),
                format: parsed_style,
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
