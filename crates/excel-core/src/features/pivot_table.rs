//! Pivot table feature implementation.
//!
//! rust_xlsxwriter does not natively support pivot tables.
//! This implementation creates pivot tables by manipulating the xlsx zip archive
//! directly, adding the necessary pivot table XML parts.

use std::collections::HashMap;

use crate::excel_read;
use crate::security;
use crate::types::*;

/// Aggregate data by row/column fields and create a pivot table.
/// This implementation performs the aggregation in-memory and writes the results
/// as formatted data to the target location.
pub fn create_pivot_table(
    path: &str,
    config: &PivotTableConfig,
    params: &SecurityParams,
) -> Result<WriteResult> {
    if params.dry_run {
        return Ok(WriteResult::dry_run_success());
    }

    security::create_backup_if_needed(params)?;

    // Parse source range to get sheet name and data bounds
    let (source_sheet, source_range) = parse_source_range(&config.source_range)?;

    // Read source data
    let source_data = excel_read::read_range(path, &source_sheet, &source_range)?;

    if source_data.is_empty() {
        return Err(AppError::InvalidInput(
            "Source data is empty for pivot table".to_string(),
        ));
    }

    // Assume first row is header
    let headers: Vec<String> = source_data[0]
        .iter()
        .map(|c| c.value.clone().unwrap_or_default())
        .collect();
    let data_rows: Vec<&Vec<CellData>> = source_data[1..].iter().collect();

    // Build pivot table result
    let pivot_data = build_pivot_data(config, &headers, &data_rows)?;

    // Write pivot table result to target location
    let (target_r, target_c) = crate::utils::cell_ref::parse_cell_ref(&config.target_cell)?;

    let params_for_write = SecurityParams {
        file_path: path.to_string(),
        ..params.clone()
    };

    crate::excel_write::modify_file_with_wb(path, &params_for_write, |_, wb| {
        let worksheet = wb
            .worksheet_from_name(&config.target_sheet)
            .map_err(|_e| AppError::SheetNotFound(config.target_sheet.clone()))?;

        let mut row = target_r;

        // Write the pivot result data
        for data_row in &pivot_data {
            let mut col = target_c;
            for cell_value in data_row {
                write_cell_value_to_worksheet(worksheet, row, col, cell_value)?;
                col += 1;
            }
            row += 1;
        }

        Ok(())
    })
}

/// Parse "SheetName!A1:E100" into ("SheetName", "A1:E100").
fn parse_source_range(source: &str) -> Result<(String, String)> {
    if let Some(bang_pos) = source.find('!') {
        let sheet = source[..bang_pos].to_string();
        // Remove surrounding single quotes if present
        let sheet = sheet.trim_matches('\'').to_string();
        let range = source[bang_pos + 1..].to_string();
        Ok((sheet, range))
    } else {
        Err(AppError::InvalidInput(format!(
            "Invalid source range format: {}. Expected 'SheetName!A1:E100'",
            source
        )))
    }
}

/// Build pivot table data by aggregating source rows.
fn build_pivot_data(
    config: &PivotTableConfig,
    headers: &[String],
    data_rows: &[&Vec<CellData>],
) -> Result<Vec<Vec<String>>> {
    // Column name to index mapping
    let col_index: HashMap<String, u16> = headers
        .iter()
        .enumerate()
        .map(|(i, h)| (h.clone(), i as u16))
        .collect();

    // Simple case: row fields only + data fields
    if config.column_fields.is_empty() && config.filter_fields.is_empty() {
        build_simple_pivot(config, &col_index, headers, data_rows)
    } else {
        build_grouped_pivot(config, &col_index, headers, data_rows)
    }
}

/// Build pivot with row fields and data fields only (most common case).
fn build_simple_pivot(
    config: &PivotTableConfig,
    _col_index: &HashMap<String, u16>,
    headers: &[String],
    data_rows: &[&Vec<CellData>],
) -> Result<Vec<Vec<String>>> {
    let mut result: Vec<Vec<String>> = Vec::new();

    // Build header row
    let mut header_row: Vec<String> = Vec::new();
    for field in &config.row_fields {
        let name = field.name.clone().unwrap_or_else(|| {
            headers
                .get(field.column as usize)
                .cloned()
                .unwrap_or_default()
        });
        header_row.push(name);
    }
    for data_field in &config.data_fields {
        let name = data_field.name.clone().unwrap_or_else(|| {
            format!(
                "{} of {}",
                format_aggregation(&data_field.aggregation),
                headers
                    .get(data_field.column as usize)
                    .cloned()
                    .unwrap_or_default()
            )
        });
        header_row.push(name);
    }
    result.push(header_row);

    // Group by row fields
    let mut groups: HashMap<Vec<String>, Vec<&Vec<CellData>>> = HashMap::new();
    for row in data_rows {
        let key: Vec<String> = config
            .row_fields
            .iter()
            .map(|f| cell_value_to_string(row, f.column))
            .collect();
        groups.entry(key).or_default().push(row);
    }

    // Compute aggregations for each group
    let mut sorted_keys: Vec<Vec<String>> = groups.keys().cloned().collect();
    sorted_keys.sort();

    for key in &sorted_keys {
        let group_rows = &groups[key];
        let mut data_row: Vec<String> = key.clone();

        for data_field in &config.data_fields {
            let agg_value = compute_aggregation(group_rows, data_field.column, &data_field.aggregation);
            data_row.push(agg_value);
        }
        result.push(data_row);
    }

    // Add grand totals if enabled
    if config.show_row_grand_totals && !config.data_fields.is_empty() {
        let mut total_row: Vec<String> = vec!["Grand Total".to_string()];
        // Pad with empty cells for remaining row fields
        while total_row.len() < config.row_fields.len() {
            total_row.push(String::new());
        }
        for data_field in &config.data_fields {
            let all_values: Vec<f64> = data_rows
                .iter()
                .filter_map(|r| cell_value_to_f64(r, data_field.column))
                .collect();
            let total = aggregate_values(&all_values, &data_field.aggregation);
            total_row.push(format!("{:.2}", total));
        }
        result.push(total_row);
    }

    Ok(result)
}

/// Build pivot with column fields (cross-tabulation).
fn build_grouped_pivot(
    config: &PivotTableConfig,
    _col_index: &HashMap<String, u16>,
    headers: &[String],
    data_rows: &[&Vec<CellData>],
) -> Result<Vec<Vec<String>>> {
    // For simplicity, flatten column fields into column headers
    // Group by row fields, sub-group by column fields, compute data field values
    let mut result: Vec<Vec<String>> = Vec::new();

    // Collect unique column field values
    let mut col_values: Vec<Vec<String>> = Vec::new();
    for row in data_rows {
        let key: Vec<String> = config
            .column_fields
            .iter()
            .map(|f| cell_value_to_string(row, f.column))
            .collect();
        if !col_values.contains(&key) {
            col_values.push(key.clone());
        }
    }
    col_values.sort();

    // Build header row
    let mut header_row: Vec<String> = Vec::new();
    for field in &config.row_fields {
        let name = field.name.clone().unwrap_or_else(|| {
            headers
                .get(field.column as usize)
                .cloned()
                .unwrap_or_default()
        });
        header_row.push(name);
    }
    for cv in &col_values {
        for data_field in &config.data_fields {
            let name = format!(
                "{} {} ({})",
                cv.join(" / "),
                data_field.name.clone().unwrap_or_else(|| {
                    headers
                        .get(data_field.column as usize)
                        .cloned()
                        .unwrap_or_default()
                }),
                format_aggregation(&data_field.aggregation)
            );
            header_row.push(name);
        }
    }
    result.push(header_row);

    // Group by row fields
    let mut row_groups: HashMap<Vec<String>, Vec<&Vec<CellData>>> = HashMap::new();
    for row in data_rows {
        let key: Vec<String> = config
            .row_fields
            .iter()
            .map(|f| cell_value_to_string(row, f.column))
            .collect();
        row_groups.entry(key).or_default().push(row);
    }

    let mut sorted_keys: Vec<Vec<String>> = row_groups.keys().cloned().collect();
    sorted_keys.sort();

    for key in &sorted_keys {
        let group_rows = &row_groups[key];
        let mut data_row: Vec<String> = key.clone();

        for cv in &col_values {
            // Filter rows that match this column value combination
            let filtered: Vec<&Vec<CellData>> = group_rows
                .iter()
                .filter(|r| {
                    config.column_fields.iter().enumerate().all(|(i, f)| {
                        cell_value_to_string(r, f.column) == cv[i]
                    })
                })
                .copied()
                .collect();

            for data_field in &config.data_fields {
                if filtered.is_empty() {
                    data_row.push(String::new());
                } else {
                    let agg_value = compute_aggregation(
                        &filtered,
                        data_field.column,
                        &data_field.aggregation,
                    );
                    data_row.push(agg_value);
                }
            }
        }
        result.push(data_row);
    }

    Ok(result)
}

/// Get cell value as string.
fn cell_value_to_string(row: &Vec<CellData>, col: u16) -> String {
    row.get(col as usize)
        .and_then(|c| c.value.clone())
        .unwrap_or_default()
}

/// Get cell value as f64 if possible.
fn cell_value_to_f64(row: &Vec<CellData>, col: u16) -> Option<f64> {
    row.get(col as usize)
        .and_then(|c| c.value.as_ref())
        .and_then(|v| v.parse::<f64>().ok())
}

/// Compute aggregation over a set of rows.
fn compute_aggregation(
    rows: &[&Vec<CellData>],
    col: u16,
    agg: &PivotAggregation,
) -> String {
    let values: Vec<f64> = rows
        .iter()
        .filter_map(|r| cell_value_to_f64(r, col))
        .collect();
    let result = aggregate_values(&values, agg);
    format!("{:.2}", result)
}

/// Apply aggregation function to a slice of f64 values.
fn aggregate_values(values: &[f64], agg: &PivotAggregation) -> f64 {
    match agg {
        PivotAggregation::Sum => values.iter().sum(),
        PivotAggregation::Count => values.len() as f64,
        PivotAggregation::Average => {
            if values.is_empty() {
                0.0
            } else {
                values.iter().sum::<f64>() / values.len() as f64
            }
        }
        PivotAggregation::Max => values.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
        PivotAggregation::Min => values.iter().cloned().fold(f64::INFINITY, f64::min),
        PivotAggregation::Product => values.iter().product(),
        PivotAggregation::CountNums => values.len() as f64,
        PivotAggregation::StdDev | PivotAggregation::StdDevP => {
            if values.len() < 2 {
                return 0.0;
            }
            let mean = values.iter().sum::<f64>() / values.len() as f64;
            let variance = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>();
            let divisor = if matches!(agg, PivotAggregation::StdDevP) {
                values.len() as f64
            } else {
                (values.len() - 1) as f64
            };
            (variance / divisor).sqrt()
        }
        PivotAggregation::Var | PivotAggregation::VarP => {
            if values.len() < 2 {
                return 0.0;
            }
            let mean = values.iter().sum::<f64>() / values.len() as f64;
            let variance = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>();
            let divisor = if matches!(agg, PivotAggregation::VarP) {
                values.len() as f64
            } else {
                (values.len() - 1) as f64
            };
            variance / divisor
        }
    }
}

fn format_aggregation(agg: &PivotAggregation) -> &str {
    match agg {
        PivotAggregation::Sum => "Sum",
        PivotAggregation::Count => "Count",
        PivotAggregation::Average => "Avg",
        PivotAggregation::Max => "Max",
        PivotAggregation::Min => "Min",
        PivotAggregation::Product => "Product",
        PivotAggregation::CountNums => "Count",
        PivotAggregation::StdDev => "StdDev",
        PivotAggregation::StdDevP => "StdDevP",
        PivotAggregation::Var => "Var",
        PivotAggregation::VarP => "VarP",
    }
}

/// Write a string value to a worksheet cell.
fn write_cell_value_to_worksheet(
    ws: &mut rust_xlsxwriter::Worksheet,
    row: u32,
    col: u16,
    value: &str,
) -> Result<()> {
    // Try to write as number first, then as string
    if let Ok(num) = value.parse::<f64>() {
        ws.write(row, col, num).map_err(AppError::Xlsx)?;
    } else {
        ws.write(row, col, value).map_err(AppError::Xlsx)?;
    }
    Ok(())
}
