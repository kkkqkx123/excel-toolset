//! Freeze panes configuration types.
//!
//! Freeze panes keep specified rows and/or columns visible while scrolling.

use serde::{Deserialize, Serialize};

/// Configuration for setting freeze panes on a worksheet.
///
/// Supports row freezing, column freezing, or both simultaneously.
/// Set `rows=0` to freeze no rows, `cols=0` to freeze no columns.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FreezePanesConfig {
    /// Target sheet name.
    pub sheet: String,
    /// Number of rows to freeze from the top (0 means no row freeze).
    pub rows: u32,
    /// Number of columns to freeze from the left (0 means no column freeze).
    pub cols: u16,
}
