use axum::{extract::Path, Json};
use serde::{Deserialize, Serialize};

use crate::cell_ref;
use crate::excel_data;
use crate::excel_diff;
use crate::excel_read;
use crate::excel_write;
use crate::types::*;
use crate::vba_util;

// ---------------------------------------------------------------------------
// File
// ---------------------------------------------------------------------------

pub async fn file_info(Path(path): Path<String>) -> Json<ApiResponse<FileInfo>> {
    match excel_read::read_file_info(&path) {
        Ok(data) => Json(ApiResponse::ok(Some(data))),
        Err(e) => Json(ApiResponse::err(e)),
    }
}

#[derive(Deserialize)]
pub struct CreateFileReq {
    pub path: String,
    #[serde(default = "default_sheet")]
    pub sheet: String,
}
fn default_sheet() -> String { "Sheet1".into() }

pub async fn file_create(Json(req): Json<CreateFileReq>) -> Json<ApiResponse<WriteResult>> {
    match excel_write::create_file(&req.path, &req.sheet) {
        Ok(data) => Json(ApiResponse::ok(Some(data))),
        Err(e) => Json(ApiResponse::err(e)),
    }
}

#[derive(Deserialize)]
pub struct BackupFileReq {
    pub path: String,
    pub output: Option<String>,
}

pub async fn file_backup(Json(req): Json<BackupFileReq>) -> Json<ApiResponse<BackupInfo>> {
    let hash = match crate::security::compute_file_hash(&req.path) {
        Ok(h) => h,
        Err(e) => return Json(ApiResponse::err(AppError::Io(e))),
    };
    match crate::security::create_backup(&req.path, &hash) {
        Ok(backup) => {
            if let Some(ref out) = req.output {
                let _ = std::fs::copy(&backup.backup_path, out);
            }
            Json(ApiResponse::ok(Some(backup)))
        }
        Err(e) => Json(ApiResponse::err(AppError::Io(e))),
    }
}

// ---------------------------------------------------------------------------
// Sheet
// ---------------------------------------------------------------------------

pub async fn sheet_list(Path(path): Path<String>) -> Json<ApiResponse<Vec<String>>> {
    match excel_read::list_sheets(&path) {
        Ok(data) => Json(ApiResponse::ok(Some(data))),
        Err(e) => Json(ApiResponse::err(e)),
    }
}

#[derive(Deserialize)]
pub struct SheetNameReq {
    pub path: String,
    pub name: String,
}

pub async fn sheet_add(Json(req): Json<SheetNameReq>) -> Json<ApiResponse<WriteResult>> {
    let params = SecurityParams { dry_run: false, create_backup: true, file_path: req.path.clone() };
    match excel_write::add_sheet(&req.path, &params, &req.name) {
        Ok(data) => Json(ApiResponse::ok(Some(data))),
        Err(e) => Json(ApiResponse::err(e)),
    }
}

pub async fn sheet_delete(Json(req): Json<SheetNameReq>) -> Json<ApiResponse<WriteResult>> {
    let params = SecurityParams { dry_run: false, create_backup: true, file_path: req.path.clone() };
    match excel_write::delete_sheet(&req.path, &params, &req.name) {
        Ok(data) => Json(ApiResponse::ok(Some(data))),
        Err(e) => Json(ApiResponse::err(e)),
    }
}

#[derive(Deserialize)]
pub struct RenameSheetReq {
    pub path: String,
    pub old: String,
    pub new: String,
}

pub async fn sheet_rename(Json(req): Json<RenameSheetReq>) -> Json<ApiResponse<WriteResult>> {
    let params = SecurityParams { dry_run: false, create_backup: true, file_path: req.path.clone() };
    match excel_write::rename_sheet(&req.path, &params, &req.old, &req.new) {
        Ok(data) => Json(ApiResponse::ok(Some(data))),
        Err(e) => Json(ApiResponse::err(e)),
    }
}

// ---------------------------------------------------------------------------
// Cell
// ---------------------------------------------------------------------------

pub async fn cell_read(Path((path, sheet, cell)): Path<(String, String, String)>) -> Json<ApiResponse<CellData>> {
    let (row, col) = match cell_ref::parse_cell_ref(&cell) {
        Ok(v) => v,
        Err(e) => return Json(ApiResponse::err(e)),
    };
    match excel_read::read_cell(&path, &sheet, row, col) {
        Ok(data) => Json(ApiResponse::ok(Some(data))),
        Err(e) => Json(ApiResponse::err(e)),
    }
}

#[derive(Deserialize)]
pub struct CellWriteReq {
    pub path: String,
    pub sheet: String,
    pub cell: String,
    pub value: String,
    #[serde(default)]
    pub dry_run: bool,
}

pub async fn cell_write(Json(req): Json<CellWriteReq>) -> Json<ApiResponse<WriteResult>> {
    let (row, col) = match cell_ref::parse_cell_ref(&req.cell) {
        Ok(v) => v,
        Err(e) => return Json(ApiResponse::err(e)),
    };
    let params = SecurityParams { dry_run: req.dry_run, create_backup: true, file_path: req.path.clone() };
    match excel_write::write_cell(&req.path, &params, &req.sheet, row, col, &parse_cell_value(&req.value)) {
        Ok(data) => Json(ApiResponse::ok(Some(data))),
        Err(e) => Json(ApiResponse::err(e)),
    }
}

// ---------------------------------------------------------------------------
// Range
// ---------------------------------------------------------------------------

pub async fn range_read(Path((path, sheet, range)): Path<(String, String, String)>) -> Json<ApiResponse<Vec<Vec<CellData>>>> {
    match excel_read::read_range(&path, &sheet, &range) {
        Ok(data) => Json(ApiResponse::ok(Some(data))),
        Err(e) => Json(ApiResponse::err(e)),
    }
}

#[derive(Deserialize)]
pub struct RangeWriteReq {
    pub path: String,
    pub sheet: String,
    pub range: String,
    pub data: Vec<Vec<serde_json::Value>>,
    #[serde(default)]
    pub dry_run: bool,
}

pub async fn range_write(Json(req): Json<RangeWriteReq>) -> Json<ApiResponse<WriteResult>> {
    let values: Vec<Vec<CellValue>> = req.data.iter().map(|row: &Vec<serde_json::Value>| {
        row.iter().map(|v: &serde_json::Value| json_val_to_cell_value(v)).collect::<Vec<CellValue>>()
    }).collect();
    let params = SecurityParams { dry_run: req.dry_run, create_backup: true, file_path: req.path.clone() };
    match excel_write::write_range(&req.path, &params, &req.sheet, &req.range, &values) {
        Ok(data) => Json(ApiResponse::ok(Some(data))),
        Err(e) => Json(ApiResponse::err(e)),
    }
}

#[derive(Deserialize)]
pub struct RangeClearReq {
    pub path: String,
    pub sheet: String,
    pub range: String,
    #[serde(default)]
    pub dry_run: bool,
}

pub async fn range_clear(Json(req): Json<RangeClearReq>) -> Json<ApiResponse<WriteResult>> {
    let params = SecurityParams { dry_run: req.dry_run, create_backup: true, file_path: req.path.clone() };
    match excel_write::clear_range(&req.path, &params, &req.sheet, &req.range) {
        Ok(data) => Json(ApiResponse::ok(Some(data))),
        Err(e) => Json(ApiResponse::err(e)),
    }
}

// ---------------------------------------------------------------------------
// Data
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct RowOpReq {
    pub path: String,
    pub sheet: String,
    pub values: Vec<String>,
    #[serde(default)]
    pub dry_run: bool,
}

pub async fn data_append_row(Json(req): Json<RowOpReq>) -> Json<ApiResponse<WriteResult>> {
    let row: Vec<Vec<CellValue>> = vec![req.values.iter().map(|v| parse_cell_value(v)).collect()];
    let params = SecurityParams { dry_run: req.dry_run, create_backup: true, file_path: req.path.clone() };
    match excel_data::append_rows(&req.path, &params, &req.sheet, &row) {
        Ok(data) => Json(ApiResponse::ok(Some(data))),
        Err(e) => Json(ApiResponse::err(e)),
    }
}

#[derive(Deserialize)]
pub struct InsertRowReq {
    pub path: String,
    pub sheet: String,
    pub row: u32,
    pub values: Vec<String>,
    #[serde(default)]
    pub dry_run: bool,
}

pub async fn data_insert_row(Json(req): Json<InsertRowReq>) -> Json<ApiResponse<WriteResult>> {
    let row: Vec<Vec<CellValue>> = vec![req.values.iter().map(|v| parse_cell_value(v)).collect()];
    let params = SecurityParams { dry_run: req.dry_run, create_backup: true, file_path: req.path.clone() };
    match excel_data::insert_rows(&req.path, &params, &req.sheet, req.row, &row) {
        Ok(data) => Json(ApiResponse::ok(Some(data))),
        Err(e) => Json(ApiResponse::err(e)),
    }
}

#[derive(Deserialize)]
pub struct DeleteRowReq {
    pub path: String,
    pub sheet: String,
    pub row: u32,
    #[serde(default)]
    pub dry_run: bool,
}

pub async fn data_delete_row(Json(req): Json<DeleteRowReq>) -> Json<ApiResponse<WriteResult>> {
    let params = SecurityParams { dry_run: req.dry_run, create_backup: true, file_path: req.path.clone() };
    match excel_data::delete_rows(&req.path, &params, &req.sheet, req.row, req.row) {
        Ok(data) => Json(ApiResponse::ok(Some(data))),
        Err(e) => Json(ApiResponse::err(e)),
    }
}

#[derive(Deserialize)]
pub struct FilterReq {
    pub path: String,
    pub sheet: String,
    pub column: u16,
    pub operator: String,
    pub value: String,
}

pub async fn data_filter(Json(req): Json<FilterReq>) -> Json<ApiResponse<Vec<Vec<CellData>>>> {
    let filter_op = match parse_filter_op(&req.operator) {
        Ok(op) => op,
        Err(e) => return Json(ApiResponse::err(e)),
    };
    let conditions = vec![FilterCondition { column: req.column, operator: filter_op, value: req.value }];
    match excel_data::filter_rows(&req.path, &req.sheet, &conditions) {
        Ok(data) => Json(ApiResponse::ok(Some(data))),
        Err(e) => Json(ApiResponse::err(e)),
    }
}

#[derive(Deserialize)]
pub struct SortReq {
    pub path: String,
    pub sheet: String,
    pub column: u16,
    #[serde(default)]
    pub descending: bool,
    #[serde(default)]
    pub dry_run: bool,
}

pub async fn data_sort(Json(req): Json<SortReq>) -> Json<ApiResponse<WriteResult>> {
    let sort_cols = vec![SortColumn { column: req.column, descending: req.descending }];
    let params = SecurityParams { dry_run: req.dry_run, create_backup: true, file_path: req.path.clone() };
    match excel_data::sort_sheet(&req.path, &params, &req.sheet, &sort_cols) {
        Ok(data) => Json(ApiResponse::ok(Some(data))),
        Err(e) => Json(ApiResponse::err(e)),
    }
}

#[derive(Deserialize)]
pub struct DedupReq {
    pub path: String,
    pub sheet: String,
    pub column: Option<u16>,
    #[serde(default)]
    pub dry_run: bool,
}

pub async fn data_dedup(Json(req): Json<DedupReq>) -> Json<ApiResponse<WriteResult>> {
    let cols = req.column.map(|c| vec![c]).unwrap_or_default();
    let params = SecurityParams { dry_run: req.dry_run, create_backup: true, file_path: req.path.clone() };
    match excel_data::dedup_sheet(&req.path, &params, &req.sheet, &cols) {
        Ok(data) => Json(ApiResponse::ok(Some(data))),
        Err(e) => Json(ApiResponse::err(e)),
    }
}

#[derive(Deserialize)]
pub struct SqlReq {
    pub path: String,
    pub sheet: String,
    pub query: String,
}

pub async fn data_sql(Json(req): Json<SqlReq>) -> Json<ApiResponse<Vec<Vec<CellData>>>> {
    match excel_data::sql_query(&req.path, &req.sheet, &req.query) {
        Ok(data) => Json(ApiResponse::ok(Some(data))),
        Err(e) => Json(ApiResponse::err(e)),
    }
}

// ---------------------------------------------------------------------------
// Formula
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct FormulaSetReq {
    pub path: String,
    pub sheet: String,
    pub cell: String,
    pub formula: String,
    #[serde(default)]
    pub dry_run: bool,
}

pub async fn formula_set(Json(req): Json<FormulaSetReq>) -> Json<ApiResponse<WriteResult>> {
    let params = SecurityParams { dry_run: req.dry_run, create_backup: true, file_path: req.path.clone() };
    match excel_write::set_formula(&req.path, &params, &req.sheet, &req.cell, &req.formula) {
        Ok(data) => Json(ApiResponse::ok(Some(data))),
        Err(e) => Json(ApiResponse::err(e)),
    }
}

#[derive(Deserialize)]
pub struct FormulaRefreshReq {
    pub path: String,
    pub sheet: String,
    #[serde(default)]
    pub dry_run: bool,
}

pub async fn formula_refresh(Json(req): Json<FormulaRefreshReq>) -> Json<ApiResponse<WriteResult>> {
    let params = SecurityParams { dry_run: req.dry_run, create_backup: true, file_path: req.path.clone() };
    match excel_write::refresh_formulas(&req.path, &params, &req.sheet) {
        Ok(data) => Json(ApiResponse::ok(Some(data))),
        Err(e) => Json(ApiResponse::err(e)),
    }
}

// ---------------------------------------------------------------------------
// Format
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct FormatSetReq {
    pub path: String,
    pub sheet: String,
    pub range: String,
    pub style: Style,
    #[serde(default)]
    pub dry_run: bool,
}

pub async fn format_set(Json(req): Json<FormatSetReq>) -> Json<ApiResponse<WriteResult>> {
    let params = SecurityParams { dry_run: req.dry_run, create_backup: true, file_path: req.path.clone() };
    match excel_write::set_format(&req.path, &params, &req.sheet, &req.range, &req.style) {
        Ok(data) => Json(ApiResponse::ok(Some(data))),
        Err(e) => Json(ApiResponse::err(e)),
    }
}

#[derive(Deserialize)]
pub struct MergeReq {
    pub path: String,
    pub sheet: String,
    pub range: String,
    #[serde(default)]
    pub value: String,
    #[serde(default)]
    pub dry_run: bool,
}

pub async fn cell_merge(Json(req): Json<MergeReq>) -> Json<ApiResponse<WriteResult>> {
    let params = SecurityParams { dry_run: req.dry_run, create_backup: true, file_path: req.path.clone() };
    match excel_write::merge_cells(&req.path, &params, &req.sheet, &req.range, &req.value) {
        Ok(data) => Json(ApiResponse::ok(Some(data))),
        Err(e) => Json(ApiResponse::err(e)),
    }
}

// ---------------------------------------------------------------------------
// Chart
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct ChartCreateReq {
    pub path: String,
    pub sheet: String,
    pub range: String,
    pub chart_type: String,
    pub title: Option<String>,
    #[serde(default)]
    pub dry_run: bool,
}

pub async fn chart_create(Json(req): Json<ChartCreateReq>) -> Json<ApiResponse<WriteResult>> {
    let ct = match req.chart_type.to_lowercase().as_str() {
        "column" => ChartType::Column,
        "line" => ChartType::Line,
        "pie" => ChartType::Pie,
        "bar" => ChartType::Bar,
        "area" => ChartType::Area,
        "scatter" => ChartType::Scatter,
        _ => return Json(ApiResponse::err(AppError::Custom(format!("Unknown chart type: {}", req.chart_type)))),
    };
    let (r1, c1, _, _) = match cell_ref::parse_range(&req.range) {
        Ok(v) => v,
        Err(e) => return Json(ApiResponse::err(e)),
    };
    let config = ChartConfig {
        chart_type: ct,
        title: req.title,
        categories_range: req.range.clone(),
        values_range: req.range,
        sheet: req.sheet,
        row: r1,
        col: c1,
    };
    let params = SecurityParams { dry_run: req.dry_run, create_backup: true, file_path: req.path.clone() };
    match excel_write::add_chart(&req.path, &params, &config) {
        Ok(data) => Json(ApiResponse::ok(Some(data))),
        Err(e) => Json(ApiResponse::err(e)),
    }
}

// ---------------------------------------------------------------------------
// VBA
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct VbaExportReq {
    pub path: String,
    pub output: String,
}

pub async fn vba_export(Json(req): Json<VbaExportReq>) -> Json<ApiResponse<String>> {
    match vba_util::export_vba(&req.path) {
        Ok(data) => {
            if let Err(e) = std::fs::write(&req.output, &data) {
                return Json(ApiResponse::err(AppError::Io(e)));
            }
            Json(ApiResponse::ok(Some(format!("Exported to {}", req.output))))
        }
        Err(e) => Json(ApiResponse::err(e)),
    }
}

#[derive(Deserialize)]
pub struct VbaImportReq {
    pub path: String,
    pub vba_file: String,
    #[serde(default)]
    pub dry_run: bool,
}

pub async fn vba_import(Json(req): Json<VbaImportReq>) -> Json<ApiResponse<WriteResult>> {
    let data = match std::fs::read(&req.vba_file) {
        Ok(d) => d,
        Err(e) => return Json(ApiResponse::err(AppError::Io(e))),
    };
    let params = SecurityParams { dry_run: req.dry_run, create_backup: true, file_path: req.path.clone() };
    match vba_util::import_vba(&req.path, &params, &data) {
        Ok(result) => Json(ApiResponse::ok(Some(result))),
        Err(e) => Json(ApiResponse::err(e)),
    }
}

// ---------------------------------------------------------------------------
// Diff
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct DiffFileReq {
    pub old_path: String,
    pub new_path: String,
    pub sheet: Option<String>,
}

pub async fn diff_file(Json(req): Json<DiffFileReq>) -> Json<ApiResponse<serde_json::Value>> {
    if let Some(ref sheet_name) = req.sheet {
        match excel_diff::diff_sheets(&req.old_path, &req.new_path, sheet_name) {
            Ok(data) => Json(ApiResponse::ok(Some(serde_json::to_value(data).unwrap()))),
            Err(e) => Json(ApiResponse::err(e)),
        }
    } else {
        match excel_diff::diff_files(&req.old_path, &req.new_path) {
            Ok(data) => Json(ApiResponse::ok(Some(serde_json::to_value(data).unwrap()))),
            Err(e) => Json(ApiResponse::err(e)),
        }
    }
}

#[derive(Deserialize)]
pub struct DiffRangeReq {
    pub old_path: String,
    pub new_path: String,
    pub sheet: String,
    pub range: String,
}

pub async fn diff_range(Json(req): Json<DiffRangeReq>) -> Json<ApiResponse<RangeDiff>> {
    match excel_diff::diff_range(&req.old_path, &req.new_path, &req.sheet, &req.range) {
        Ok(data) => Json(ApiResponse::ok(Some(data))),
        Err(e) => Json(ApiResponse::err(e)),
    }
}

// ---------------------------------------------------------------------------
// Health
// ---------------------------------------------------------------------------

pub async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "status": "ok", "version": "0.1.0" }))
}

// ---------------------------------------------------------------------------
// ApiResponse helpers
// ---------------------------------------------------------------------------

impl<T: Serialize> ApiResponse<T> {
    fn ok(data: Option<T>) -> Self {
        ApiResponse {
            success: true,
            message: String::new(),
            file_hash: None,
            data,
            diff: None,
            backup_info: None,
        }
    }

    fn err(e: AppError) -> Self {
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
// Helpers (shared with CLI)
// ---------------------------------------------------------------------------

fn parse_cell_value(s: &str) -> CellValue {
    if let Ok(n) = s.parse::<f64>() {
        return CellValue::Number(n);
    }
    match s.to_lowercase().as_str() {
        "true" => return CellValue::Bool(true),
        "false" => return CellValue::Bool(false),
        "null" | "none" | "empty" => return CellValue::Empty,
        _ => {}
    }
    CellValue::String(s.to_string())
}

fn json_val_to_cell_value(v: &serde_json::Value) -> CellValue {
    match v {
        serde_json::Value::Number(n) => CellValue::Number(n.as_f64().unwrap_or(0.0)),
        serde_json::Value::Bool(b) => CellValue::Bool(*b),
        serde_json::Value::String(s) => CellValue::String(s.clone()),
        serde_json::Value::Null => CellValue::Empty,
        _ => CellValue::String(v.to_string()),
    }
}

fn parse_filter_op(s: &str) -> Result<FilterOp> {
    match s.to_lowercase().as_str() {
        "eq" | "=" | "==" => Ok(FilterOp::Eq),
        "ne" | "!=" => Ok(FilterOp::Ne),
        "gt" | ">" => Ok(FilterOp::Gt),
        "lt" | "<" => Ok(FilterOp::Lt),
        "ge" | ">=" => Ok(FilterOp::Ge),
        "le" | "<=" => Ok(FilterOp::Le),
        "contains" => Ok(FilterOp::Contains),
        "startswith" | "starts_with" => Ok(FilterOp::StartsWith),
        "endswith" | "ends_with" => Ok(FilterOp::EndsWith),
        _ => Err(AppError::Custom(format!("Unknown filter operator: {}", s))),
    }
}
