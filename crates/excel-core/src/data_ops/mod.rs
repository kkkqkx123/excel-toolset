mod modify;
mod query;
mod rows;

pub use modify::modify_data_file;
pub use query::{dedup_sheet, filter_rows, sort_sheet};
pub use rows::{append_rows, delete_rows, insert_rows};

#[cfg_attr(not(feature = "sql"), allow(unused_imports))]
pub(crate) use modify::write_sheet_data;