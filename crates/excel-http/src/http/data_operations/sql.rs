use axum::Json;
use serde::Deserialize;

use excel_core::operations;
use excel_core::types::*;

#[derive(Deserialize)]
pub struct SqlReq {
    pub path: String,
    pub sheet: String,
    pub query: String,
}

pub async fn data_sql(Json(req): Json<SqlReq>) -> Json<ApiResponse<Vec<Vec<CellData>>>> {
    match operations::sql_query(&req.path, &req.sheet, &req.query) {
        Ok(data) => Json(ApiResponse::ok(Some(data))),
        Err(e) => Json(ApiResponse::err(e)),
    }
}