use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "excel", version = "0.1.0", about = "Excel Tool Gateway")]
pub struct Cli {
    #[arg(long, short)]
    pub pretty: bool,

    #[arg(long, default_value = "json", value_parser = ["json", "text"])]
    pub format: String,

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
    Batch(BatchArgs),
    Rollback(RollbackArgs),
    Comments(CommentsArgs),
    NamedRange(NamedRangeArgs),
    Search(SearchArgs),
    ConditionalFormat(ConditionalFormatArgs),
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
    WriteCsv {
        path: String,
        sheet: String,
        range: String,
        csv: String,
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
    Read {
        path: String,
        sheet: String,
        cell: String,
    },
    CalcMode {
        path: String,
        #[arg(long, default_value = "auto")]
        mode: String,
        #[arg(long)]
        dry_run: bool,
    },
    Trace {
        path: String,
        sheet: String,
        cell: String,
    },
    Explain {
        path: String,
        sheet: String,
        cell: String,
        #[arg(long, default_value = "en")]
        language: String,
    },
    ExplainLogic {
        path: String,
        sheet: String,
        cell: String,
        #[arg(long, default_value = "en")]
        language: String,
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
    Has {
        path: String,
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
    /// Special command for git diff driver integration.
    /// Automatically reads file paths from environment variables (GIT_DIFF_PATH_OLD, GIT_DIFF_PATH_NEW)
    /// or from command line arguments.
    GitDriver,
    InstallGitDriver {},
    UninstallGitDriver {},
}

#[derive(clap::Args)]
pub struct BatchArgs {
    #[command(subcommand)]
    pub command: BatchSub,
}

#[derive(Subcommand)]
pub enum BatchSub {
    Modify {
        path: String,
        #[arg(long)]
        operations: String,
        #[arg(long)]
        dry_run: bool,
    },
}

#[derive(clap::Args)]
pub struct RollbackArgs {
    pub path: String,
    pub backup_path: String,
}

// ── Comments ──

#[derive(clap::Args)]
pub struct CommentsArgs {
    #[command(subcommand)]
    pub command: CommentsSub,
}

#[derive(Subcommand)]
pub enum CommentsSub {
    Get {
        path: String,
        sheet: String,
        cell: String,
    },
    Add {
        path: String,
        sheet: String,
        cell: String,
        text: String,
        #[arg(long)]
        dry_run: bool,
    },
    Update {
        path: String,
        sheet: String,
        cell: String,
        text: String,
        #[arg(long)]
        dry_run: bool,
    },
    Delete {
        path: String,
        sheet: String,
        cell: String,
        #[arg(long)]
        dry_run: bool,
    },
}

// ── Named Range ──

#[derive(clap::Args)]
pub struct NamedRangeArgs {
    #[command(subcommand)]
    pub command: NamedRangeSub,
}

#[derive(Subcommand)]
pub enum NamedRangeSub {
    List {
        path: String,
    },
    Get {
        path: String,
        name: String,
    },
    Create {
        path: String,
        name: String,
        range: String,
        #[arg(long)]
        sheet: Option<String>,
        #[arg(long)]
        dry_run: bool,
    },
    Delete {
        path: String,
        name: String,
        #[arg(long)]
        dry_run: bool,
    },
}

// ── Search ──

#[derive(clap::Args)]
pub struct SearchArgs {
    #[command(subcommand)]
    pub command: SearchSub,
}

#[derive(Subcommand)]
pub enum SearchSub {
    Workbook {
        path: String,
        pattern: String,
        #[arg(long, default_value = "contains")]
        match_type: String,
        #[arg(long, default_value = "both")]
        search_type: String,
        #[arg(long)]
        case_sensitive: bool,
        #[arg(long)]
        sheets: Option<Vec<String>>,
    },
    Sheet {
        path: String,
        sheet: String,
        pattern: String,
        #[arg(long, default_value = "contains")]
        match_type: String,
        #[arg(long, default_value = "both")]
        search_type: String,
        #[arg(long)]
        case_sensitive: bool,
    },
}

// ── Conditional Format ──

#[derive(clap::Args)]
pub struct ConditionalFormatArgs {
    #[command(subcommand)]
    pub command: ConditionalFormatSub,
}

#[derive(Subcommand)]
pub enum ConditionalFormatSub {
    Add {
        path: String,
        sheet: String,
        range: String,
        rule_type: String,
        condition: String,
        #[arg(long)]
        style: Option<String>,
        #[arg(long)]
        dry_run: bool,
    },
    Remove {
        path: String,
        sheet: String,
        range: String,
        #[arg(long)]
        dry_run: bool,
    },
}
