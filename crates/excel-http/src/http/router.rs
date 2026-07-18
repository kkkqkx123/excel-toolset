use axum::{
    Router,
    routing::{delete, get, post},
};

use super::{
    data_operations::{filter, rows, sql},
    formatting::{cell_format, conditional, merge},
    formula::{analysis, basic},
    handlers::{
        auto_filter, batch, cell, chart, comments, data_validation, diff, file, formula_ops,
        freeze_panes, health, image, named_ranges, page_setup, pivot_table, range, search, sheet,
        sheet_protection, slicer, sparkline, table, vba, workbook_overview,
    },
};

pub fn create_router() -> Router {
    Router::new()
        .route("/health", get(health::health))
        .route("/api/file/info", post(file::file_info))
        .route("/api/file/create", post(file::file_create))
        .route("/api/file/backup", post(file::file_backup))
        .route("/api/file/rollback", post(file::file_rollback))
        .route("/api/sheet/list", post(sheet::sheet_list))
        .route("/api/sheet/add", post(sheet::sheet_add))
        .route("/api/sheet/delete", post(sheet::sheet_delete))
        .route("/api/sheet/rename", post(sheet::sheet_rename))
        .route("/api/sheet/visibility", post(sheet::sheet_set_visibility))
        .route(
            "/api/freeze-panes/set",
            post(freeze_panes::freeze_panes_set),
        )
        .route(
            "/api/freeze-panes/clear",
            post(freeze_panes::freeze_panes_clear),
        )
        .route("/api/cell/read", post(cell::cell_read))
        .route("/api/cell/write", post(cell::cell_write))
        .route("/api/range/read", post(range::range_read))
        .route("/api/range/write", post(range::range_write))
        .route(
            "/api/range/write-from-csv",
            post(range::range_write_from_csv),
        )
        .route("/api/range/clear", post(range::range_clear))
        .route("/api/batch/modify", post(batch::batch_modify))
        .route(
            "/api/batch/validate_formula",
            post(batch::batch_validate_formula),
        )
        .route("/api/data/append-row", post(rows::data_append_row))
        .route("/api/data/insert-row", post(rows::data_insert_row))
        .route("/api/data/delete-row", post(rows::data_delete_row))
        .route("/api/data/filter", post(filter::data_filter))
        .route("/api/data/sort", post(filter::data_sort))
        .route("/api/data/dedup", post(filter::data_dedup))
        .route("/api/data/sql", post(sql::data_sql))
        .route("/api/data/sql_session", post(sql::create_session))
        .route("/api/data/sql_session/:id", delete(sql::close_session))
        .route("/api/formula/set", post(basic::formula_set))
        .route("/api/formula/refresh", post(basic::formula_refresh))
        .route("/api/formula/read", post(basic::formula_read))
        .route("/api/formula/calc-mode", post(basic::formula_calc_mode))
        .route(
            "/api/formula/trace_dependencies",
            post(analysis::trace_dependencies),
        )
        .route("/api/formula/explain", post(analysis::explain_formula))
        .route(
            "/api/formula/explain_logic",
            post(analysis::explain_formula_logic),
        )
        .route("/api/formula/fill", post(formula_ops::formula_fill))
        .route("/api/formula/evaluate", post(formula_ops::formula_evaluate))
        .route(
            "/api/formula/evaluate-batch",
            post(formula_ops::formula_evaluate_batch),
        )
        .route("/api/search/workbook", post(search::search_workbook))
        .route("/api/search/sheet", post(search::search_sheet))
        .route("/api/format/set", post(cell_format::format_set))
        .route("/api/cell/merge", post(merge::cell_merge))
        .route("/api/chart/create", post(chart::chart_create))
        .route("/api/comments/get", post(comments::get_comment))
        .route("/api/comments/add", post(comments::add_comment))
        .route("/api/comments/update", post(comments::update_comment))
        .route("/api/comments/delete", post(comments::delete_comment))
        .route(
            "/api/named_ranges/list",
            post(named_ranges::list_named_ranges),
        )
        .route(
            "/api/named_ranges/get_value",
            post(named_ranges::get_named_range_value),
        )
        .route(
            "/api/named_ranges/create",
            post(named_ranges::create_named_range),
        )
        .route(
            "/api/named_ranges/delete",
            post(named_ranges::delete_named_range),
        )
        .route(
            "/api/conditional_format/add",
            post(conditional::add_conditional_format),
        )
        .route(
            "/api/conditional_format/remove",
            post(conditional::remove_conditional_format),
        )
        .route("/api/vba/export", post(vba::vba_export))
        .route("/api/vba/import", post(vba::vba_import))
        .route("/api/vba/has", post(vba::vba_has))
        .route("/api/diff/file", post(diff::diff_file))
        .route("/api/diff/range", post(diff::handle_diff_range))
        .route("/api/diff/semantic", post(diff::diff_semantic))
        .route(
            "/api/diff/formula_dependencies",
            post(diff::diff_formula_dependencies_handler),
        )
        .route("/api/table/create", post(table::table_create))
        .route("/api/table/remove", post(table::table_remove))
        .route("/api/table/list", post(table::table_list))
        .route("/api/table/get", post(table::table_get))
        .route(
            "/api/data_validation/add",
            post(data_validation::data_validation_add),
        )
        .route(
            "/api/data_validation/remove",
            post(data_validation::data_validation_remove),
        )
        .route(
            "/api/pivot_table/create",
            post(pivot_table::pivot_table_create),
        )
        .route("/api/slicer/create", post(slicer::slicer_create))
        .route("/api/sparkline/add", post(sparkline::sparkline_add))
        .route("/api/sparkline/remove", post(sparkline::sparkline_remove))
        .route(
            "/api/workbook/overview",
            post(workbook_overview::workbook_overview),
        )
        .route(
            "/api/workbook/history",
            post(workbook_overview::workbook_history),
        )
        .route(
            "/api/workbook/sheet_overview",
            post(workbook_overview::sheet_overview),
        )
        .route("/api/auto-filter/set", post(auto_filter::auto_filter_set))
        .route(
            "/api/auto-filter/remove",
            post(auto_filter::auto_filter_remove),
        )
        .route("/api/auto-filter/get", post(auto_filter::auto_filter_get))
        .route(
            "/api/protection/sheet/protect",
            post(sheet_protection::sheet_protection_protect),
        )
        .route(
            "/api/protection/sheet/unprotect",
            post(sheet_protection::sheet_protection_unprotect),
        )
        .route(
            "/api/protection/sheet/is-protected",
            post(sheet_protection::sheet_protection_is_protected),
        )
        .route(
            "/api/page-setup/configure",
            post(page_setup::page_setup_configure),
        )
        .route(
            "/api/page-setup/page-breaks/set",
            post(page_setup::page_breaks_set),
        )
        .route(
            "/api/page-setup/page-breaks/clear",
            post(page_setup::page_breaks_clear),
        )
        .route("/api/image/insert", post(image::image_insert))
        .route("/api/image/remove", post(image::image_remove))
        .route("/api/image/shape/insert", post(image::shape_insert))
}
