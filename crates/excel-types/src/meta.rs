use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

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

// ===== Workbook overview types =====

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkbookOverview {
    pub path: String,
    pub sheets: Vec<SheetOverview>,
    pub named_ranges_count: usize,
    pub total_cells: usize,
    pub formula_cells: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SheetOverview {
    pub name: String,
    pub used_range: String,
    pub row_count: usize,
    pub col_count: usize,
    pub has_formulas: bool,
    pub has_data: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnSummary {
    pub column: String,
    pub inferred_type: String,
    pub non_empty_rows: usize,
    pub first_row_value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkbookHistoryEntry {
    pub timestamp: DateTime<Utc>,
    pub operation_type: String,
    pub target_path: String,
    pub old_hash: String,
    pub new_hash: String,
    pub result: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkbookBlueprint {
    pub path: String,
    pub sheets: Vec<BlueprintSheet>,
    pub data_flows: Vec<DataFlow>,
    pub key_cells: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlueprintSheet {
    pub name: String,
    pub row_count: usize,
    pub col_count: usize,
    pub named_ranges: Vec<String>,
    pub formula_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataFlow {
    pub from: String,
    pub to: String,
    pub via: String,
}
