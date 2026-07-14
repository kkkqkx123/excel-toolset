use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Style {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_size: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bold: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub italic: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub border: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub horizontal_align: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vertical_align: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub number_format: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ChartType {
    Column,
    ColumnStacked,
    ColumnPercentStacked,
    Line,
    LineStacked,
    LinePercentStacked,
    Pie,
    Doughnut,
    Bar,
    BarStacked,
    BarPercentStacked,
    Area,
    AreaStacked,
    AreaPercentStacked,
    Scatter,
    ScatterStraight,
    ScatterStraightWithMarkers,
    ScatterSmooth,
    ScatterSmoothWithMarkers,
    Stock,
    Radar,
    RadarWithMarkers,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChartTrendlineConfig {
    /// Trendline type: linear, logarithmic, polynomial, power, exponential, moving_average
    pub trend_type: String,
    /// Polynomial order (required when trend_type = "polynomial")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub polynomial_order: Option<u8>,
    /// Moving average period (required when trend_type = "moving_average")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub moving_average_period: Option<u32>,
    /// Forward forecast periods
    #[serde(skip_serializing_if = "Option::is_none")]
    pub forward_period: Option<f64>,
    /// Backward forecast periods
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backward_period: Option<f64>,
    /// Display trendline equation on chart
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_equation: Option<bool>,
    /// Display R-squared value on chart
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_r_squared: Option<bool>,
    /// Trendline name for legend
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChartErrorBarsConfig {
    /// Error bar type: fixed_value, percentage, standard_deviation, standard_error, custom
    pub error_type: String,
    /// Direction: plus, minus, both
    #[serde(skip_serializing_if = "Option::is_none")]
    pub direction: Option<String>,
    /// Value for fixed_value/percentage/standard_deviation types
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<f64>,
    /// Show end cap on error bars
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_cap: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChartConfig {
    pub chart_type: ChartType,
    pub title: Option<String>,
    pub categories_range: String,
    pub values_range: String,
    pub sheet: String,
    pub row: u32,
    pub col: u16,
    /// Optional trendline configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trendline: Option<ChartTrendlineConfig>,
    /// Optional Y error bars configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub y_error_bars: Option<ChartErrorBarsConfig>,
    /// Optional X error bars configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub x_error_bars: Option<ChartErrorBarsConfig>,
    /// Logarithmic base for Y axis (e.g., 10). None means linear scale.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub log_base: Option<u16>,
}
