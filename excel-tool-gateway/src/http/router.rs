use axum::{routing::{get, post}, Router};

use super::handlers;

pub fn create_router() -> Router {
    Router::new()
        // Health
        .route("/health", get(handlers::health))
        // File
        .route("/api/file/info/{path}", get(handlers::file_info))
        .route("/api/file/create", post(handlers::file_create))
        .route("/api/file/backup", post(handlers::file_backup))
        // Sheet
        .route("/api/sheet/list/{path}", get(handlers::sheet_list))
        .route("/api/sheet/add", post(handlers::sheet_add))
        .route("/api/sheet/delete", post(handlers::sheet_delete))
        .route("/api/sheet/rename", post(handlers::sheet_rename))
        // Cell
        .route("/api/cell/read/{path}/{sheet}/{cell}", get(handlers::cell_read))
        .route("/api/cell/write", post(handlers::cell_write))
        // Range
        .route("/api/range/read/{path}/{sheet}/{range}", get(handlers::range_read))
        .route("/api/range/write", post(handlers::range_write))
        .route("/api/range/clear", post(handlers::range_clear))
        // Data
        .route("/api/data/append-row", post(handlers::data_append_row))
        .route("/api/data/insert-row", post(handlers::data_insert_row))
        .route("/api/data/delete-row", post(handlers::data_delete_row))
        .route("/api/data/filter", post(handlers::data_filter))
        .route("/api/data/sort", post(handlers::data_sort))
        .route("/api/data/dedup", post(handlers::data_dedup))
        .route("/api/data/sql", post(handlers::data_sql))
        // Formula
        .route("/api/formula/set", post(handlers::formula_set))
        .route("/api/formula/refresh", post(handlers::formula_refresh))
        // Format
        .route("/api/format/set", post(handlers::format_set))
        .route("/api/cell/merge", post(handlers::cell_merge))
        // Chart
        .route("/api/chart/create", post(handlers::chart_create))
        // VBA
        .route("/api/vba/export", post(handlers::vba_export))
        .route("/api/vba/import", post(handlers::vba_import))
        // Diff
        .route("/api/diff/file", post(handlers::diff_file))
        .route("/api/diff/range", post(handlers::diff_range))
}
