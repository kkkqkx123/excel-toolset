use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SparklineType {
    Line,
    Column,
    WinLose,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SparklineConfig {
    pub sparkline_type: SparklineType,
    pub sheet: String,
    /// Source data range as sheet-qualified string, e.g., "'Sheet1'!A1:E1".
    pub source_range: String,
    pub target_row: u32,
    pub target_col: u16,
    pub style: Option<u8>,
}
