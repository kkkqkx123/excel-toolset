//! Image and shape insertion types.
//!
//! Provides types for embedding images (PNG, JPG, GIF, SVG) and basic shapes
//! (rectangles, ellipses, lines, text boxes) into worksheets.

use serde::{Deserialize, Serialize};

/// Image insertion configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageConfig {
    /// Target sheet name.
    pub sheet: String,
    /// Path to the image file on disk.
    pub image_path: String,
    /// Anchor cell for the top-left corner, e.g. "B2" (row, col reference).
    pub anchor_cell: String,
    /// Optional scaling.
    #[serde(default)]
    pub scale: Option<ImageScale>,
    /// Horizontal offset in pixels from the anchor cell's top-left corner.
    #[serde(default)]
    pub x_offset: Option<u32>,
    /// Vertical offset in pixels from the anchor cell's top-left corner.
    #[serde(default)]
    pub y_offset: Option<u32>,
    /// Alternative text for accessibility.
    #[serde(default)]
    pub alt_text: Option<String>,
}

/// Image scaling configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageScale {
    /// Horizontal scale factor (1.0 = original size).
    pub x_scale: f64,
    /// Vertical scale factor (1.0 = original size).
    pub y_scale: f64,
}

/// Shape insertion configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShapeConfig {
    /// Target sheet name.
    pub sheet: String,
    /// Shape type.
    pub shape_type: ShapeType,
    /// Anchor cell for the top-left corner, e.g. "B2".
    pub anchor_cell: String,
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Fill color as hex string, e.g. "FF0000".
    #[serde(default)]
    pub fill_color: Option<String>,
    /// Line/border color as hex string.
    #[serde(default)]
    pub line_color: Option<String>,
    /// Line/border width.
    #[serde(default)]
    pub line_width: Option<f64>,
    /// Alternative text for accessibility.
    #[serde(default)]
    pub alt_text: Option<String>,
}

/// Supported shape types.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ShapeType {
    /// Simple rectangle.
    Rectangle,
    /// Rounded rectangle with corner radius.
    RoundedRectangle,
    /// Ellipse / oval.
    Ellipse,
    /// Straight line.
    Line,
    /// Text box (rectangle with text content).
    TextBox,
}
