//! AutoFilter configuration types.
//!
//! Defines types for Excel native AutoFilter feature:
//! - AutoFilterConfig: setting an autofilter range on a worksheet
//! - AutoFilterInfo: reading the current autofilter state

use serde::{Deserialize, Serialize};

/// Configuration for setting an Excel native AutoFilter on a worksheet.
///
/// AutoFilter adds dropdown arrows to column headers, allowing users
/// to filter rows within the specified range.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoFilterConfig {
    /// Target sheet name.
    pub sheet: String,
    /// Range to apply the autofilter to, including header row.
    /// Format: "A1:D100".
    pub range: String,
}

/// Information about the current AutoFilter state on a worksheet.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoFilterInfo {
    /// Sheet name.
    pub sheet: String,
    /// The autofilter range string, if set.
    pub range: Option<String>,
    /// Whether the autofilter is enabled.
    pub enabled: bool,
}
