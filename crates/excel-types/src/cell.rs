use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CellRef {
    Cell { sheet: String, row: u32, col: u32 },
    Range { sheet: String, range: String },
}

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum CellValue {
    String(String),
    Number(f64),
    Bool(bool),
    DateTime(NaiveDateTime),
    /// Excel error value, e.g. "#DIV/0!", "#N/A", "#REF!"
    Error(String),
    Empty,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SheetData {
    pub name: String,
    pub rows: Vec<Vec<CellData>>,
    /// Absolute 0-based row index of `rows[0]` within the worksheet.
    ///
    /// calamine's used-range may start below row 1 when the sheet has leading
    /// blank rows. Consumers that report cell coordinates (diff) must add this
    /// offset, otherwise a change in `E3` gets reported as `C1`.
    #[serde(default)]
    pub start_row: u32,
    /// Absolute 0-based column index of `rows[_][0]` within the worksheet.
    #[serde(default)]
    pub start_col: u32,
}

// ===== ReadRange advanced mode types =====

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadRangeOptions {
    #[serde(default)]
    pub mode: OutputMode,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub truncate: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub include_context: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub context_size: Option<usize>,
}

impl Default for ReadRangeOptions {
    fn default() -> Self {
        Self {
            mode: OutputMode::Detailed,
            truncate: None,
            include_context: None,
            context_size: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum OutputMode {
    #[serde(rename = "compact")]
    Compact,
    #[serde(rename = "csv")]
    Csv,
    #[serde(rename = "detailed")]
    #[default]
    Detailed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadRangeResult {
    pub mode: OutputMode,
    pub data: ReadRangeData,
    pub total_rows: usize,
    pub total_cols: usize,
    #[serde(skip_serializing_if = "std::ops::Not::not", default)]
    pub truncated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ReadRangeData {
    Detailed(Vec<Vec<CellData>>),
    Compact(Vec<String>),
    Csv(String),
}
