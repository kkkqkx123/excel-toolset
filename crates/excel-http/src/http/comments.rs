use axum::Json;
use serde::Deserialize;

use excel_core::features::comments;
use excel_core::types::*;

#[derive(Deserialize)]
pub struct GetCommentReq {
    pub path: String,
    pub sheet: String,
    pub cell: String,
}

#[derive(Deserialize)]
pub struct AddCommentReq {
    pub path: String,
    pub sheet: String,
    pub cell: String,
    pub comment: String,
    #[serde(default)]
    pub dry_run: bool,
}

#[derive(Deserialize)]
pub struct UpdateCommentReq {
    pub path: String,
    pub sheet: String,
    pub cell: String,
    pub comment: String,
    #[serde(default)]
    pub dry_run: bool,
}

#[derive(Deserialize)]
pub struct DeleteCommentReq {
    pub path: String,
    pub sheet: String,
    pub cell: String,
    #[serde(default)]
    pub dry_run: bool,
}

pub async fn get_comment(
    Json(req): Json<GetCommentReq>,
) -> Json<ApiResponse<Option<comments::Comment>>> {
    match comments::get_comment(&req.path, &req.sheet, &req.cell) {
        Ok(comment) => Json(ApiResponse::ok(Some(comment))),
        Err(e) => Json(ApiResponse::err(e)),
    }
}

pub async fn add_comment(Json(req): Json<AddCommentReq>) -> Json<ApiResponse<WriteResult>> {
    let params = SecurityParams {
        dry_run: req.dry_run,
        create_backup: true,
        file_path: req.path.clone(),
    };

    match comments::add_comment(&req.path, &req.sheet, &req.cell, &req.comment, &params) {
        Ok(result) => Json(ApiResponse::ok(Some(result))),
        Err(e) => Json(ApiResponse::err(e)),
    }
}

pub async fn update_comment(Json(req): Json<UpdateCommentReq>) -> Json<ApiResponse<WriteResult>> {
    let params = SecurityParams {
        dry_run: req.dry_run,
        create_backup: true,
        file_path: req.path.clone(),
    };

    match comments::update_comment(&req.path, &req.sheet, &req.cell, &req.comment, &params) {
        Ok(result) => Json(ApiResponse::ok(Some(result))),
        Err(e) => Json(ApiResponse::err(e)),
    }
}

pub async fn delete_comment(Json(req): Json<DeleteCommentReq>) -> Json<ApiResponse<WriteResult>> {
    let params = SecurityParams {
        dry_run: req.dry_run,
        create_backup: true,
        file_path: req.path.clone(),
    };

    match comments::delete_comment(&req.path, &req.sheet, &req.cell, &params) {
        Ok(result) => Json(ApiResponse::ok(Some(result))),
        Err(e) => Json(ApiResponse::err(e)),
    }
}
