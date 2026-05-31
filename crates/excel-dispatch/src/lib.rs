#[cfg(not(feature = "sql"))]
use excel_core::excel_data;
use excel_core::types::*;

// ---------------------------------------------------------------------------
// Feature-gated dispatch: Rust fallback vs DuckDB SQL engine
// ---------------------------------------------------------------------------

#[cfg(feature = "sql")]
pub fn filter_rows_dispatch(
    path: &str,
    sheet: &str,
    conditions: &[FilterCondition],
) -> Result<Vec<Vec<CellData>>> {
    excel_sql::filter_rows(path, sheet, conditions)
}

#[cfg(not(feature = "sql"))]
pub fn filter_rows_dispatch(
    path: &str,
    sheet: &str,
    conditions: &[FilterCondition],
) -> Result<Vec<Vec<CellData>>> {
    excel_data::filter_rows(path, sheet, conditions)
}

#[cfg(feature = "sql")]
pub fn sort_sheet_dispatch(
    path: &str,
    params: &SecurityParams,
    sheet: &str,
    sort_columns: &[SortColumn],
) -> Result<WriteResult> {
    excel_sql::sort_sheet(path, params, sheet, sort_columns)
}

#[cfg(not(feature = "sql"))]
pub fn sort_sheet_dispatch(
    path: &str,
    params: &SecurityParams,
    sheet: &str,
    sort_columns: &[SortColumn],
) -> Result<WriteResult> {
    excel_data::sort_sheet(path, params, sheet, sort_columns)
}

#[cfg(feature = "sql")]
pub fn dedup_sheet_dispatch(
    path: &str,
    params: &SecurityParams,
    sheet: &str,
    columns: &[u16],
) -> Result<WriteResult> {
    excel_sql::dedup_sheet(path, params, sheet, columns)
}

#[cfg(not(feature = "sql"))]
pub fn dedup_sheet_dispatch(
    path: &str,
    params: &SecurityParams,
    sheet: &str,
    columns: &[u16],
) -> Result<WriteResult> {
    excel_data::dedup_sheet(path, params, sheet, columns)
}

#[cfg(feature = "sql")]
pub fn sql_query_dispatch(path: &str, _sheet: &str, query: &str) -> Result<Vec<Vec<CellData>>> {
    excel_sql::sql_query(path, query)
}

#[cfg(not(feature = "sql"))]
pub fn sql_query_dispatch(_path: &str, _sheet: &str, _query: &str) -> Result<Vec<Vec<CellData>>> {
    Err(AppError::FeatureNotEnabled(
        "SQL queries require the 'sql' feature (enable with --features sql)".into(),
    ))
}
