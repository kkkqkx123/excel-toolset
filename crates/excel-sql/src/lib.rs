pub mod config;
pub mod error;
pub mod utils;
mod converter;
mod db;
mod ops;

pub use converter::QueryResult;
pub use error::SqlResult;
pub use config::SqlConfig;
pub use ops::query::{filter_rows, sql_query};
pub use ops::write::{dedup_sheet, sort_sheet};
