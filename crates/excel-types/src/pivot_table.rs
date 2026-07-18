use serde::{Deserialize, Serialize};

/// Aggregation function for pivot table data fields.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum PivotAggregation {
    #[default]
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
    /// Custom label for grand total row (default: "Grand Total")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grand_total_caption: Option<String>,
    /// Subtotals configuration
    #[serde(default)]
    pub subtotals: PivotSubtotals,
    /// Sort configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort: Option<PivotSort>,
    /// Whether to repeat row labels (fill down)
    #[serde(default)]
    pub repeat_labels: bool,
    /// Grouping configuration for date fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date_grouping: Option<DateGrouping>,
    /// Calculated fields defined from existing data fields
    #[serde(default)]
    pub calculated_fields: Vec<PivotCalculatedField>,
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
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum PivotLayout {
    /// Compact form (default Excel style): row fields nested in one column
    #[default]
    Compact,
    /// Outline form: each row field in its own column, children below parents
    Outline,
    /// Tabular form: like compact but field names in header row
    Tabular,
}

/// Subtotals configuration for pivot table.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum PivotSubtotals {
    /// Show subtotals at each group level
    On,
    /// Hide all subtotals
    #[default]
    Off,
    /// Show subtotals only at the top
    Top,
}

/// Sort configuration for pivot table fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PivotSort {
    /// Sort by label or value
    #[serde(default)]
    pub sort_by: PivotSortBy,
    /// Sort order
    #[serde(default)]
    pub order: PivotSortOrder,
    /// 0-based index of the data field to sort by (only when sort_by=Value)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_field_index: Option<usize>,
}

/// What to sort by in a pivot table.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum PivotSortBy {
    #[default]
    Label,
    Value,
}

/// Sort order for pivot table.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum PivotSortOrder {
    #[default]
    Ascending,
    Descending,
}

/// Date grouping configuration for pivot table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DateGrouping {
    /// 0-based column index of the date field
    pub column: u16,
    /// Group by year
    #[serde(default)]
    pub by_year: bool,
    /// Group by quarter
    #[serde(default)]
    pub by_quarter: bool,
    /// Group by month
    #[serde(default)]
    pub by_month: bool,
    /// Group by day
    #[serde(default)]
    pub by_day: bool,
}

/// A calculated field in a pivot table.
///
/// Allows defining a virtual field based on an arithmetic expression
/// involving existing data columns. The expression supports the
/// four basic arithmetic operators (+, -, *, /) and parentheses.
/// Field names in the expression are matched against column headers.
///
/// Example formulas:
///   "=Revenue - Cost"
///   "=Price * Quantity"
///   "=(Income - Expense) * TaxRate"
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PivotCalculatedField {
    /// Display name for the calculated field
    pub name: String,
    /// Formula expression using existing field names
    /// Supports +, -, *, / and parentheses
    pub formula: String,
}
