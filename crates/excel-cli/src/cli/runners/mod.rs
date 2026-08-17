// runners/mod.rs：命令调度中枢。各子命令实现见同级子文件。
use super::args::*;
use excel_core::types::*;

pub fn execute(cli: &Cli) -> Result<()> {
    let result = run_command(cli);
    match result {
        Ok(json) => {
            if cli.format == "text" {
                if let Some(text) = json.get("raw_text").and_then(|v| v.as_str()) {
                    println!("{}", text);
                    return Ok(());
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
            Ok(())
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
            Err(e)
        }
    }
}

// ── Sparkline ──

pub(crate) fn run_command(cli: &Cli) -> Result<serde_json::Value> {
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
        Commands::Table(args) => run_table(args),
        Commands::DataValidation(args) => run_data_validation(args),
        Commands::PivotTable(args) => run_pivot_table(args),
        Commands::Slicer(args) => run_slicer(args),
        Commands::Sparkline(args) => run_sparkline(args),
        Commands::Overview(args) => run_overview(args),
        Commands::History(args) => run_history(args),
        Commands::FreezePane(args) => run_freeze_pane(args),
        Commands::AutoFilter(args) => run_auto_filter(args),
        Commands::Protection(args) => run_protection(args),
        Commands::PageSetup(args) => run_page_setup(args),
        Commands::Image(args) => run_image(args),
    }
}

// ── 子命令实现（每命令一个文件）──
mod auto_filter;
mod batch;
mod cell;
mod chart;
mod comments;
mod conditional_format;
mod data;
mod data_validation;
mod diff;
mod file;
mod format;
mod formula;
mod freeze_pane;
mod history;
mod image;
mod named_range;
mod overview;
mod page_setup;
mod pivot_table;
mod protection;
mod range;
mod rollback;
mod search;
mod sheet;
mod slicer;
mod sparkline;
mod table;
mod vba;

use self::{
    auto_filter::run_auto_filter, batch::run_batch, cell::run_cell, chart::run_chart,
    comments::run_comments, conditional_format::run_conditional_format, data::run_data,
    data_validation::run_data_validation, diff::run_diff, file::run_file, format::run_format,
    formula::run_formula, freeze_pane::run_freeze_pane, history::run_history, image::run_image,
    named_range::run_named_range, overview::run_overview, page_setup::run_page_setup,
    pivot_table::run_pivot_table, protection::run_protection, range::run_range,
    rollback::run_rollback, search::run_search, sheet::run_sheet, slicer::run_slicer,
    sparkline::run_sparkline, table::run_table, vba::run_vba,
};
