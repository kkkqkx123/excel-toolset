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
    Table(TableArgs),
    DataValidation(DataValidationArgs),
    PivotTable(PivotTableArgs),
    Slicer(SlicerArgs),
    Sparkline(SparklineArgs),
    Overview(OverviewArgs),
    History(HistoryArgs),
    FreezePane(FreezePaneArgs),
    AutoFilter(AutoFilterArgs),
    Protection(ProtectionArgs),
    PageSetup(PageSetupArgs),
    Image(ImageArgs),
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
    SetVisibility {
        path: String,
        name: String,
        /// Visibility: visible, hidden, very_hidden
        #[arg(long)]
        visibility: String,
        #[arg(long)]
        dry_run: bool,
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
        #[arg(long, default_value = "detailed")]
        mode: String,
        #[arg(long)]
        truncate: Option<usize>,
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
        #[arg(long)]
        session: bool,
        #[arg(long)]
        cache: bool,
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
        eval: bool,
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
    Fill {
        path: String,
        sheet: String,
        source: String,
        target_range: String,
        #[arg(long)]
        dry_run: bool,
    },
    Eval {
        path: String,
        sheet: String,
        cell: String,
        formula: String,
        #[arg(long)]
        no_eval: bool,
        #[arg(long)]
        dry_run: bool,
    },
    EvalBatch {
        path: String,
        sheet: String,
        formulas: String,
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
        value: Option<String>,
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
        #[arg(long)]
        title: Option<String>,
        #[arg(long)]
        position: Option<String>,
        #[arg(long)]
        dry_run: bool,
        /// Trendline config as JSON, e.g. '{"trend_type":"linear","display_equation":true}'
        #[arg(long)]
        trendline: Option<String>,
        /// Y error bars config as JSON, e.g. '{"error_type":"standard_error","direction":"both"}'
        #[arg(long)]
        y_error_bars: Option<String>,
        /// X error bars config as JSON
        #[arg(long)]
        x_error_bars: Option<String>,
        /// Logarithmic base for Y axis
        #[arg(long)]
        log_base: Option<u16>,
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
        #[arg(long)]
        semantic: bool,
    },
    Range {
        old_path: String,
        new_path: String,
        sheet: String,
        range: String,
        #[arg(long)]
        semantic: bool,
    },
    Semantic {
        old_path: String,
        new_path: String,
    },
    FormulaDeps {
        old_path: String,
        new_path: String,
        sheet: String,
    },
    /// Special command for git diff driver integration.
    /// Automatically reads file paths from environment variables (GIT_DIFF_PATH_OLD, GIT_DIFF_PATH_NEW)
    /// or from command line arguments.
    ///
    /// Git calls the external diff driver with 7 trailing arguments:
    /// <path> <old-file> <old-hex> <old-mode> <new-file> <new-hex> <new-mode>
    /// These must be accepted (via trailing_var_arg) even though they are parsed
    /// manually by get_git_diff_file_paths() rather than by clap.
    #[command(trailing_var_arg = true)]
    GitDriver {
        /// Git-provided arguments (path, old-file, old-hex, old-mode, new-file, new-hex, new-mode).
        /// Parsed manually by get_git_diff_file_paths() using env vars or positional args.
        #[arg(hide = true)]
        args: Vec<String>,
    },
    /// Install the git diff driver for Excel files.
    /// By default configures the current repository only.
    /// Use --global to apply system-wide.
    InstallGitDriver {
        /// Apply system-wide (all repositories)
        #[arg(long)]
        global: bool,
        /// Comma-separated file patterns (default: *.xlsx,*.xls,*.xlsm,*.xlsb)
        #[arg(long, value_delimiter = ',')]
        patterns: Vec<String>,
    },
    /// Uninstall the git diff driver.
    /// Use --global to remove system-wide configuration.
    UninstallGitDriver {
        /// Remove system-wide configuration
        #[arg(long)]
        global: bool,
    },
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
        #[arg(long, default_value = "best-effort")]
        strategy: String,
        #[arg(long)]
        validate_only: bool,
    },
    ValidateRefs {
        path: String,
        sheet: String,
        formula: String,
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
        /// JSON config for DataBar, ColorScale, IconSet types.
        /// Example: '{"fill_color":"#00FF00"}' or '{"icon_type":"three_arrows"}'
        #[arg(long)]
        config: Option<String>,
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

// ── Table ──

#[derive(clap::Args)]
pub struct TableArgs {
    #[command(subcommand)]
    pub command: TableSub,
}

#[derive(Subcommand)]
pub enum TableSub {
    Create {
        path: String,
        #[arg(long)]
        config: String,
        #[arg(long)]
        dry_run: bool,
    },
    Remove {
        path: String,
        name: String,
        #[arg(long)]
        dry_run: bool,
    },
    List {
        path: String,
    },
    Get {
        path: String,
        name: String,
    },
}

// ── Data Validation ──

#[derive(clap::Args)]
pub struct DataValidationArgs {
    #[command(subcommand)]
    pub command: DataValidationSub,
}

#[derive(Subcommand)]
pub enum DataValidationSub {
    Add {
        path: String,
        sheet: String,
        #[arg(long)]
        config: String,
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

// ── Pivot Table ──

#[derive(clap::Args)]
pub struct PivotTableArgs {
    #[command(subcommand)]
    pub command: PivotTableSub,
}

#[derive(Subcommand)]
pub enum PivotTableSub {
    Create {
        path: String,
        #[arg(long)]
        config: String,
        #[arg(long)]
        dry_run: bool,
    },
}

// ── Slicer ──

#[derive(clap::Args)]
pub struct SlicerArgs {
    #[command(subcommand)]
    pub command: SlicerSub,
}

#[derive(Subcommand)]
pub enum SlicerSub {
    Create {
        path: String,
        /// JSON SlicerConfig
        #[arg(long)]
        config: String,
        #[arg(long)]
        dry_run: bool,
    },
}

// ── Sparkline ──

#[derive(clap::Args)]
pub struct SparklineArgs {
    #[command(subcommand)]
    pub command: SparklineSub,
}

#[derive(Subcommand)]
pub enum SparklineSub {
    Add {
        path: String,
        sheet: String,
        /// Source data range, e.g., "'Sheet1'!A1:E1"
        source_range: String,
        /// Sparkline type: line, column, winlose
        #[arg(long, default_value = "line")]
        sparkline_type: String,
        /// Target cell, e.g., "F1"
        target_cell: String,
        #[arg(long)]
        style: Option<u8>,
        #[arg(long)]
        dry_run: bool,
    },
    Remove {
        path: String,
        sheet: String,
        target_cell: String,
        #[arg(long)]
        dry_run: bool,
    },
}

// ── Overview / History ──

#[derive(clap::Args)]
pub struct OverviewArgs {
    pub path: String,
    #[arg(long)]
    pub blueprint: bool,
}

#[derive(clap::Args)]
pub struct HistoryArgs {
    pub path: String,
}

// ── Freeze Pane ──

#[derive(clap::Args)]
pub struct FreezePaneArgs {
    #[command(subcommand)]
    pub command: FreezePaneSub,
}

#[derive(Subcommand)]
pub enum FreezePaneSub {
    Set {
        path: String,
        sheet: String,
        /// Number of rows to freeze from top (0 = no row freeze)
        #[arg(long, default_value = "0")]
        rows: u32,
        /// Number of columns to freeze from left (0 = no column freeze)
        #[arg(long, default_value = "0")]
        cols: u16,
        #[arg(long)]
        dry_run: bool,
    },
    Clear {
        path: String,
        sheet: String,
        #[arg(long)]
        dry_run: bool,
    },
}

// ── AutoFilter ──

#[derive(clap::Args)]
pub struct AutoFilterArgs {
    #[command(subcommand)]
    pub command: AutoFilterSub,
}

#[derive(Subcommand)]
pub enum AutoFilterSub {
    Set {
        path: String,
        sheet: String,
        /// Autofilter range including header row, e.g. "A1:D100"
        range: String,
        #[arg(long)]
        dry_run: bool,
    },
    Remove {
        path: String,
        sheet: String,
        #[arg(long)]
        dry_run: bool,
    },
    Get {
        path: String,
        sheet: String,
    },
}

// ── Protection ──

#[derive(clap::Args)]
pub struct ProtectionArgs {
    #[command(subcommand)]
    pub command: ProtectionSub,
}

#[derive(Subcommand)]
pub enum ProtectionSub {
    Protect {
        path: String,
        sheet: String,
        /// Optional password for protection
        #[arg(long)]
        password: Option<String>,
        /// JSON ProtectionOptions config
        #[arg(long)]
        options: Option<String>,
        #[arg(long)]
        dry_run: bool,
    },
    Unprotect {
        path: String,
        sheet: String,
        #[arg(long)]
        dry_run: bool,
    },
    IsProtected {
        path: String,
        sheet: String,
    },
}

// ── Page Setup ──

#[derive(clap::Args)]
pub struct PageSetupArgs {
    #[command(subcommand)]
    pub command: PageSetupSub,
}

#[derive(Subcommand)]
pub enum PageSetupSub {
    /// Configure page setup (orientation, paper size, margins, etc.)
    Configure {
        path: String,
        sheet: String,
        /// JSON PageSetupConfig (without the 'sheet' field which is taken from the positional arg)
        #[arg(long)]
        config: String,
        #[arg(long)]
        dry_run: bool,
    },
    /// Set page breaks
    PageBreaks {
        path: String,
        /// JSON PageBreakConfig (includes sheet field)
        #[arg(long)]
        config: String,
        #[arg(long)]
        dry_run: bool,
    },
    /// Clear all page breaks
    ClearBreaks {
        path: String,
        sheet: String,
        #[arg(long)]
        dry_run: bool,
    },
}

// ── Image ──

#[derive(clap::Args)]
pub struct ImageArgs {
    #[command(subcommand)]
    pub command: ImageSub,
}

#[derive(Subcommand)]
pub enum ImageSub {
    /// Insert an image into a worksheet
    Insert {
        path: String,
        /// JSON ImageConfig
        #[arg(long)]
        config: String,
        #[arg(long)]
        dry_run: bool,
    },
    /// Remove an image from a worksheet
    Remove {
        path: String,
        sheet: String,
        /// Anchor cell where the image was placed, e.g. "B2"
        anchor_cell: String,
        #[arg(long)]
        dry_run: bool,
    },
    /// Insert a shape (rectangle, ellipse, line) into a worksheet
    ShapeInsert {
        path: String,
        /// JSON ShapeConfig
        #[arg(long)]
        config: String,
        #[arg(long)]
        dry_run: bool,
    },
}
