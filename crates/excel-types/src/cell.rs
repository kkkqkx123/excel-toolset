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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CellValue {
    String(String),
    Number(f64),
    Bool(bool),
    DateTime(NaiveDateTime),
    Empty,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SheetData {
    pub name: String,
    pub rows: Vec<Vec<CellData>>,
}
