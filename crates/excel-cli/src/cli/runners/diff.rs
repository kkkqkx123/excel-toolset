use crate::cli::args::*;
use excel_core::types::*;
use excel_diff::diff_files;
use excel_diff::diff_range;
use excel_diff::diff_sheets;
use excel_diff::get_git_diff_file_paths;
use excel_diff::git_driver;
use excel_diff::semantic::{self, Verbosity};
use excel_diff::summarize;

pub(crate) fn run_diff(args: &DiffArgs, format: &str) -> Result<serde_json::Value> {
    match &args.command {
        DiffSub::File {
            old_path,
            new_path,
            sheet,
            semantic: use_semantic,
        } => {
            if *use_semantic {
                let sd = excel_diff::diff_with_semantic(old_path, new_path)?;
                return serde_json::to_value(sd).map_err(|e| AppError::Serialize(e.to_string()));
            }
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
            semantic: use_semantic,
        } => {
            if *use_semantic {
                let sd = excel_diff::diff_range(old_path, new_path, sheet, range)?;
                let sheet_diff = SheetDiff {
                    sheet_name: sheet.clone(),
                    row_count_diff: 0,
                    col_count_diff: 0,
                    cell_diffs: sd.cell_diffs.clone(),
                };
                let summary = summarize::summarize(std::slice::from_ref(&sheet_diff));
                let fd = FileDiff {
                    file_hash_match: false,
                    sheet_diffs: vec![sheet_diff],
                    summary,
                };
                let report = semantic::to_semantic_report(&fd, None);
                let mut entries = Vec::new();
                for (idx, op) in report.operations.iter().enumerate() {
                    let sentence = report
                        .detail_sentences
                        .get(idx)
                        .cloned()
                        .unwrap_or_default();
                    let (cell, change_type) = match op {
                        semantic::grouper::LogicalOperation::CellModified {
                            sheet,
                            cell_ref,
                            ..
                        } => (format!("{}!{}", sheet, cell_ref), "modified".to_string()),
                        semantic::grouper::LogicalOperation::CellPassive {
                            sheet,
                            cell_ref,
                            ..
                        } => (format!("{}!{}", sheet, cell_ref), "passive".to_string()),
                        semantic::grouper::LogicalOperation::RowAdded { sheet, row, .. } => {
                            (format!("{}!row-{}", sheet, row + 1), "added".to_string())
                        }
                        semantic::grouper::LogicalOperation::RowDeleted { sheet, row, .. } => {
                            (format!("{}!row-{}", sheet, row + 1), "deleted".to_string())
                        }
                        _ => (String::new(), String::new()),
                    };
                    if !cell.is_empty() {
                        entries.push(SemanticDiffEntry {
                            cell,
                            change_type,
                            description: sentence,
                            impact: None,
                        });
                    }
                }
                let semantic_diff = SemanticDiff {
                    summary: report.summary,
                    entries,
                    statistics: fd.summary,
                };
                return serde_json::to_value(semantic_diff)
                    .map_err(|e| AppError::Serialize(e.to_string()));
            }
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
        DiffSub::Semantic { old_path, new_path } => {
            let sd = excel_diff::diff_with_semantic(old_path, new_path)?;
            Ok(serde_json::to_value(sd).map_err(|e| AppError::Serialize(e.to_string()))?)
        }
        DiffSub::FormulaDeps {
            old_path,
            new_path,
            sheet,
        } => {
            let deps = excel_diff::diff_formula_dependencies(old_path, new_path, sheet)?;
            Ok(serde_json::to_value(deps).map_err(|e| AppError::Serialize(e.to_string()))?)
        }
        DiffSub::GitDriver { args: _ } => {
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
        DiffSub::InstallGitDriver { global, patterns } => {
            git_driver::install_git_driver(*global, patterns)?;
            let scope = if *global {
                "global"
            } else {
                "current repository"
            };
            Ok(serde_json::json!({
                "success": true,
                "message": format!("Git diff driver installed ({}). Match patterns: {}",
                    scope,
                    if patterns.is_empty() {
                        "*.xlsx, *.xls, *.xlsm, *.xlsb (default)"
                    } else {
                        ""
                    }
                )
            }))
        }
        DiffSub::UninstallGitDriver { global } => {
            git_driver::uninstall_git_driver(*global)?;
            let scope = if *global {
                "global"
            } else {
                "current repository"
            };
            Ok(serde_json::json!({
                "success": true,
                "message": format!("Git diff driver uninstalled from {}", scope)
            }))
        }
    }
}
