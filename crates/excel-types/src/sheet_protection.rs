//! Sheet protection configuration types.
//!
//! Defines types for worksheet protection:
//! - SheetProtectionConfig: password and operation permissions
//! - ProtectionOptions: fine-grained control over allowed operations

use serde::{Deserialize, Serialize};

/// Configuration for protecting a worksheet.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SheetProtectionConfig {
    /// Target sheet name.
    pub sheet: String,
    /// Optional password for protection.
    /// When `None`, protection is set without a password.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    /// Protection options controlling which operations are allowed.
    #[serde(default)]
    pub options: ProtectionOptions,
}

/// Fine-grained options for worksheet protection.
///
/// Each field corresponds to a checkbox in Excel's "Protect Sheet" dialog.
/// When a field is `true`, the corresponding operation is ALLOWED while
/// the sheet is protected.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtectionOptions {
    /// Allow selecting locked cells.
    #[serde(default = "default_true")]
    pub select_locked_cells: bool,
    /// Allow selecting unlocked cells.
    #[serde(default = "default_true")]
    pub select_unlocked_cells: bool,
    /// Allow formatting cells.
    #[serde(default)]
    pub format_cells: bool,
    /// Allow formatting columns.
    #[serde(default)]
    pub format_columns: bool,
    /// Allow formatting rows.
    #[serde(default)]
    pub format_rows: bool,
    /// Allow inserting rows.
    #[serde(default)]
    pub insert_rows: bool,
    /// Allow inserting columns.
    #[serde(default)]
    pub insert_columns: bool,
    /// Allow inserting hyperlinks.
    #[serde(default)]
    pub insert_links: bool,
    /// Allow deleting rows.
    #[serde(default)]
    pub delete_rows: bool,
    /// Allow deleting columns.
    #[serde(default)]
    pub delete_columns: bool,
    /// Allow sorting.
    #[serde(default)]
    pub sort: bool,
    /// Allow using autofilter.
    #[serde(default)]
    pub auto_filter: bool,
    /// Allow using pivot tables.
    #[serde(default)]
    pub pivot_tables: bool,
    /// Allow editing scenarios.
    #[serde(default)]
    pub edit_scenarios: bool,
    /// Allow editing objects.
    #[serde(default)]
    pub edit_objects: bool,
    /// Allow modifying cell contents.
    #[serde(default)]
    pub contents: bool,
}

fn default_true() -> bool {
    true
}

impl Default for ProtectionOptions {
    fn default() -> Self {
        Self {
            select_locked_cells: true,
            select_unlocked_cells: true,
            format_cells: false,
            format_columns: false,
            format_rows: false,
            insert_rows: false,
            insert_columns: false,
            insert_links: false,
            delete_rows: false,
            delete_columns: false,
            sort: false,
            auto_filter: false,
            pivot_tables: false,
            edit_scenarios: false,
            edit_objects: false,
            contents: false,
        }
    }
}
