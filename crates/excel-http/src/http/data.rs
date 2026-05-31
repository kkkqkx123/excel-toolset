use axum::Json;
use serde::Deserialize;

use excel_core::excel_data;
use excel_core::helpers;
use excel_core::types::*;

#[derive(Deserialize)]
pub struct RowOpReq {
    pub path: String,
    pub sheet: String,
    pub values: Vec<String>,
    #[serde(default)]
    pub dry_run: bool,
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

#[derive(Deserialize)]
pub struct DeleteRowReq {
    pub path: String,
    pub sheet: String,
    pub row: u32,
    #[serde(default)]
    pub dry_run: bool,
}

#[derive(Deserialize)]
pub struct FilterReq {
    pub path: String,
    pub sheet: String,
    pub column: u16,
    pub operator: String,
    pub value: String,
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

#[derive(Deserialize)]
pub struct DedupReq {
    pub path: String,
    pub sheet: String,
    pub column: Option<u16>,
    #[serde(default)]
    pub dry_run: bool,
}

#[derive(Deserialize)]
pub struct SqlReq {
    pub path: String,
    pub sheet: String,
    pub query: String,
}

// ---------------------------------------------------------------------------
// Feature-gated dispatch: Rust fallback vs DuckDB SQL engine
// ---------------------------------------------------------------------------

#[cfg(feature = "sql")]
fn filter_rows_dispatch(
    path: &str,
    sheet: &str,
    conditions: &[FilterCondition],
) -> Result<Vec<Vec<CellData>>> {
    excel_sql::filter_rows(path, sheet, conditions)
}

#[cfg(not(feature = "sql"))]
fn filter_rows_dispatch(
    path: &str,
    sheet: &str,
    conditions: &[FilterCondition],
) -> Result<Vec<Vec<CellData>>> {
    excel_data::filter_rows(path, sheet, conditions)
}

#[cfg(feature = "sql")]
fn sort_sheet_dispatch(
    path: &str,
    params: &SecurityParams,
    sheet: &str,
    sort_columns: &[SortColumn],
) -> Result<WriteResult> {
    excel_sql::sort_sheet(path, params, sheet, sort_columns)
}

#[cfg(not(feature = "sql"))]
fn sort_sheet_dispatch(
    path: &str,
    params: &SecurityParams,
    sheet: &str,
    sort_columns: &[SortColumn],
) -> Result<WriteResult> {
    excel_data::sort_sheet(path, params, sheet, sort_columns)
}

#[cfg(feature = "sql")]
fn dedup_sheet_dispatch(
    path: &str,
    params: &SecurityParams,
    sheet: &str,
    columns: &[u16],
) -> Result<WriteResult> {
    excel_sql::dedup_sheet(path, params, sheet, columns)
}

#[cfg(not(feature = "sql"))]
fn dedup_sheet_dispatch(
    path: &str,
    params: &SecurityParams,
    sheet: &str,
    columns: &[u16],
) -> Result<WriteResult> {
    excel_data::dedup_sheet(path, params, sheet, columns)
}

#[cfg(feature = "sql")]
fn sql_query_dispatch(path: &str, _sheet: &str, query: &str) -> Result<Vec<Vec<CellData>>> {
    excel_sql::sql_query(path, query)
}

#[cfg(not(feature = "sql"))]
fn sql_query_dispatch(_path: &str, _sheet: &str, _query: &str) -> Result<Vec<Vec<CellData>>> {
    Err(AppError::FeatureNotEnabled(
        "SQL queries require the 'sql' feature (enable with --features sql)".into(),
    ))
}

pub async fn data_append_row(Json(req): Json<RowOpReq>) -> Json<ApiResponse<WriteResult>> {
    let row: Vec<Vec<CellValue>> = vec![
        req.values
            .iter()
            .map(|v| helpers::parse_cell_value(v))
            .collect(),
    ];
    let params = SecurityParams {
        dry_run: req.dry_run,
        create_backup: true,
        file_path: req.path.clone(),
    };
    match excel_data::append_rows(&req.path, &params, &req.sheet, &row) {
        Ok(data) => Json(ApiResponse::ok(Some(data))),
        Err(e) => Json(ApiResponse::err(e)),
    }
}

pub async fn data_insert_row(Json(req): Json<InsertRowReq>) -> Json<ApiResponse<WriteResult>> {
    let row: Vec<Vec<CellValue>> = vec![
        req.values
            .iter()
            .map(|v| helpers::parse_cell_value(v))
            .collect(),
    ];
    let params = SecurityParams {
        dry_run: req.dry_run,
        create_backup: true,
        file_path: req.path.clone(),
    };
    match excel_data::insert_rows(&req.path, &params, &req.sheet, req.row, &row) {
        Ok(data) => Json(ApiResponse::ok(Some(data))),
        Err(e) => Json(ApiResponse::err(e)),
    }
}

pub async fn data_delete_row(Json(req): Json<DeleteRowReq>) -> Json<ApiResponse<WriteResult>> {
    let params = SecurityParams {
        dry_run: req.dry_run,
        create_backup: true,
        file_path: req.path.clone(),
    };
    match excel_data::delete_rows(&req.path, &params, &req.sheet, req.row, req.row) {
        Ok(data) => Json(ApiResponse::ok(Some(data))),
        Err(e) => Json(ApiResponse::err(e)),
    }
}

pub async fn data_filter(Json(req): Json<FilterReq>) -> Json<ApiResponse<Vec<Vec<CellData>>>> {
    let filter_op = match helpers::parse_filter_op(&req.operator) {
        Ok(op) => op,
        Err(e) => return Json(ApiResponse::err(e)),
    };
    let conditions = vec![FilterCondition {
        column: req.column,
        operator: filter_op,
        value: req.value,
    }];
    match filter_rows_dispatch(&req.path, &req.sheet, &conditions) {
        Ok(data) => Json(ApiResponse::ok(Some(data))),
        Err(e) => Json(ApiResponse::err(e)),
    }
}

pub async fn data_sort(Json(req): Json<SortReq>) -> Json<ApiResponse<WriteResult>> {
    let sort_cols = vec![SortColumn {
        column: req.column,
        descending: req.descending,
    }];
    let params = SecurityParams {
        dry_run: req.dry_run,
        create_backup: true,
        file_path: req.path.clone(),
    };
    match sort_sheet_dispatch(&req.path, &params, &req.sheet, &sort_cols) {
        Ok(data) => Json(ApiResponse::ok(Some(data))),
        Err(e) => Json(ApiResponse::err(e)),
    }
}

pub async fn data_dedup(Json(req): Json<DedupReq>) -> Json<ApiResponse<WriteResult>> {
    let cols = req.column.map(|c| vec![c]).unwrap_or_default();
    let params = SecurityParams {
        dry_run: req.dry_run,
        create_backup: true,
        file_path: req.path.clone(),
    };
    match dedup_sheet_dispatch(&req.path, &params, &req.sheet, &cols) {
        Ok(data) => Json(ApiResponse::ok(Some(data))),
        Err(e) => Json(ApiResponse::err(e)),
    }
}

pub async fn data_sql(Json(req): Json<SqlReq>) -> Json<ApiResponse<Vec<Vec<CellData>>>> {
    match sql_query_dispatch(&req.path, &req.sheet, &req.query) {
        Ok(data) => Json(ApiResponse::ok(Some(data))),
        Err(e) => Json(ApiResponse::err(e)),
    }
}
