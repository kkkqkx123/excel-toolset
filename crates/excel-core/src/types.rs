use std::io;

use chrono::{DateTime, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

#[derive(Debug, Error)]
pub enum AppError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("Calamine error: {0}")]
    Calamine(#[from] calamine::XlsxError),

    #[error("Xlsx error: {0}")]
    Xlsx(#[from] rust_xlsxwriter::XlsxError),

    #[error("Sheet '{0}' not found")]
    SheetNotFound(String),

    #[error("Sheet '{0}' already exists")]
    SheetAlreadyExists(String),

    #[error("Cell ({0},{1}) not found")]
    CellNotFound(u32, u16),

    #[error("Invalid cell reference: {0}")]
    InvalidCellRef(String),

    #[error("Invalid range: {0}")]
    InvalidRange(String),

    #[error("Unknown filter operator: {0}")]
    InvalidFilterOp(String),

    #[error("Unknown chart type: {0}")]
    InvalidChartType(String),

    #[error("{0}")]
    VbaNotSupported(String),

    #[error("{0}")]
    FeatureNotEnabled(String),

    #[error("DuckDB error: {0}")]
    DuckDb(String),

    #[error("Serialization error: {0}")]
    Serialize(String),

    #[error("{0}")]
    Custom(String),
}

impl AppError {
    /// Semantic error code for front-end programmatic handling.
    pub fn error_code(&self) -> &'static str {
        match self {
            AppError::Io(_) => "IO_ERROR",
            AppError::Calamine(_) => "CALAMINE_ERROR",
            AppError::Xlsx(_) => "XLSX_ERROR",
            AppError::SheetNotFound(_) => "SHEET_NOT_FOUND",
            AppError::SheetAlreadyExists(_) => "SHEET_ALREADY_EXISTS",
            AppError::CellNotFound(..) => "CELL_NOT_FOUND",
            AppError::InvalidCellRef(_) => "INVALID_CELL_REF",
            AppError::InvalidRange(_) => "INVALID_RANGE",
            AppError::InvalidFilterOp(_) => "INVALID_FILTER_OP",
            AppError::InvalidChartType(_) => "INVALID_CHART_TYPE",
            AppError::VbaNotSupported(_) => "VBA_NOT_SUPPORTED",
            AppError::FeatureNotEnabled(_) => "FEATURE_NOT_ENABLED",
            AppError::DuckDb(_) => "DUCKDB_ERROR",
            AppError::Serialize(_) => "SERIALIZE_ERROR",
            AppError::Custom(_) => "CUSTOM_ERROR",
        }
    }
}

pub type Result<T> = std::result::Result<T, AppError>;

// ---------------------------------------------------------------------------
// API response
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T: Serialize> {
    pub success: bool,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diff: Option<FileDiff>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backup_info: Option<BackupInfo>,
}

impl<T: Serialize> ApiResponse<T> {
    pub fn ok(data: Option<T>) -> Self {
        ApiResponse {
            success: true,
            message: String::new(),
            error_code: None,
            file_hash: None,
            data,
            diff: None,
            backup_info: None,
        }
    }

    pub fn err(e: AppError) -> Self {
        let code = e.error_code();
        ApiResponse {
            success: false,
            message: e.to_string(),
            error_code: Some(code),
            file_hash: None,
            data: None,
            diff: None,
            backup_info: None,
        }
    }
}

// ---------------------------------------------------------------------------
// File / Backup
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    pub path: String,
    pub hash: String,
    pub size: u64,
    pub sheets: Vec<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupInfo {
    pub backup_path: String,
    pub timestamp: DateTime<Utc>,
    pub operation: String,
    pub file_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityParams {
    pub dry_run: bool,
    pub create_backup: bool,
    pub file_path: String,
}

impl Default for SecurityParams {
    fn default() -> Self {
        Self {
            dry_run: false,
            create_backup: true,
            file_path: String::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Cell reference
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CellRef {
    Cell { sheet: String, row: u32, col: u32 },
    Range { sheet: String, range: String },
}

// ---------------------------------------------------------------------------
// Cell data (read result)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CellDataType {
    String,
    Float,
    Int,
    Bool,
    DateTime,
    Error,
    Empty,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CellData {
    pub value: Option<String>,
    pub data_type: CellDataType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub formula: Option<String>,
}

// ---------------------------------------------------------------------------
// Cell value (write input)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CellValue {
    String(String),
    Number(f64),
    Bool(bool),
    DateTime(NaiveDateTime),
    Empty,
}

// ---------------------------------------------------------------------------
// Sheet data (full sheet in memory)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SheetData {
    pub name: String,
    pub rows: Vec<Vec<CellData>>,
}

// ---------------------------------------------------------------------------
// Write results
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WriteResult {
    pub success: bool,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backup_info: Option<BackupInfo>,
    pub old_hash: String,
    pub new_hash: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diff: Option<Vec<CellDiff>>,
}

// ---------------------------------------------------------------------------
// Style / Format (simplified)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Style {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_size: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bold: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub italic: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub border: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub horizontal_align: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vertical_align: Option<String>,
}

// ---------------------------------------------------------------------------
// Chart configuration
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChartType {
    Column,
    Line,
    Pie,
    Bar,
    Area,
    Scatter,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChartConfig {
    pub chart_type: ChartType,
    pub title: Option<String>,
    pub categories_range: String,
    pub values_range: String,
    pub sheet: String,
    pub row: u32,
    pub col: u16,
}

// ---------------------------------------------------------------------------
// Filter / Sort
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterCondition {
    pub column: u16,
    pub operator: FilterOp,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FilterOp {
    Eq,
    Ne,
    Gt,
    Lt,
    Ge,
    Le,
    Contains,
    StartsWith,
    EndsWith,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SortColumn {
    pub column: u16,
    pub descending: bool,
}

// ---------------------------------------------------------------------------
// Diff types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DiffType {
    Add,
    Delete,
    Modify,
    Passive,
    NoChange,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OperationMode {
    Live,
    DryRun,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CellDiff {
    pub row: u32,
    pub col: u16,
    pub cell_ref: String,
    pub diff_type: DiffType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub old_value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub old_formula: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_formula: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SheetDiff {
    pub sheet_name: String,
    pub row_count_diff: i32,
    pub col_count_diff: i32,
    pub cell_diffs: Vec<CellDiff>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffSummary {
    pub adds: usize,
    pub deletes: usize,
    pub modifies: usize,
    pub passives: usize,
    pub total_changes: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDiff {
    pub file_hash_match: bool,
    pub sheet_diffs: Vec<SheetDiff>,
    pub summary: DiffSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RangeDiff {
    pub range: String,
    pub cell_diffs: Vec<CellDiff>,
}

// ---------------------------------------------------------------------------
// Batch operations
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum BatchOperation {
    WriteCell {
        sheet: String,
        row: u32,
        col: u16,
        value: CellValue,
    },
    WriteRange {
        sheet: String,
        range: String,
        data: Vec<Vec<CellValue>>,
    },
    WriteRangeFromCsv {
        sheet: String,
        range: String,
        csv_path: String,
    },
    ClearRange {
        sheet: String,
        range: String,
    },
    SetFormula {
        sheet: String,
        cell: String,
        formula: String,
    },
    InsertRows {
        sheet: String,
        at_row: u32,
        data: Vec<Vec<CellValue>>,
    },
    DeleteRows {
        sheet: String,
        start_row: u32,
        end_row: u32,
    },
    AppendRows {
        sheet: String,
        data: Vec<Vec<CellValue>>,
    },
    AddSheet {
        name: String,
    },
    DeleteSheet {
        name: String,
    },
    RenameSheet {
        old_name: String,
        new_name: String,
    },
    SortSheet {
        sheet: String,
        columns: Vec<SortColumn>,
    },
    DedupSheet {
        sheet: String,
        columns: Vec<u16>,
    },
    MergeCells {
        sheet: String,
        range: String,
        value: Option<String>,
    },
    SetFormat {
        sheet: String,
        range: String,
        style: Style,
    },
    AddChart {
        config: ChartConfig,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchWriteResult {
    pub success: bool,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backup_info: Option<BackupInfo>,
    pub old_hash: String,
    pub new_hash: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diff: Option<FileDiff>,
    pub operations_count: usize,
    pub succeeded_count: usize,
}

// ---------------------------------------------------------------------------
// RowDiff (kept for compatibility with existing types)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RowDiff {
    pub row_index: u32,
    pub diff_type: DiffType,
    pub cell_diffs: Vec<CellDiff>,
}
