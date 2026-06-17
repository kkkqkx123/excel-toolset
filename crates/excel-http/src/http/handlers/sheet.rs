use axum::Json;
use serde::Deserialize;

use excel_core::excel_read;
use excel_core::excel_write;
use excel_core::types::*;

#[derive(Deserialize)]
pub struct SheetNameReq {
    pub path: String,
    pub name: String,
}

#[derive(Deserialize)]
pub struct RenameSheetReq {
    pub path: String,
    pub old: String,
    pub new: String,
}

#[derive(Deserialize)]
pub struct SheetListReq {
    pub path: String,
}

pub async fn sheet_list(Json(req): Json<SheetListReq>) -> Json<ApiResponse<Vec<String>>> {
    match excel_read::list_sheets(&req.path) {
        Ok(data) => Json(ApiResponse::ok(Some(data))),
        Err(e) => Json(ApiResponse::err(e)),
    }
}

pub async fn sheet_add(Json(req): Json<SheetNameReq>) -> Json<ApiResponse<WriteResult>> {
    let params = SecurityParams {
        dry_run: false,
        create_backup: true,
        file_path: req.path.clone(),
    };
    match excel_write::add_sheet(&req.path, &params, &req.name) {
        Ok(data) => Json(ApiResponse::ok(Some(data))),
        Err(e) => Json(ApiResponse::err(e)),
    }
}

pub async fn sheet_delete(Json(req): Json<SheetNameReq>) -> Json<ApiResponse<WriteResult>> {
    let params = SecurityParams {
        dry_run: false,
        create_backup: true,
        file_path: req.path.clone(),
    };
    match excel_write::delete_sheet(&req.path, &params, &req.name) {
        Ok(data) => Json(ApiResponse::ok(Some(data))),
        Err(e) => Json(ApiResponse::err(e)),
    }
}

pub async fn sheet_rename(Json(req): Json<RenameSheetReq>) -> Json<ApiResponse<WriteResult>> {
    let params = SecurityParams {
        dry_run: false,
        create_backup: true,
        file_path: req.path.clone(),
    };
    match excel_write::rename_sheet(&req.path, &params, &req.old, &req.new) {
        Ok(data) => Json(ApiResponse::ok(Some(data))),
        Err(e) => Json(ApiResponse::err(e)),
    }
}
