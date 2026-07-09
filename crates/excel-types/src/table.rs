use serde::{Deserialize, Serialize};

/// Predefined Excel table styles.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TableStylePreset {
    None,
    Light1,
    Light2,
    Light3,
    Light4,
    Light5,
    Light6,
    Light7,
    Light8,
    Light9,
    Light10,
    Light11,
    Light12,
    Light13,
    Light14,
    Light15,
    Light16,
    Light17,
    Light18,
    Light19,
    Light20,
    Light21,
    Medium1,
    Medium2,
    Medium3,
    Medium4,
    Medium5,
    Medium6,
    Medium7,
    Medium8,
    Medium9,
    Medium10,
    Medium11,
    Medium12,
    Medium13,
    Medium14,
    Medium15,
    Medium16,
    Medium17,
    Medium18,
    Medium19,
    Medium20,
    Medium21,
    Medium22,
    Medium23,
    Medium24,
    Medium25,
    Medium26,
    Medium27,
    Medium28,
    Dark1,
    Dark2,
    Dark3,
    Dark4,
    Dark5,
    Dark6,
    Dark7,
    Dark8,
    Dark9,
    Dark10,
    Dark11,
}

impl Default for TableStylePreset {
    fn default() -> Self {
        TableStylePreset::Medium2
    }
}

/// Description of an existing table in a worksheet.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableInfo {
    /// Table name
    pub name: String,
    /// Sheet the table belongs to
    pub sheet: String,
    /// Cell range of the table (e.g. "A1:D10")
    pub range: String,
    /// Whether the table has a header row
    pub has_header: bool,
    /// Whether the table has a total row
    pub has_total: bool,
}

/// Configuration for creating an Excel table (ListObject).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableConfig {
    /// Table name (must be unique within the workbook)
    pub name: String,
    /// Target sheet name (if not present, parsed from range or uses first sheet)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sheet: Option<String>,
    /// Cell range for the table (e.g. "A1:D10" or "Sheet1!A1:D10")
    pub range: String,
    /// Whether the first row contains headers
    #[serde(default = "default_true")]
    pub has_header: bool,
    /// Whether to show a total row
    #[serde(default)]
    pub has_total: bool,
    /// Table style preset
    #[serde(default)]
    pub style: TableStylePreset,
    /// Column formulas for the total row (keyed by 0-based column index)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_row_functions: Option<Vec<TotalColumnFunction>>,
}

fn default_true() -> bool {
    true
}

/// Total row function for a specific column.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TotalColumnFunction {
    /// 0-based column index
    pub column: u16,
    /// Excel function for the total row (sum, average, count, max, min, etc.)
    pub function: TotalFunction,
}

/// Functions available for table total rows.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TotalFunction {
    Sum,
    Average,
    Count,
    CountNums,
    Max,
    Min,
    StdDev,
    Var,
    Custom(String),
}
