use serde::{Deserialize, Serialize};

use crate::cell::CellValue;
use crate::diff::FileDiff;
use crate::filter::SortColumn;
use crate::meta::BackupInfo;
use crate::pivot_table::PivotTableConfig;
use crate::sparkline::SparklineConfig;
use crate::style::{ChartConfig, Style};
use crate::table::TableConfig;
use crate::validation::DataValidationConfig;

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
#[serde(tag = "op", rename_all = "snake_case")]
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
    AddTable {
        config: TableConfig,
    },
    AddDataValidation {
        sheet: String,
        config: DataValidationConfig,
    },
    AddPivotTable {
        config: PivotTableConfig,
    },
    AddSparkline {
        config: SparklineConfig,
    },
    RemoveSparkline {
        sheet: String,
        target_row: u32,
        target_col: u16,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failed_operations: Option<Vec<FailedOperation>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailedOperation {
    pub operation_index: usize,
    pub error: String,
    pub operation_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BatchExecutionStrategy {
    #[serde(rename = "all_or_nothing")]
    AllOrNothing,
    #[serde(rename = "best_effort")]
    BestEffort,
    #[serde(rename = "dry_run")]
    DryRun,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchValidateResult {
    pub valid: bool,
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    pub operation_index: usize,
    pub error_type: ValidationErrorType,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValidationErrorType {
    #[serde(rename = "sheet_not_found")]
    SheetNotFound,
    #[serde(rename = "cell_not_found")]
    CellNotFound,
    #[serde(rename = "invalid_formula")]
    InvalidFormula,
    #[serde(rename = "range_out_of_bounds")]
    RangeOutOfBounds,
}
