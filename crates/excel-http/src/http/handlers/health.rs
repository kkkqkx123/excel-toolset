use axum::Json;

use excel_core::types::ApiResponse;

/// Health check.
///
/// Uses the same `{success, data, error}` envelope as every other endpoint so
/// clients do not have to special-case it (the docs promise a unified shape).
pub async fn health() -> Json<ApiResponse<serde_json::Value>> {
    Json(ApiResponse::ok(Some(serde_json::json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION"),
    }))))
}
