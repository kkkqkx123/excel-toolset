use axum::Json;
use serde::Deserialize;

use excel_core::excel_write;
use excel_core::types::*;

#[derive(Deserialize)]
pub struct ImageInsertReq {
    pub path: String,
    pub sheet: String,
    pub image_path: String,
    pub anchor_cell: String,
    #[serde(default)]
    pub x_scale: Option<f64>,
    #[serde(default)]
    pub y_scale: Option<f64>,
    #[serde(default)]
    pub x_offset: Option<u32>,
    #[serde(default)]
    pub y_offset: Option<u32>,
    #[serde(default)]
    pub alt_text: Option<String>,
}

#[derive(Deserialize)]
pub struct ImageRemoveReq {
    pub path: String,
    pub sheet: String,
    pub anchor_cell: String,
}

#[derive(Deserialize)]
pub struct ShapeInsertReq {
    pub path: String,
    pub sheet: String,
    /// Shape type: rectangle, rounded_rectangle, ellipse, line, text_box
    pub shape_type: String,
    pub anchor_cell: String,
    pub width: u32,
    pub height: u32,
    #[serde(default)]
    pub fill_color: Option<String>,
    #[serde(default)]
    pub line_color: Option<String>,
    #[serde(default)]
    pub line_width: Option<f64>,
    #[serde(default)]
    pub alt_text: Option<String>,
}

pub async fn image_insert(Json(req): Json<ImageInsertReq>) -> Json<ApiResponse<WriteResult>> {
    let params = SecurityParams {
        dry_run: false,
        create_backup: true,
        file_path: req.path.clone(),
    };

    let scale = match (req.x_scale, req.y_scale) {
        (Some(x), Some(y)) => Some(ImageScale {
            x_scale: x,
            y_scale: y,
        }),
        _ => None,
    };

    let config = ImageConfig {
        sheet: req.sheet,
        image_path: req.image_path,
        anchor_cell: req.anchor_cell,
        scale,
        x_offset: req.x_offset,
        y_offset: req.y_offset,
        alt_text: req.alt_text,
    };

    match excel_write::insert_image(&req.path, &params, &config) {
        Ok(data) => Json(ApiResponse::ok(Some(data))),
        Err(e) => Json(ApiResponse::err(e)),
    }
}

pub async fn image_remove(Json(req): Json<ImageRemoveReq>) -> Json<ApiResponse<WriteResult>> {
    let params = SecurityParams {
        dry_run: false,
        create_backup: true,
        file_path: req.path.clone(),
    };
    match excel_write::remove_image(&req.path, &params, &req.sheet, &req.anchor_cell) {
        Ok(data) => Json(ApiResponse::ok(Some(data))),
        Err(e) => Json(ApiResponse::err(e)),
    }
}

pub async fn shape_insert(Json(req): Json<ShapeInsertReq>) -> Json<ApiResponse<WriteResult>> {
    let params = SecurityParams {
        dry_run: false,
        create_backup: true,
        file_path: req.path.clone(),
    };

    let shape_type: ShapeType =
        serde_json::from_str(&format!("\"{}\"", req.shape_type))
            .map_err(|e| AppError::Serialize(format!("Invalid shape type: {}", e)))
            .unwrap_or(ShapeType::Rectangle);

    let config = ShapeConfig {
        sheet: req.sheet,
        shape_type,
        anchor_cell: req.anchor_cell,
        width: req.width,
        height: req.height,
        fill_color: req.fill_color,
        line_color: req.line_color,
        line_width: req.line_width,
        alt_text: req.alt_text,
    };

    match excel_write::insert_shape(&req.path, &params, &config) {
        Ok(data) => Json(ApiResponse::ok(Some(data))),
        Err(e) => Json(ApiResponse::err(e)),
    }
}
