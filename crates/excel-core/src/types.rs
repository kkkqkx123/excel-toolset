use std::fmt;
use std::io;

use chrono::{DateTime, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub enum AppError {
    Io(io::Error),
    Calamine(String),
    Xlsx(rust_xlsxwriter::XlsxError),
    Custom(String),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::Io(e) => write!(f, "IO error: {}", e),
            AppError::Calamine(e) => write!(f, "Calamine error: {}", e),
            AppError::Xlsx(e) => write!(f, "Xlsx error: {}", e),
            AppError::Custom(e) => write!(f, "{}", e),
        }
    }
}

impl std::error::Error for AppError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            AppError::Io(e) => Some(e),
            AppError::Xlsx(e) => Some(e),
            _ => None,
        }
    }
}

impl From<io::Error> for AppError {
    fn from(e: io::Error) -> Self {
        AppError::Io(e)
    }
}

impl From<rust_xlsxwriter::XlsxError> for AppError {
    fn from(e: rust_xlsxwriter::XlsxError) -> Self {
        AppError::Xlsx(e)
    }
}

impl From<String> for AppError {
    fn from(e: String) -> Self {
        AppError::Custom(e)
    }
}

impl From<&str> for AppError {
    fn from(e: &str) -> Self {
        AppError::Custom(e.to_string())
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
            file_hash: None,
            data,
            diff: None,
            backup_info: None,
        }
    }

    pub fn err(e: AppError) -> Self {
        ApiResponse {
            success: false,
            message: e.to_string(),
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

#[derive(Debug, Clone)]
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
// RowDiff (kept for compatibility with existing types)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RowDiff {
    pub row_index: u32,
    pub diff_type: DiffType,
    pub cell_diffs: Vec<CellDiff>,
}
