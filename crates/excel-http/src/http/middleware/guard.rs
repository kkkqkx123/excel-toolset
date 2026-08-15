//! H4: transport-level hardening.
//!
//! Two independent guards, both configured through the environment so the
//! default local-developer experience is unchanged while a deployed instance
//! can be locked down:
//!
//! * `EXCEL_HTTP_TOKEN` — when set, every request (except `/health`) must
//!   carry `Authorization: Bearer <token>` or `?token=<token>`, else `401`.
//! * `EXCEL_HTTP_ROOT`  — when set, every `path`-like field in the JSON body
//!   must resolve inside that directory, else `403`. This is what stops
//!   `{"path":"/etc/passwd"}` from being read or overwritten.

use std::path::PathBuf;

use axum::body::{Body, Bytes};
use axum::extract::Request;
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use serde_json::Value;

use excel_core::security::validate_path_inside_root;

/// Body fields treated as filesystem paths.
const PATH_FIELDS: &[&str] = &[
    "path",
    "old_path",
    "new_path",
    "source_path",
    "target_path",
    "dest_path",
    "output_path",
    "backup_path",
    "image_path",
    "file_path",
];

/// Maximum body size we are willing to buffer for inspection (16 MiB).
const MAX_INSPECT_BYTES: usize = 16 * 1024 * 1024;

pub fn configured_token() -> Option<String> {
    std::env::var("EXCEL_HTTP_TOKEN")
        .ok()
        .filter(|t| !t.trim().is_empty())
}

pub fn configured_root() -> Option<PathBuf> {
    std::env::var("EXCEL_HTTP_ROOT")
        .ok()
        .filter(|r| !r.trim().is_empty())
        .map(PathBuf::from)
}

fn deny(status: StatusCode, code: &str, message: &str) -> Response {
    let body = serde_json::json!({
        "success": false,
        "message": message,
        "data": Value::Null,
        "error": { "code": code, "message": message },
    });
    (status, axum::Json(body)).into_response()
}

/// Bearer-token authentication. No-op when `EXCEL_HTTP_TOKEN` is unset.
pub async fn auth(req: Request, next: Next) -> Response {
    let Some(expected) = configured_token() else {
        return next.run(req).await;
    };

    // Health check stays open so orchestrators can probe without a secret.
    if req.uri().path() == "/health" {
        return next.run(req).await;
    }

    let header_token = req
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(str::trim)
        .map(str::to_string);

    let query_token = req.uri().query().and_then(|q| {
        q.split('&')
            .find_map(|kv| kv.strip_prefix("token=").map(str::to_string))
    });

    match header_token.or(query_token) {
        Some(t) if constant_time_eq(&t, &expected) => next.run(req).await,
        Some(_) => deny(
            StatusCode::UNAUTHORIZED,
            "UNAUTHORIZED",
            "invalid API token",
        ),
        None => deny(
            StatusCode::UNAUTHORIZED,
            "UNAUTHORIZED",
            "missing API token: send 'Authorization: Bearer <token>'",
        ),
    }
}

fn constant_time_eq(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

/// Rejects request bodies whose path fields escape `EXCEL_HTTP_ROOT`.
/// No-op when the variable is unset.
pub async fn path_guard(req: Request, next: Next) -> Response {
    let Some(root) = configured_root() else {
        return next.run(req).await;
    };

    let (parts, body) = req.into_parts();
    let bytes = match axum::body::to_bytes(body, MAX_INSPECT_BYTES).await {
        Ok(b) => b,
        Err(_) => {
            return deny(
                StatusCode::PAYLOAD_TOO_LARGE,
                "PAYLOAD_TOO_LARGE",
                "request body too large to validate",
            );
        }
    };

    if let Some(msg) = offending_path(&bytes, &root) {
        return deny(StatusCode::FORBIDDEN, "PATH_NOT_ALLOWED", &msg);
    }

    let req = Request::from_parts(parts, Body::from(bytes));
    next.run(req).await
}

fn offending_path(bytes: &Bytes, root: &std::path::Path) -> Option<String> {
    if bytes.is_empty() {
        return None;
    }
    let json: Value = serde_json::from_slice(bytes).ok()?;
    let mut found = None;
    walk(&json, root, &mut found);
    found
}

fn walk(v: &Value, root: &std::path::Path, found: &mut Option<String>) {
    if found.is_some() {
        return;
    }
    match v {
        Value::Object(map) => {
            for (k, val) in map {
                if PATH_FIELDS.contains(&k.as_str())
                    && let Some(p) = val.as_str()
                        && let Err(e) = validate_path_inside_root(p, root) {
                            *found = Some(e.to_string());
                            return;
                        }
                walk(val, root, found);
            }
        }
        Value::Array(items) => {
            for item in items {
                walk(item, root, found);
            }
        }
        _ => {}
    }
}
