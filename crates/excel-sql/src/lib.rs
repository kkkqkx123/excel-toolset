pub mod config;
pub mod error;
pub mod utils;
mod converter;
mod db;
mod ops;

pub use converter::QueryResult;
pub use error::SqlResult;
pub use config::SqlConfig;
pub use ops::query::{sql_query_on_data, filter_rows_on_data};
pub use ops::write::{sort_sheet_on_data, dedup_sheet_on_data};
