use axum::Json;
use serde::Deserialize;

use excel_core::excel_write;
use excel_core::types::*;

#[derive(Deserialize)]
pub struct PageSetupConfigureReq {
    pub path: String,
    pub sheet: String,
    #[serde(default)]
    pub paper_size: Option<String>,
    #[serde(default)]
    pub orientation: Option<String>,
    #[serde(default)]
    pub margins: Option<PageMargins>,
    #[serde(default)]
    pub print_area: Option<String>,
    #[serde(default)]
    pub print_title_rows: Option<String>,
    #[serde(default)]
    pub print_title_cols: Option<String>,
    #[serde(default)]
    pub fit_to_pages_width: Option<u16>,
    #[serde(default)]
    pub fit_to_pages_height: Option<u16>,
    #[serde(default)]
    pub scale: Option<u16>,
    #[serde(default)]
    pub print_gridlines: bool,
    #[serde(default)]
    pub print_headings: bool,
    #[serde(default)]
    pub center_horizontally: bool,
    #[serde(default)]
    pub center_vertically: bool,
}

#[derive(Deserialize)]
pub struct PageBreakSetReq {
    pub path: String,
    pub sheet: String,
    #[serde(default)]
    pub horizontal_breaks: Vec<u32>,
    #[serde(default)]
    pub vertical_breaks: Vec<u16>,
}

#[derive(Deserialize)]
pub struct PageBreakClearReq {
    pub path: String,
    pub sheet: String,
}

pub async fn page_setup_configure(
    Json(req): Json<PageSetupConfigureReq>,
) -> Json<ApiResponse<WriteResult>> {
    let params = SecurityParams {
        dry_run: false,
        create_backup: true,
        file_path: req.path.clone(),
    };

    let fit_to_pages = match (req.fit_to_pages_width, req.fit_to_pages_height) {
        (Some(w), Some(h)) => Some(FitToPages { width: w, height: h }),
        _ => None,
    };

    let config = PageSetupConfig {
        sheet: req.sheet,
        paper_size: req.paper_size.and_then(|s| {
            serde_json::from_str(&format!("\"{}\"", s)).ok()
        }),
        orientation: req.orientation.and_then(|s| {
            serde_json::from_str(&format!("\"{}\"", s)).ok()
        }),
        margins: req.margins,
        print_area: req.print_area,
        print_title_rows: req.print_title_rows,
        print_title_cols: req.print_title_cols,
        fit_to_pages,
        scale: req.scale,
        print_gridlines: req.print_gridlines,
        print_headings: req.print_headings,
        center_horizontally: req.center_horizontally,
        center_vertically: req.center_vertically,
    };

    match excel_write::configure_page_setup(&req.path, &params, &config) {
        Ok(data) => Json(ApiResponse::ok(Some(data))),
        Err(e) => Json(ApiResponse::err(e)),
    }
}

pub async fn page_breaks_set(
    Json(req): Json<PageBreakSetReq>,
) -> Json<ApiResponse<WriteResult>> {
    let params = SecurityParams {
        dry_run: false,
        create_backup: true,
        file_path: req.path.clone(),
    };
    let config = PageBreakConfig {
        sheet: req.sheet,
        horizontal_breaks: req.horizontal_breaks,
        vertical_breaks: req.vertical_breaks,
    };
    match excel_write::set_page_breaks(&req.path, &params, &config) {
        Ok(data) => Json(ApiResponse::ok(Some(data))),
        Err(e) => Json(ApiResponse::err(e)),
    }
}

pub async fn page_breaks_clear(
    Json(req): Json<PageBreakClearReq>,
) -> Json<ApiResponse<WriteResult>> {
    let params = SecurityParams {
        dry_run: false,
        create_backup: true,
        file_path: req.path.clone(),
    };
    match excel_write::clear_page_breaks(&req.path, &params, &req.sheet) {
        Ok(data) => Json(ApiResponse::ok(Some(data))),
        Err(e) => Json(ApiResponse::err(e)),
    }
}
