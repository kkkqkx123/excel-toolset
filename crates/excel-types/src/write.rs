use serde::{Deserialize, Serialize};

use crate::cell::CellValue;
use crate::diff::FileDiff;
use crate::filter::SortColumn;
use crate::meta::BackupInfo;
use crate::style::{ChartConfig, Style};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WriteResult {
    pub success: bool,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backup_info: Option<BackupInfo>,
    pub old_hash: String,
    pub new_hash: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diff: Option<FileDiff>,
}

impl WriteResult {
    pub fn dry_run_success() -> Self {
        Self {
            success: true,
            message: "Dry run completed successfully".to_string(),
            backup_info: None,
            old_hash: String::new(),
            new_hash: String::new(),
            diff: None,
        }
    }
}

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
