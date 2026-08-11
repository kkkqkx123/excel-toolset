//! Transport-level response wrapper.
//!
//! `axum::Json` always emits `200 OK`, which meant every business failure
//! (missing file, unknown sheet, disabled feature, ...) was reported as a
//! success at the HTTP layer while only the body said `success:false`.
//!
//! [`ApiJson`] wraps [`ApiResponse`] and emits the status code the envelope
//! carries, so handlers keep their simple
//! `ApiJson(ApiResponse::ok(..)) / ApiJson(ApiResponse::err(e))` shape.

use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;

use excel_core::types::ApiResponse;

pub struct ApiJson<T: Serialize>(pub ApiResponse<T>);

impl<T: Serialize> IntoResponse for ApiJson<T> {
    fn into_response(self) -> Response {
        let status =
            StatusCode::from_u16(self.0.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        (status, Json(self.0)).into_response()
    }
}

impl<T: Serialize> From<ApiResponse<T>> for ApiJson<T> {
    fn from(r: ApiResponse<T>) -> Self {
        ApiJson(r)
    }
}
