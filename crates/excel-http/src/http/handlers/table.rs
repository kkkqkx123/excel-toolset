use axum::Json;
use serde::Deserialize;

use excel_core::excel_write;
use excel_core::types::*;

#[derive(Deserialize)]
pub struct TableCreateReq {
    pub path: String,
    pub config: TableConfig,
    #[serde(default)]
    pub dry_run: bool,
}

#[derive(Deserialize)]
pub struct TableRemoveReq {
    pub path: String,
    pub name: String,
    #[serde(default)]
    pub dry_run: bool,
}

#[derive(Deserialize)]
pub struct TableListReq {
    pub path: String,
}

#[derive(Deserialize)]
pub struct TableGetReq {
    pub path: String,
    pub name: String,
}

pub async fn table_create(Json(req): Json<TableCreateReq>) -> Json<ApiResponse<WriteResult>> {
    let params = SecurityParams {
        dry_run: req.dry_run,
        create_backup: true,
        file_path: req.path.clone(),
    };
    match excel_write::create_table(&req.path, &params, &req.config) {
        Ok(data) => Json(ApiResponse::ok(Some(data))),
        Err(e) => Json(ApiResponse::err(e)),
    }
}

pub async fn table_remove(Json(req): Json<TableRemoveReq>) -> Json<ApiResponse<WriteResult>> {
    let params = SecurityParams {
        dry_run: req.dry_run,
        create_backup: true,
        file_path: req.path.clone(),
    };
    match excel_write::remove_table(&req.path, &params, &req.name) {
        Ok(data) => Json(ApiResponse::ok(Some(data))),
        Err(e) => Json(ApiResponse::err(e)),
    }
}

pub async fn table_list(Json(req): Json<TableListReq>) -> Json<ApiResponse<Vec<TableInfo>>> {
    match excel_write::list_tables(&req.path) {
        Ok(data) => Json(ApiResponse::ok(Some(data))),
        Err(e) => Json(ApiResponse::err(e)),
    }
}

pub async fn table_get(Json(req): Json<TableGetReq>) -> Json<ApiResponse<TableInfo>> {
    match excel_write::get_table(&req.path, &req.name) {
        Ok(data) => Json(ApiResponse::ok(Some(data))),
        Err(e) => Json(ApiResponse::err(e)),
    }
}
