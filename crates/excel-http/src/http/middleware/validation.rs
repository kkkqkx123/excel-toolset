#![allow(
    unused_variables,
    unused_imports,
    clippy::collapsible_if,
    clippy::collapsible_match
)]

use axum::Json;
use axum::extract::Request;
use serde_json::json;
use std::collections::HashMap;

pub fn validate_request(req: &Request) -> Result<(), Json<serde_json::Value>> {
    let uri = req.uri().path();

    if uri.starts_with("/api/file/") {
        validate_file_request(req, uri)
    } else if uri.starts_with("/api/sheet/") {
        validate_sheet_request(req, uri)
    } else if uri.starts_with("/api/cell/") {
        validate_cell_request(req, uri)
    } else if uri.starts_with("/api/range/") {
        validate_range_request(req, uri)
    } else if uri.starts_with("/api/data/") {
        validate_data_request(req, uri)
    } else {
        Ok(())
    }
}

fn validate_file_request(req: &Request, uri: &str) -> Result<(), Json<serde_json::Value>> {
    if uri.contains("/{path}") && !uri.ends_with("/{path}") {
        return Err(Json(json!({
            "error": "Invalid path parameter",
            "code": "INVALID_PATH"
        })));
    }
    Ok(())
}

fn validate_sheet_request(req: &Request, uri: &str) -> Result<(), Json<serde_json::Value>> {
    if uri.contains("/{path}") {
        if let Some(path) = uri.split("/{path}").next() {
            if path.is_empty() {
                return Err(Json(json!({
                    "error": "Path cannot be empty",
                    "code": "EMPTY_PATH"
                })));
            }
        }
    }
    Ok(())
}

fn validate_cell_request(req: &Request, uri: &str) -> Result<(), Json<serde_json::Value>> {
    let parts: Vec<&str> = uri.split('/').collect();
    if parts.len() < 6 {
        return Err(Json(json!({
            "error": "Invalid cell reference format. Expected: /api/cell/read/{path}/{sheet}/{cell}",
            "code": "INVALID_CELL_FORMAT"
        })));
    }
    Ok(())
}

fn validate_range_request(req: &Request, uri: &str) -> Result<(), Json<serde_json::Value>> {
    let parts: Vec<&str> = uri.split('/').collect();
    if parts.len() < 6 {
        return Err(Json(json!({
            "error": "Invalid range format. Expected: /api/range/read/{path}/{sheet}/{range}",
            "code": "INVALID_RANGE_FORMAT"
        })));
    }
    Ok(())
}

fn validate_data_request(req: &Request, uri: &str) -> Result<(), Json<serde_json::Value>> {
    if uri.ends_with("sql") {
        // SQL requests require special validation
        if let Some(content_type) = req.headers().get("content-type") {
            if content_type.to_str().unwrap_or("") != "application/json" {
                return Err(Json(json!({
                    "error": "Content-Type must be application/json for SQL requests",
                    "code": "INVALID_CONTENT_TYPE"
                })));
            }
        }
    }
    Ok(())
}

pub fn parse_path_params(uri: &str) -> HashMap<String, String> {
    let mut params = HashMap::new();
    let parts: Vec<&str> = uri.split('/').collect();

    if parts.len() >= 4 {
        if parts[1] == "api" {
            let resource = parts.get(2).unwrap_or(&"");
            let action = parts.get(3).unwrap_or(&"");

            match (*resource, *action) {
                ("file", "info") => {
                    if let Some(path) = parts.get(4) {
                        params.insert("path".to_string(), path.to_string());
                    }
                }
                ("sheet", "list") => {
                    if let Some(path) = parts.get(4) {
                        params.insert("path".to_string(), path.to_string());
                    }
                }
                ("cell", "read") => {
                    if parts.len() >= 6 {
                        params.insert("path".to_string(), parts[4].to_string());
                        params.insert("sheet".to_string(), parts[5].to_string());
                        params.insert("cell".to_string(), parts[6].to_string());
                    }
                }
                ("range", "read") => {
                    if parts.len() >= 6 {
                        params.insert("path".to_string(), parts[4].to_string());
                        params.insert("sheet".to_string(), parts[5].to_string());
                        params.insert("range".to_string(), parts[6].to_string());
                    }
                }
                _ => {}
            }
        }
    }

    params
}
