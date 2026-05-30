use clap::{Parser, Subcommand};

use excel_core::excel_data;
use excel_core::excel_read;
use excel_core::excel_write;
use excel_core::security;
use excel_core::types::*;
use excel_core::vba_util;
use excel_diff::diff_files;
use excel_diff::diff_range;
use excel_diff::diff_sheets;
use excel_diff::git_driver;

#[derive(Parser)]
#[command(name = "excel", version = "0.1.0", about = "Excel Tool Gateway")]
pub struct Cli {
    #[arg(long, short)]
    pub pretty: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    File(FileArgs),
    Sheet(SheetArgs),
    Cell(CellArgs),
    Range(RangeArgs),
    Data(DataArgs),
    Formula(FormulaArgs),
    Format(FormatArgs),
    Chart(ChartArgs),
    Vba(VbaArgs),
    Diff(DiffArgs),
    Rollback(RollbackArgs),
}

#[derive(clap::Args)]
pub struct FileArgs {
    #[command(subcommand)]
    pub command: FileSub,
}

#[derive(Subcommand)]
pub enum FileSub {
    Create {
        path: String,
        #[arg(long, default_value = "Sheet1")]
        sheet: String,
    },
    Info {
        path: String,
    },
    Backup {
        path: String,
        #[arg(long)]
        output: Option<String>,
    },
}

#[derive(clap::Args)]
pub struct SheetArgs {
    #[command(subcommand)]
    pub command: SheetSub,
}

#[derive(Subcommand)]
pub enum SheetSub {
    List {
        path: String,
    },
    Add {
        path: String,
        name: String,
    },
    Delete {
        path: String,
        name: String,
    },
    Rename {
        path: String,
        old: String,
        new: String,
    },
}

#[derive(clap::Args)]
pub struct CellArgs {
    #[command(subcommand)]
    pub command: CellSub,
}

#[derive(Subcommand)]
pub enum CellSub {
    Read {
        path: String,
        sheet: String,
        cell: String,
    },
    Write {
        path: String,
        sheet: String,
        cell: String,
        value: String,
        #[arg(long)]
        dry_run: bool,
    },
}

#[derive(clap::Args)]
pub struct RangeArgs {
    #[command(subcommand)]
    pub command: RangeSub,
}

#[derive(Subcommand)]
pub enum RangeSub {
    Read {
        path: String,
        sheet: String,
        range: String,
    },
    Write {
        path: String,
        sheet: String,
        range: String,
        data: String,
        #[arg(long)]
        dry_run: bool,
    },
    Clear {
        path: String,
        sheet: String,
        range: String,
        #[arg(long)]
        dry_run: bool,
    },
}

#[derive(clap::Args)]
pub struct DataArgs {
    #[command(subcommand)]
    pub command: DataSub,
}

#[derive(Subcommand)]
pub enum DataSub {
    AppendRow {
        path: String,
        sheet: String,
        values: Vec<String>,
        #[arg(long)]
        dry_run: bool,
    },
    InsertRow {
        path: String,
        sheet: String,
        row: u32,
        values: Vec<String>,
        #[arg(long)]
        dry_run: bool,
    },
    DeleteRow {
        path: String,
        sheet: String,
        row: u32,
        #[arg(long)]
        dry_run: bool,
    },
    Filter {
        path: String,
        sheet: String,
        column: u16,
        op: String,
        value: String,
    },
    Sort {
        path: String,
        sheet: String,
        column: u16,
        #[arg(long)]
        desc: bool,
        #[arg(long)]
        dry_run: bool,
    },
    Dedup {
        path: String,
        sheet: String,
        #[arg(long)]
        column: Option<u16>,
        #[arg(long)]
        dry_run: bool,
    },
    Sql {
        path: String,
        sheet: String,
        query: String,
    },
}

#[derive(clap::Args)]
pub struct FormulaArgs {
    #[command(subcommand)]
    pub command: FormulaSub,
}

#[derive(Subcommand)]
pub enum FormulaSub {
    Set {
        path: String,
        sheet: String,
        cell: String,
        formula: String,
        #[arg(long)]
        dry_run: bool,
    },
    Refresh {
        path: String,
        sheet: String,
        #[arg(long)]
        dry_run: bool,
    },
}

#[derive(clap::Args)]
pub struct FormatArgs {
    #[command(subcommand)]
    pub command: FormatSub,
}

#[derive(Subcommand)]
pub enum FormatSub {
    Set {
        path: String,
        sheet: String,
        range: String,
        style: String,
        #[arg(long)]
        dry_run: bool,
    },
    Merge {
        path: String,
        sheet: String,
        range: String,
        #[arg(long)]
        dry_run: bool,
    },
}

#[derive(clap::Args)]
pub struct ChartArgs {
    #[command(subcommand)]
    pub command: ChartSub,
}

#[derive(Subcommand)]
pub enum ChartSub {
    Create {
        path: String,
        sheet: String,
        range: String,
        chart_type: String,
        title: Option<String>,
        #[arg(long)]
        dry_run: bool,
    },
}

#[derive(clap::Args)]
pub struct VbaArgs {
    #[command(subcommand)]
    pub command: VbaSub,
}

#[derive(Subcommand)]
pub enum VbaSub {
    Export {
        path: String,
        output: String,
    },
    Import {
        path: String,
        vba_file: String,
        #[arg(long)]
        dry_run: bool,
    },
}

#[derive(clap::Args)]
pub struct DiffArgs {
    #[command(subcommand)]
    pub command: DiffSub,
}

#[derive(Subcommand)]
pub enum DiffSub {
    File {
        old_path: String,
        new_path: String,
        #[arg(long)]
        sheet: Option<String>,
    },
    Range {
        old_path: String,
        new_path: String,
        sheet: String,
        range: String,
    },
    InstallGitDriver {},
    UninstallGitDriver {},
}

#[derive(clap::Args)]
pub struct RollbackArgs {
    pub path: String,
    pub backup_path: String,
}

pub fn execute(cli: &Cli) {
    let result = run_command(cli);
    match result {
        Ok(json) => {
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
        Commands::Diff(args) => run_diff(args),
        Commands::Rollback(args) => run_rollback(args),
    }
}

fn run_file(args: &FileArgs) -> Result<serde_json::Value> {
    match &args.command {
        FileSub::Create { path, sheet } => {
            let result = excel_write::create_file(path, sheet)?;
            Ok(serde_json::to_value(result).unwrap())
        }
        FileSub::Info { path } => {
            let info = excel_read::read_file_info(path)?;
            Ok(serde_json::to_value(info).unwrap())
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
            Ok(serde_json::to_value(result).unwrap())
        }
        SheetSub::Delete { path, name } => {
            let result = excel_write::delete_sheet(path, &params, name)?;
            Ok(serde_json::to_value(result).unwrap())
        }
        SheetSub::Rename { path, old, new } => {
            let result = excel_write::rename_sheet(path, &params, old, new)?;
            Ok(serde_json::to_value(result).unwrap())
        }
    }
}

fn run_cell(args: &CellArgs) -> Result<serde_json::Value> {
    match &args.command {
        CellSub::Read { path, sheet, cell } => {
            let (row, col) = excel_core::cell_ref::parse_cell_ref(cell)?;
            let data = excel_read::read_cell(path, sheet, row, col)?;
            Ok(serde_json::to_value(data).unwrap())
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
            let cell_value = parse_cell_value(value);
            let result = excel_write::write_cell(path, &params, sheet, row, col, &cell_value)?;
            Ok(serde_json::to_value(result).unwrap())
        }
    }
}

fn run_range(args: &RangeArgs) -> Result<serde_json::Value> {
    match &args.command {
        RangeSub::Read { path, sheet, range } => {
            let data = excel_read::read_range(path, sheet, range)?;
            Ok(serde_json::to_value(data).unwrap())
        }
        RangeSub::Write {
            path,
            sheet,
            range,
            data,
            dry_run,
        } => {
            let values: Vec<Vec<CellValue>> = parse_cell_value_grid(data)?;
            let params = SecurityParams {
                dry_run: *dry_run,
                create_backup: true,
                file_path: path.clone(),
            };
            let result = excel_write::write_range(path, &params, sheet, range, &values)?;
            Ok(serde_json::to_value(result).unwrap())
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
            Ok(serde_json::to_value(result).unwrap())
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
            let cell_values: Vec<Vec<CellValue>> =
                vec![values.iter().map(|v| parse_cell_value(v)).collect()];
            let params = SecurityParams {
                dry_run: *dry_run,
                create_backup: true,
                file_path: path.clone(),
            };
            let result = excel_data::append_rows(path, &params, sheet, &cell_values)?;
            Ok(serde_json::to_value(result).unwrap())
        }
        DataSub::InsertRow {
            path,
            sheet,
            row,
            values,
            dry_run,
        } => {
            let cell_values: Vec<Vec<CellValue>> =
                vec![values.iter().map(|v| parse_cell_value(v)).collect()];
            let params = SecurityParams {
                dry_run: *dry_run,
                create_backup: true,
                file_path: path.clone(),
            };
            let result = excel_data::insert_rows(path, &params, sheet, *row, &cell_values)?;
            Ok(serde_json::to_value(result).unwrap())
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
            Ok(serde_json::to_value(result).unwrap())
        }
        DataSub::Filter {
            path,
            sheet,
            column,
            op,
            value,
        } => {
            let filter_op = parse_filter_op(op)?;
            let conditions = vec![FilterCondition {
                column: *column,
                operator: filter_op,
                value: value.clone(),
            }];
            let result = excel_data::filter_rows(path, sheet, &conditions)?;
            Ok(serde_json::to_value(result).unwrap())
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
            let result = excel_data::sort_sheet(path, &params, sheet, &sort_cols)?;
            Ok(serde_json::to_value(result).unwrap())
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
            let result = excel_data::dedup_sheet(path, &params, sheet, &cols)?;
            Ok(serde_json::to_value(result).unwrap())
        }
        DataSub::Sql { path, sheet, query } => {
            let result = excel_data::sql_query(path, sheet, query)?;
            Ok(serde_json::to_value(result).unwrap())
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
            Ok(serde_json::to_value(result).unwrap())
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
            Ok(serde_json::to_value(result).unwrap())
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
                .map_err(|e| AppError::Custom(format!("Invalid style JSON: {}", e)))?;
            let params = SecurityParams {
                dry_run: *dry_run,
                create_backup: true,
                file_path: path.clone(),
            };
            let result = excel_write::set_format(path, &params, sheet, range, &style_val)?;
            Ok(serde_json::to_value(result).unwrap())
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
            Ok(serde_json::to_value(result).unwrap())
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
            let ct = match chart_type.to_lowercase().as_str() {
                "column" => ChartType::Column,
                "line" => ChartType::Line,
                "pie" => ChartType::Pie,
                "bar" => ChartType::Bar,
                "area" => ChartType::Area,
                "scatter" => ChartType::Scatter,
                _ => {
                    return Err(AppError::Custom(format!(
                        "Unknown chart type: {}",
                        chart_type
                    )));
                }
            };
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
            Ok(serde_json::to_value(result).unwrap())
        }
    }
}

fn run_vba(args: &VbaArgs) -> Result<serde_json::Value> {
    match &args.command {
        VbaSub::Export { path, output } => {
            let data = vba_util::export_vba(path)?;
            std::fs::write(output, &data)?;
            Ok(
                serde_json::json!({ "success": true, "message": format!("VBA exported to {}", output) }),
            )
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
            Ok(serde_json::to_value(result).unwrap())
        }
    }
}

fn run_diff(args: &DiffArgs) -> Result<serde_json::Value> {
    match &args.command {
        DiffSub::File {
            old_path,
            new_path,
            sheet,
        } => match sheet {
            Some(s) => {
                let diff = diff_sheets(old_path, new_path, s)?;
                Ok(serde_json::to_value(diff).unwrap())
            }
            None => {
                let diff = diff_files(old_path, new_path)?;
                Ok(serde_json::to_value(diff).unwrap())
            }
        },
        DiffSub::Range {
            old_path,
            new_path,
            sheet,
            range,
        } => {
            let diff = diff_range(old_path, new_path, sheet, range)?;
            Ok(serde_json::to_value(diff).unwrap())
        }
        DiffSub::InstallGitDriver {} => {
            git_driver::install_git_driver().map_err(AppError::Custom)?;
            Ok(serde_json::json!({ "success": true, "message": "Git diff driver installed" }))
        }
        DiffSub::UninstallGitDriver {} => {
            git_driver::uninstall_git_driver().map_err(AppError::Custom)?;
            Ok(serde_json::json!({ "success": true, "message": "Git diff driver uninstalled" }))
        }
    }
}

fn run_rollback(args: &RollbackArgs) -> Result<serde_json::Value> {
    let backup = BackupInfo {
        backup_path: args.backup_path.clone(),
        timestamp: chrono::Utc::now(),
        operation: "rollback".into(),
        file_hash: String::new(),
    };
    security::rollback(&backup, &args.path)?;
    Ok(
        serde_json::json!({ "success": true, "message": format!("Rolled back {} from {}", args.path, args.backup_path) }),
    )
}

fn parse_cell_value(s: &str) -> CellValue {
    if let Ok(n) = s.parse::<f64>() {
        return CellValue::Number(n);
    }
    match s.to_lowercase().as_str() {
        "true" => return CellValue::Bool(true),
        "false" => return CellValue::Bool(false),
        "null" | "none" | "empty" => return CellValue::Empty,
        _ => {}
    }
    CellValue::String(s.to_string())
}

fn parse_cell_value_grid(s: &str) -> Result<Vec<Vec<CellValue>>> {
    let outer: Vec<Vec<serde_json::Value>> = serde_json::from_str(s)
        .map_err(|e| AppError::Custom(format!("Invalid data JSON: {}", e)))?;
    let mut grid = Vec::new();
    for row in outer {
        let mut cells = Vec::new();
        for val in row {
            match val {
                serde_json::Value::Number(n) => {
                    cells.push(CellValue::Number(n.as_f64().unwrap_or(0.0)));
                }
                serde_json::Value::Bool(b) => cells.push(CellValue::Bool(b)),
                serde_json::Value::String(s) => cells.push(CellValue::String(s)),
                serde_json::Value::Null => cells.push(CellValue::Empty),
                _ => cells.push(CellValue::String(val.to_string())),
            }
        }
        grid.push(cells);
    }
    Ok(grid)
}

fn parse_filter_op(s: &str) -> Result<FilterOp> {
    match s.to_lowercase().as_str() {
        "eq" | "=" | "==" => Ok(FilterOp::Eq),
        "ne" | "!=" => Ok(FilterOp::Ne),
        "gt" | ">" => Ok(FilterOp::Gt),
        "lt" | "<" => Ok(FilterOp::Lt),
        "ge" | ">=" => Ok(FilterOp::Ge),
        "le" | "<=" => Ok(FilterOp::Le),
        "contains" => Ok(FilterOp::Contains),
        "startswith" | "starts_with" => Ok(FilterOp::StartsWith),
        "endswith" | "ends_with" => Ok(FilterOp::EndsWith),
        _ => Err(AppError::Custom(format!("Unknown filter operator: {}", s))),
    }
}
