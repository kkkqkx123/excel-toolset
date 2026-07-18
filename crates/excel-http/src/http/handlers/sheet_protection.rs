use axum::Json;
use serde::Deserialize;

use excel_core::excel_write;
use excel_core::types::*;

#[derive(Deserialize)]
pub struct SheetProtectionProtectReq {
    pub path: String,
    pub sheet: String,
    #[serde(default)]
    pub password: Option<String>,
    #[serde(default)]
    pub options: Option<ProtectionOptions>,
}

#[derive(Deserialize)]
pub struct SheetProtectionSheetReq {
    pub path: String,
    pub sheet: String,
}

pub async fn sheet_protection_protect(
    Json(req): Json<SheetProtectionProtectReq>,
) -> Json<ApiResponse<WriteResult>> {
    let params = SecurityParams {
        dry_run: false,
        create_backup: true,
        file_path: req.path.clone(),
    };
    let config = SheetProtectionConfig {
        sheet: req.sheet,
        password: req.password,
        options: req.options.unwrap_or_default(),
    };
    match excel_write::protect_sheet(&req.path, &params, &config) {
        Ok(data) => Json(ApiResponse::ok(Some(data))),
        Err(e) => Json(ApiResponse::err(e)),
    }
}

pub async fn sheet_protection_unprotect(
    Json(req): Json<SheetProtectionSheetReq>,
) -> Json<ApiResponse<WriteResult>> {
    let params = SecurityParams {
        dry_run: false,
        create_backup: true,
        file_path: req.path.clone(),
    };
    match excel_write::unprotect_sheet(&req.path, &params, &req.sheet) {
        Ok(data) => Json(ApiResponse::ok(Some(data))),
        Err(e) => Json(ApiResponse::err(e)),
    }
}

pub async fn sheet_protection_is_protected(
    Json(req): Json<SheetProtectionSheetReq>,
) -> Json<ApiResponse<bool>> {
    match excel_write::is_sheet_protected(&req.path, &req.sheet) {
        Ok(data) => Json(ApiResponse::ok(Some(data))),
        Err(e) => Json(ApiResponse::err(e)),
    }
}
