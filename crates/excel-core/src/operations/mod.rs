pub mod core;
pub mod diff;
pub mod query;

pub use core::modify_data_file;
pub use query::{dedup_sheet, filter_rows, sort_sheet};

#[cfg(feature = "sql")]
pub use query::sql_query;
