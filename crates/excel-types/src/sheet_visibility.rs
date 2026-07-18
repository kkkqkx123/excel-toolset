//! Sheet visibility control types.
//!
//! Defines the visibility levels for Excel worksheets:
//! - Visible: normal visible sheet
//! - Hidden: hidden but user can unhide via Excel UI
//! - VeryHidden: deeply hidden, can only be unhidden via VBA

use serde::{Deserialize, Serialize};

/// Sheet visibility level.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SheetVisibility {
    Visible,
    Hidden,
    VeryHidden,
}

/// Request to set sheet visibility.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SheetVisibilityRequest {
    /// Target sheet name.
    pub sheet: String,
    /// Desired visibility level.
    pub visibility: SheetVisibility,
}
