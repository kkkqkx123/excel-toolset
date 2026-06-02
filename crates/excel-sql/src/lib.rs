pub mod config;
mod converter;
mod db;
pub mod error;
mod ops;
pub mod utils;

pub use config::SqlConfig;
pub use converter::QueryResult;
pub use error::SqlResult;
pub use ops::query::{filter_rows_on_data, sql_query_on_data};
pub use ops::session::QuerySession;
pub use ops::write::{dedup_sheet_on_data, sort_sheet_on_data};

// 新增查询功能
pub use db::{
    ExcelQueryEngine, clear_database, drop_table, get_table_schema, list_tables, query,
    query_to_strings, query_with_params, table_exists, table_row_count,
};
