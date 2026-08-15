// runners/mod.rs：命令调度中枢。各子命令实现见同级子文件。
use excel_core::types::*;
use super::args::*;

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
mod sparkline;
mod file;
mod sheet;
mod cell;
mod range;
mod data;
mod formula;
mod format;
mod chart;
mod vba;
mod batch;
mod diff;
mod rollback;
mod comments;
mod named_range;
mod search;
mod conditional_format;
mod table;
mod data_validation;
mod pivot_table;
mod slicer;
mod overview;
mod history;
mod freeze_pane;
mod auto_filter;
mod protection;
mod page_setup;
mod image;

use self::{
    sparkline::run_sparkline,
    file::run_file,
    sheet::run_sheet,
    cell::run_cell,
    range::run_range,
    data::run_data,
    formula::run_formula,
    format::run_format,
    chart::run_chart,
    vba::run_vba,
    batch::run_batch,
    diff::run_diff,
    rollback::run_rollback,
    comments::run_comments,
    named_range::run_named_range,
    search::run_search,
    conditional_format::run_conditional_format,
    table::run_table,
    data_validation::run_data_validation,
    pivot_table::run_pivot_table,
    slicer::run_slicer,
    overview::run_overview,
    history::run_history,
    freeze_pane::run_freeze_pane,
    auto_filter::run_auto_filter,
    protection::run_protection,
    page_setup::run_page_setup,
    image::run_image,
};
