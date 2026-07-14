use serde::{Deserialize, Serialize};

/// Data validation rule types supported by Excel.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DataValidationType {
    /// Dropdown list validation (list of allowed values)
    List,
    /// Whole number validation
    Whole,
    /// Decimal number validation
    Decimal,
    /// Date validation
    Date,
    /// Time validation
    Time,
    /// Text length validation
    TextLength,
    /// Custom formula-based validation
    Custom,
}

/// Operator used in data validation criteria.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DataValidationOperator {
    Between,
    NotBetween,
    Equal,
    NotEqual,
    GreaterThan,
    LessThan,
    GreaterThanOrEqual,
    LessThanOrEqual,
}

/// Error style displayed when validation fails.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum DataValidationErrorStyle {
    #[default]
    Stop,
    Warning,
    Information,
}

/// Configuration for adding data validation to a cell range.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataValidationConfig {
    /// The range to apply validation to (e.g. "A1:A10")
    pub range: String,
    /// Type of validation
    pub validation_type: DataValidationType,
    /// Comparison operator (required for Whole/Decimal/Date/Time/TextLength)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operator: Option<DataValidationOperator>,
    /// First formula/value (e.g. "10" for min, or "=A1" for formula reference)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub formula1: Option<String>,
    /// Second formula/value (e.g. "20" for max, required with Between/NotBetween)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub formula2: Option<String>,
    /// For List type: comma-separated list of allowed values or a range reference
    #[serde(skip_serializing_if = "Option::is_none")]
    pub list_values: Option<Vec<String>>,
    /// Allow blank cells to pass validation
    #[serde(default = "default_true")]
    pub allow_blank: bool,
    /// Show dropdown in cell (for List type)
    #[serde(default = "default_true")]
    pub show_dropdown: bool,
    /// Error dialog style
    #[serde(default)]
    pub error_style: DataValidationErrorStyle,
    /// Title of the error dialog
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_title: Option<String>,
    /// Message shown in the error dialog
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    /// Title of the input prompt
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_title: Option<String>,
    /// Message shown in the input prompt
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_message: Option<String>,
}

fn default_true() -> bool {
    true
}
