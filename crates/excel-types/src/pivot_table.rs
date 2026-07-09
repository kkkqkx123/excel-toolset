use serde::{Deserialize, Serialize};

/// Aggregation function for pivot table data fields.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PivotAggregation {
    Sum,
    Count,
    Average,
    Max,
    Min,
    Product,
    CountNums,
    StdDev,
    StdDevP,
    Var,
    VarP,
}

impl Default for PivotAggregation {
    fn default() -> Self {
        PivotAggregation::Sum
    }
}

/// A pivot table field configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PivotField {
    /// Column index (0-based) in the source data
    pub column: u16,
    /// Display name for the field (defaults to column header)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// Configuration for creating a pivot table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PivotTableConfig {
    /// Name for the pivot table
    pub name: String,
    /// Source data range (e.g. "Sheet1!A1:E100")
    pub source_range: String,
    /// Target sheet to place the pivot table
    pub target_sheet: String,
    /// Target cell to place the top-left of the pivot table (e.g. "A1")
    pub target_cell: String,
    /// Fields placed in the row area
    #[serde(default)]
    pub row_fields: Vec<PivotField>,
    /// Fields placed in the column area
    #[serde(default)]
    pub column_fields: Vec<PivotField>,
    /// Fields placed in the data/values area
    #[serde(default)]
    pub data_fields: Vec<PivotDataField>,
    /// Fields placed in the filter/page area
    #[serde(default)]
    pub filter_fields: Vec<PivotField>,
    /// Show grand totals for rows
    #[serde(default = "default_true")]
    pub show_row_grand_totals: bool,
    /// Show grand totals for columns
    #[serde(default = "default_true")]
    pub show_column_grand_totals: bool,
    /// Layout style
    #[serde(default)]
    pub layout: PivotLayout,
}

fn default_true() -> bool {
    true
}

/// A data field in a pivot table (with aggregation).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PivotDataField {
    /// Column index (0-based) in the source data
    pub column: u16,
    /// Display name for the data field
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Aggregation function
    #[serde(default)]
    pub aggregation: PivotAggregation,
    /// Show values as (percentage of total, difference from, etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub show_as: Option<PivotShowAs>,
}

/// How to display values in a pivot table data field.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PivotShowAs {
    Normal,
    PercentOfGrandTotal,
    PercentOfRowTotal,
    PercentOfColumnTotal,
    PercentOf,
    DifferenceFrom,
    RunningTotal,
    Rank,
    Index,
}

/// Pivot table layout style.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PivotLayout {
    /// Compact form (default Excel style)
    Compact,
    /// Outline form
    Outline,
    /// Tabular form
    Tabular,
}

impl Default for PivotLayout {
    fn default() -> Self {
        PivotLayout::Compact
    }
}
