use serde::{Deserialize, Serialize};

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RowDiff {
    pub row_index: u32,
    pub diff_type: DiffType,
    pub cell_diffs: Vec<CellDiff>,
}