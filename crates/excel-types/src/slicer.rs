use serde::{Deserialize, Serialize};

/// Configuration for creating a slicer on a pivot table.
///
/// A slicer provides interactive visual filtering for pivot table fields,
/// allowing users to quickly filter data by clicking on filter buttons.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlicerConfig {
    /// Display name / caption for the slicer
    pub name: String,
    /// Name of the pivot table this slicer is associated with
    pub pivot_table_name: String,
    /// Source data range that the pivot table is built on (e.g. "Sheet1!A1:E100")
    pub source_range: String,
    /// 0-based column index of the field to slice on (within the source data)
    pub field_column: u16,
    /// Target worksheet where the slicer will be placed
    pub target_sheet: String,
    /// Position and dimensions of the slicer on the target sheet
    pub position: SlicerPosition,
    /// Optional built-in style (e.g. "SlicerStyleLight1" through "SlicerStyleLight6",
    /// "SlicerStyleDark1" through "SlicerStyleDark6")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<String>,
    /// Number of button columns in the slicer (default 1)
    #[serde(default = "default_slicer_columns")]
    pub columns: u32,
    /// Whether to show the slicer header
    #[serde(default = "default_true")]
    pub show_header: bool,
    /// Names of additional pivot tables that should be linked to this slicer
    #[serde(default)]
    pub linked_pivots: Vec<String>,
}

fn default_slicer_columns() -> u32 {
    1
}

fn default_true() -> bool {
    true
}

/// Position and size of a slicer on a worksheet.
///
/// Coordinates are in EMU (English Metric Units) as used by DrawingML.
/// 1 pixel ~= 9525 EMU at 96 DPI, but pixel values are also accepted
/// and will be converted internally.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlicerPosition {
    /// Column offset in pixels from the left of the worksheet
    pub col: u32,
    /// Row offset in pixels from the top of the worksheet
    pub row: u32,
    /// Width of the slicer in pixels
    pub width: u32,
    /// Height of the slicer in pixels
    pub height: u32,
}

impl SlicerPosition {
    /// Convert pixel values to EMU (1 pixel = 9525 EMU at 96 DPI).
    pub fn to_emu(&self) -> (i64, i64, i64, i64) {
        let factor = 9525i64;
        (
            self.col as i64 * factor,
            self.row as i64 * factor,
            self.width as i64 * factor,
            self.height as i64 * factor,
        )
    }
}
