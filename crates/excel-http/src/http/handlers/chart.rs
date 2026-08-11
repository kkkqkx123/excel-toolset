use axum::Json;
use serde::Deserialize;

use excel_core::excel_write;
use excel_core::types::*;
use excel_core::utils::helpers;
use crate::http::response::ApiJson;

#[derive(Deserialize)]
pub struct ChartCreateReq {
    pub path: String,
    pub sheet: String,
    pub range: String,
    pub chart_type: String,
    pub title: Option<String>,
    #[serde(default)]
    pub dry_run: bool,
    /// Optional trendline configuration
    pub trendline: Option<ChartTrendlineConfig>,
    /// Optional Y error bars configuration
    pub y_error_bars: Option<ChartErrorBarsConfig>,
    /// Optional X error bars configuration
    pub x_error_bars: Option<ChartErrorBarsConfig>,
    /// Logarithmic base for Y axis
    pub log_base: Option<u16>,
}

pub async fn chart_create(Json(req): Json<ChartCreateReq>) -> ApiJson<WriteResult> {
    let ct = match helpers::chart_type_from_str(&req.chart_type) {
        Ok(t) => t,
        Err(e) => return ApiJson(ApiResponse::err(e)),
    };
    let (r1, c1, r2, c2) = match excel_core::utils::cell_ref::parse_range(&req.range) {
        Ok(v) => v,
        Err(e) => return ApiJson(ApiResponse::err(e)),
    };
    // Build sheet-qualified absolute ranges, exactly like the CLI does. Passing
    // the bare range (e.g. "B1:B4") for *both* categories and values made
    // rust_xlsxwriter reject the series with
    // "Chart series must contain a 'values' range".
    let col_name = excel_core::utils::cell_ref::index_to_col;
    let categories_range = format!(
        "'{}'!${}${}:${}${}",
        req.sheet,
        col_name(c1),
        r1 + 1,
        col_name(c1),
        r2 + 1
    );
    // First column = categories, remaining columns = values.
    let values_range = if c2 > c1 {
        format!(
            "'{}'!${}${}:${}${}",
            req.sheet,
            col_name(c1 + 1),
            r1 + 1,
            col_name(c2),
            r2 + 1
        )
    } else {
        categories_range.clone()
    };
    let config = ChartConfig {
        chart_type: ct,
        title: req.title,
        categories_range,
        values_range,
        sheet: req.sheet,
        // Place the chart below the data, matching CLI behaviour, so it does
        // not cover the source range.
        row: r2 + 1,
        col: c1,
        trendline: req.trendline,
        y_error_bars: req.y_error_bars,
        x_error_bars: req.x_error_bars,
        log_base: req.log_base,
    };
    let params = SecurityParams {
        dry_run: req.dry_run,
        create_backup: true,
        file_path: req.path.clone(),
    };
    match excel_write::add_chart(&req.path, &params, &config) {
        Ok(data) => ApiJson(ApiResponse::ok(Some(data))),
        Err(e) => ApiJson(ApiResponse::err(e)),
    }
}
