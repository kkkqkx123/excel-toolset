use axum::{
    Router,
    routing::{get, post},
};

use super::{batch, cell, chart, data, diff, file, format, formula, health, range, sheet, vba};

pub fn create_router() -> Router {
    Router::new()
        .route("/health", get(health::health))
        .route("/api/file/info/{path}", get(file::file_info))
        .route("/api/file/create", post(file::file_create))
        .route("/api/file/backup", post(file::file_backup))
        .route("/api/sheet/list/{path}", get(sheet::sheet_list))
        .route("/api/sheet/add", post(sheet::sheet_add))
        .route("/api/sheet/delete", post(sheet::sheet_delete))
        .route("/api/sheet/rename", post(sheet::sheet_rename))
        .route("/api/cell/read/{path}/{sheet}/{cell}", get(cell::cell_read))
        .route("/api/cell/write", post(cell::cell_write))
        .route(
            "/api/range/read/{path}/{sheet}/{range}",
            get(range::range_read),
        )
        .route("/api/range/write", post(range::range_write))
        .route(
            "/api/range/write-from-csv",
            post(range::range_write_from_csv),
        )
        .route("/api/range/clear", post(range::range_clear))
        .route("/api/batch/modify", post(batch::batch_modify))
        .route("/api/data/append-row", post(data::data_append_row))
        .route("/api/data/insert-row", post(data::data_insert_row))
        .route("/api/data/delete-row", post(data::data_delete_row))
        .route("/api/data/filter", post(data::data_filter))
        .route("/api/data/sort", post(data::data_sort))
        .route("/api/data/dedup", post(data::data_dedup))
        .route("/api/data/sql", post(data::data_sql))
        .route("/api/formula/set", post(formula::formula_set))
        .route("/api/formula/refresh", post(formula::formula_refresh))
        .route("/api/format/set", post(format::format_set))
        .route("/api/cell/merge", post(format::cell_merge))
        .route("/api/chart/create", post(chart::chart_create))
        .route("/api/vba/export", post(vba::vba_export))
        .route("/api/vba/import", post(vba::vba_import))
        .route("/api/diff/file", post(diff::diff_file))
        .route("/api/diff/range", post(diff::handle_diff_range))
}
