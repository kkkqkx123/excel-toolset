//! Pivot table feature implementation.
//!
//! rust_xlsxwriter does not natively support pivot tables.
//! This implementation creates pivot tables by reading source data,
//! performing in-memory aggregation, and writing results as formatted data.

use std::collections::HashMap;

use crate::excel_read;
use crate::security;
use crate::types::*;

/// Aggregate data by row/column fields and create a pivot table.
pub fn create_pivot_table(
    path: &str,
    config: &PivotTableConfig,
    params: &SecurityParams,
) -> Result<WriteResult> {
    if params.dry_run {
        return Ok(WriteResult::dry_run_success());
    }

    security::create_backup_if_needed(params)?;

    let (source_sheet, source_range) = parse_source_range(&config.source_range)?;

    let source_data = excel_read::read_range(path, &source_sheet, &source_range)?;

    if source_data.is_empty() {
        return Err(AppError::InvalidInput(
            "Source data is empty for pivot table".to_string(),
        ));
    }

    let headers: Vec<String> = source_data[0]
        .iter()
        .map(|c| c.value.clone().unwrap_or_default())
        .collect();
    let data_rows: Vec<&Vec<CellData>> = source_data[1..].iter().collect();

    // Apply date grouping if configured
    let (mut adjusted_headers, mut adjusted_rows) =
        apply_date_grouping(config, &headers, &data_rows);

    // Process calculated fields: evaluate formulas and append as new columns
    let calc_field_columns =
        process_calculated_fields(config, &mut adjusted_headers, &mut adjusted_rows)?;

    let pivot_data = build_pivot_data(
        config,
        &adjusted_headers,
        &adjusted_rows,
        &calc_field_columns,
    )?;

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

/// Apply date grouping to source data.
/// Returns adjusted headers and new data rows with grouped date values.
fn apply_date_grouping(
    config: &PivotTableConfig,
    headers: &[String],
    data_rows: &[&Vec<CellData>],
) -> (Vec<String>, Vec<Vec<CellData>>) {
    let grouping = match &config.date_grouping {
        Some(g) => g,
        None => {
            // No date grouping, return original data
            let cloned: Vec<Vec<CellData>> = data_rows.iter().map(|r| (*r).clone()).collect();
            return (headers.to_vec(), cloned);
        }
    };

    let new_headers = headers.to_vec();
    let col = grouping.column as usize;
    if col >= new_headers.len() {
        return (
            headers.to_vec(),
            data_rows.iter().map(|r| (*r).clone()).collect(),
        );
    }

    let mut new_rows: Vec<Vec<CellData>> = Vec::new();

    for row in data_rows {
        let mut new_row = (*row).clone();
        let date_val = row
            .get(col)
            .and_then(|c| c.value.clone())
            .unwrap_or_default();

        // Try to parse as date and group
        if let Some(grouped) = group_date_value(&date_val, grouping) {
            new_row[col] = CellData {
                value: Some(grouped),
                data_type: CellDataType::String,
                formula: None,
            };
        }

        new_rows.push(new_row);
    }

    (new_headers, new_rows)
}

/// Group a date value string by year/quarter/month/day.
fn group_date_value(value: &str, grouping: &DateGrouping) -> Option<String> {
    // Parse YYYY-MM-DD or YYYY/MM/DD
    let parts: Vec<&str> = value.split(|c: char| c == '-' || c == '/').collect();

    if parts.len() < 3 {
        return None;
    }

    let year: i32 = parts[0].parse().ok()?;
    let month: u32 = parts[1].parse().ok()?;
    let day: u32 = parts[2].parse().ok()?;

    if month == 0 || month > 12 || day == 0 || day > 31 {
        return None;
    }

    let mut result_parts: Vec<String> = Vec::new();

    if grouping.by_year {
        result_parts.push(format!("{}", year));
    }
    if grouping.by_quarter {
        let quarter = (month + 2) / 3;
        result_parts.push(format!("Q{}", quarter));
    }
    if grouping.by_month {
        result_parts.push(format!("{:02}", month));
    }
    if grouping.by_day {
        result_parts.push(format!("{:02}", day));
    }

    if result_parts.is_empty() {
        None
    } else {
        Some(result_parts.join("-"))
    }
}

/// Parse "SheetName!A1:E100" into ("SheetName", "A1:E100").
fn parse_source_range(source: &str) -> Result<(String, String)> {
    if let Some(bang_pos) = source.find('!') {
        let sheet = source[..bang_pos].trim_matches('\'').to_string();
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
    data_rows: &[Vec<CellData>],
    calc_field_columns: &[(String, u16)],
) -> Result<Vec<Vec<String>>> {
    let col_index: HashMap<String, u16> = headers
        .iter()
        .enumerate()
        .map(|(i, h)| (h.clone(), i as u16))
        .collect();

    // Build extended data fields: original + auto-generated from calculated fields
    let mut all_data_fields: Vec<PivotDataField> = config.data_fields.clone();
    for (name, col) in calc_field_columns {
        all_data_fields.push(PivotDataField {
            column: *col,
            name: Some(name.clone()),
            aggregation: PivotAggregation::Sum,
            show_as: None,
        });
    }

    if config.column_fields.is_empty() && config.filter_fields.is_empty() {
        build_simple_pivot(config, &col_index, headers, data_rows, &all_data_fields)
    } else {
        build_grouped_pivot(config, &col_index, headers, data_rows, &all_data_fields)
    }
}

/// Build pivot with row fields and data fields only.
fn build_simple_pivot(
    config: &PivotTableConfig,
    _col_index: &HashMap<String, u16>,
    headers: &[String],
    data_rows: &[Vec<CellData>],
    data_fields: &[PivotDataField],
) -> Result<Vec<Vec<String>>> {
    let mut result: Vec<Vec<String>> = Vec::new();

    // Determine layout
    let uses_compact = matches!(config.layout, PivotLayout::Compact);
    let _uses_outline = matches!(config.layout, PivotLayout::Outline);
    let _uses_tabular = matches!(config.layout, PivotLayout::Tabular);

    // Build header row based on layout
    let mut header_row: Vec<String> = Vec::new();
    if uses_compact {
        // Compact: single column for all row fields
        header_row.push("Row Labels".to_string());
    } else {
        // Outline/Tabular: one column per row field
        for field in &config.row_fields {
            let name = field.name.clone().unwrap_or_else(|| {
                headers
                    .get(field.column as usize)
                    .cloned()
                    .unwrap_or_default()
            });
            header_row.push(name);
        }
    }

    // Column grand totals header
    if config.show_column_grand_totals && data_fields.len() > 1 {
        for data_field in data_fields {
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
    } else if data_fields.len() == 1 {
        // Single data field: just use the field name
        let data_field = &data_fields[0];
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
    } else {
        // Multiple data fields without column grand totals: list each
        for data_field in data_fields {
            let name = data_field.name.clone().unwrap_or_else(|| {
                headers
                    .get(data_field.column as usize)
                    .cloned()
                    .unwrap_or_default()
            });
            header_row.push(name);
        }
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

    // Sort groups based on configuration
    let mut sorted_keys: Vec<Vec<String>> = groups.keys().cloned().collect();
    sort_keys(&mut sorted_keys, config, data_rows);

    // Compute grand total for show_as calculations
    let grand_totals: Vec<f64> = if needs_grand_total_for_show_as(data_fields) {
        compute_grand_totals(data_fields, data_rows)
    } else {
        Vec::new()
    };

    // Compute row totals for PercentOfRowTotal
    let row_totals: HashMap<Vec<String>, f64> = if data_fields
        .iter()
        .any(|df| df.show_as == Some(PivotShowAs::PercentOfRowTotal))
    {
        compute_row_totals(data_fields, &groups)
    } else {
        HashMap::new()
    };

    // Running total accumulator
    let mut running_totals: Vec<f64> = vec![0.0; data_fields.len()];

    for (key_idx, key) in sorted_keys.iter().enumerate() {
        let group_rows = &groups[key];
        let mut data_row: Vec<String> = Vec::new();

        // Row label rendering based on layout and repeat_labels
        if uses_compact {
            // Compact layout: all row field values joined
            if config.repeat_labels && key_idx > 0 {
                // Fill down: repeat parent values
                let prev_key = &sorted_keys[key_idx - 1];
                let mut label = String::new();
                for (i, k) in key.iter().enumerate() {
                    if i == key.len() - 1 || prev_key.get(i) != Some(k) {
                        if !label.is_empty() {
                            label.push_str(" / ");
                        }
                        label.push_str(k);
                    }
                }
                data_row.push(if label.is_empty() {
                    String::new()
                } else {
                    label
                });
            } else {
                data_row.push(key.join(" / "));
            }
        } else {
            // Outline/Tabular layout: one column per row field
            for (i, k) in key.iter().enumerate() {
                if config.repeat_labels {
                    data_row.push(k.clone());
                } else if i == 0 || key[i - 1] != key[i] {
                    data_row.push(k.clone());
                } else {
                    // Check if parent changed
                    let prev_key = if key_idx > 0 {
                        &sorted_keys[key_idx - 1]
                    } else {
                        key
                    };
                    if i < prev_key.len() && prev_key[i] != key[i] {
                        data_row.push(k.clone());
                    } else if i == prev_key.len() {
                        data_row.push(k.clone());
                    } else {
                        data_row.push(String::new());
                    }
                }
            }
        }

        // Compute and write data field values
        for (df_idx, data_field) in data_fields.iter().enumerate() {
            let raw_value =
                compute_aggregation(group_rows, data_field.column, &data_field.aggregation);
            let raw_f64: f64 = raw_value.parse().unwrap_or(0.0);

            let display_value = apply_show_as(
                raw_f64,
                &data_field.show_as,
                df_idx,
                &grand_totals,
                row_totals.get(key).copied(),
                &mut running_totals,
                key_idx + 1,
                sorted_keys.len(),
            );

            data_row.push(display_value);
        }

        result.push(data_row);
    }

    // Add subtotals if enabled
    if config.subtotals == PivotSubtotals::On && config.row_fields.len() > 1 {
        result = insert_subtotals(result, config, data_fields, data_rows);
    }

    // Add grand totals if enabled
    if config.show_row_grand_totals && !data_fields.is_empty() {
        let caption = config
            .grand_total_caption
            .clone()
            .unwrap_or_else(|| "Grand Total".to_string());

        let mut total_row: Vec<String> = Vec::new();

        if uses_compact {
            total_row.push(caption);
        } else {
            total_row.push(caption);
            while total_row.len() < config.row_fields.len() {
                total_row.push(String::new());
            }
        }

        for (df_idx, data_field) in data_fields.iter().enumerate() {
            let all_values: Vec<f64> = data_rows
                .iter()
                .filter_map(|r| cell_value_to_f64(r, data_field.column))
                .collect();
            let total = aggregate_values(&all_values, &data_field.aggregation);

            let display_value = apply_show_as_on_grand_total(
                total,
                &data_field.show_as,
                df_idx,
                &grand_totals,
                data_rows.len(),
            );

            total_row.push(display_value);
        }
        result.push(total_row);
    }

    Ok(result)
}

/// Insert subtotal rows into pivot data.
fn insert_subtotals(
    data: Vec<Vec<String>>,
    config: &PivotTableConfig,
    _data_fields: &[PivotDataField],
    _data_rows: &[Vec<CellData>],
) -> Vec<Vec<String>> {
    if data.is_empty() || config.row_fields.is_empty() {
        return data;
    }

    let uses_compact = matches!(config.layout, PivotLayout::Compact);
    let mut result: Vec<Vec<String>> = Vec::new();

    // The header row
    if !data.is_empty() {
        result.push(data[0].clone());
    }

    let mut prev_group_key: Vec<String> = Vec::new();

    for row in data.iter().skip(1) {
        // Check if we should insert a subtotal (when the first row field changes)
        let current_key: Vec<String> = if uses_compact {
            row.first().map(|s| vec![s.clone()]).unwrap_or_default()
        } else {
            row.iter().take(config.row_fields.len()).cloned().collect()
        };

        if !prev_group_key.is_empty() && current_key.first() != prev_group_key.first() {
            // Insert a subtotal for the previous group
            let mut subtotal_row: Vec<String> = Vec::new();
            if uses_compact {
                subtotal_row.push(format!(
                    "{} Total",
                    prev_group_key.first().unwrap_or(&String::new())
                ));
            } else {
                subtotal_row.push(format!(
                    "{} Total",
                    prev_group_key.first().unwrap_or(&String::new())
                ));
                while subtotal_row.len() < config.row_fields.len() {
                    subtotal_row.push(String::new());
                }
            }

            // Compute subtotal values
            let subtotal_vals = compute_subtotal_for_group(
                config,
                prev_group_key.first().cloned().unwrap_or_default(),
            );
            for val in &subtotal_vals {
                subtotal_row.push(val.clone());
            }
            result.push(subtotal_row);
        }

        result.push(row.clone());
        prev_group_key = current_key;
    }

    result
}

/// Compute subtotal values for a group (simplified: returns zeros placeholder).
fn compute_subtotal_for_group(_config: &PivotTableConfig, _group_key: String) -> Vec<String> {
    // In a full implementation, this would aggregate the actual group data
    // For now, return placeholder
    Vec::new()
}

/// Check if any data field needs grand total for show_as calculation.
fn needs_grand_total_for_show_as(data_fields: &[PivotDataField]) -> bool {
    data_fields.iter().any(|df| {
        matches!(
            df.show_as,
            Some(PivotShowAs::PercentOfGrandTotal)
                | Some(PivotShowAs::PercentOfColumnTotal)
                | Some(PivotShowAs::Index)
                | Some(PivotShowAs::Rank)
        )
    })
}

/// Compute grand totals for each data field.
fn compute_grand_totals(data_fields: &[PivotDataField], data_rows: &[Vec<CellData>]) -> Vec<f64> {
    data_fields
        .iter()
        .map(|df| {
            let values: Vec<f64> = data_rows
                .iter()
                .filter_map(|r| cell_value_to_f64(r, df.column))
                .collect();
            aggregate_values(&values, &df.aggregation)
        })
        .collect()
}

/// Compute row totals per group for PercentOfRowTotal.
fn compute_row_totals(
    data_fields: &[PivotDataField],
    groups: &HashMap<Vec<String>, Vec<&Vec<CellData>>>,
) -> HashMap<Vec<String>, f64> {
    let mut totals = HashMap::new();
    for (key, rows) in groups {
        let total: f64 = data_fields
            .iter()
            .map(|df| {
                let values: Vec<f64> = rows
                    .iter()
                    .filter_map(|r| cell_value_to_f64(r, df.column))
                    .collect();
                aggregate_values(&values, &df.aggregation)
            })
            .sum();
        totals.insert(key.clone(), total);
    }
    totals
}

/// Apply show_as transformation to a value.
fn apply_show_as(
    value: f64,
    show_as: &Option<PivotShowAs>,
    df_index: usize,
    grand_totals: &[f64],
    row_total: Option<f64>,
    running_total: &mut [f64],
    rank: usize,
    _total_count: usize,
) -> String {
    match show_as {
        None | Some(PivotShowAs::Normal) => format!("{:.2}", value),
        Some(PivotShowAs::PercentOfGrandTotal) => {
            let gt = grand_totals.get(df_index).copied().unwrap_or(1.0);
            if gt == 0.0 {
                format!("{:.2}%", 0.0)
            } else {
                format!("{:.2}%", (value / gt) * 100.0)
            }
        }
        Some(PivotShowAs::PercentOfRowTotal) => {
            let rt = row_total.unwrap_or(1.0);
            if rt == 0.0 {
                format!("{:.2}%", 0.0)
            } else {
                format!("{:.2}%", (value / rt) * 100.0)
            }
        }
        Some(PivotShowAs::PercentOfColumnTotal) => {
            let gt = grand_totals.get(df_index).copied().unwrap_or(1.0);
            if gt == 0.0 {
                format!("{:.2}%", 0.0)
            } else {
                format!("{:.2}%", (value / gt) * 100.0)
            }
        }
        Some(PivotShowAs::RunningTotal) => {
            running_total[df_index] += value;
            format!("{:.2}", running_total[df_index])
        }
        Some(PivotShowAs::Rank) => {
            format!("{}", rank)
        }
        Some(PivotShowAs::Index) => {
            // Index = ((value in cell) x (Grand Total of Grand Totals)) /
            //         ((Grand Row Total) x (Grand Column Total))
            let gt = grand_totals.get(df_index).copied().unwrap_or(1.0);
            if gt == 0.0 {
                format!("{:.2}", 0.0)
            } else {
                let overall = grand_totals.iter().sum::<f64>();
                let index_val = (value * overall) / (gt * gt);
                format!("{:.2}", index_val)
            }
        }
        Some(PivotShowAs::PercentOf) | Some(PivotShowAs::DifferenceFrom) => {
            // These require a base field/item reference, not fully implemented
            format!("{:.2}", value)
        }
    }
}

/// Apply show_as on grand total value.
fn apply_show_as_on_grand_total(
    value: f64,
    show_as: &Option<PivotShowAs>,
    _df_index: usize,
    _grand_totals: &[f64],
    _count: usize,
) -> String {
    match show_as {
        Some(PivotShowAs::RunningTotal) => format!("{:.2}", value),
        Some(PivotShowAs::PercentOfGrandTotal) => "100.00%".to_string(),
        _ => format!("{:.2}", value),
    }
}

/// Build pivot with column fields (cross-tabulation).
fn build_grouped_pivot(
    config: &PivotTableConfig,
    _col_index: &HashMap<String, u16>,
    headers: &[String],
    data_rows: &[Vec<CellData>],
    data_fields: &[PivotDataField],
) -> Result<Vec<Vec<String>>> {
    let mut result: Vec<Vec<String>> = Vec::new();

    let uses_compact = matches!(config.layout, PivotLayout::Compact);

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
    if uses_compact {
        header_row.push("Row Labels".to_string());
    } else {
        for field in &config.row_fields {
            let name = field.name.clone().unwrap_or_else(|| {
                headers
                    .get(field.column as usize)
                    .cloned()
                    .unwrap_or_default()
            });
            header_row.push(name);
        }
    }
    for cv in &col_values {
        for data_field in data_fields {
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
    sort_keys(&mut sorted_keys, config, data_rows);

    // Compute grand totals for show_as
    let grand_totals = compute_grand_totals(data_fields, data_rows);

    let mut running_totals: Vec<f64> = vec![0.0; data_fields.len() * col_values.len()];

    for (key_idx, key) in sorted_keys.iter().enumerate() {
        let group_rows = &row_groups[key];
        let mut data_row: Vec<String> = Vec::new();

        if uses_compact {
            data_row.push(key.join(" / "));
        } else {
            for k in key.iter() {
                data_row.push(k.clone());
            }
        }

        for (cv_idx, cv) in col_values.iter().enumerate() {
            let filtered: Vec<&Vec<CellData>> = group_rows
                .iter()
                .filter(|r| {
                    config
                        .column_fields
                        .iter()
                        .enumerate()
                        .all(|(i, f)| cell_value_to_string(r, f.column) == cv[i])
                })
                .copied()
                .collect();

            for (df_idx, data_field) in data_fields.iter().enumerate() {
                let agg_value = if filtered.is_empty() {
                    String::new()
                } else {
                    let raw_value =
                        compute_aggregation(&filtered, data_field.column, &data_field.aggregation);
                    let raw_f64: f64 = raw_value.parse().unwrap_or(0.0);
                    let _rt_idx = cv_idx * data_fields.len() + df_idx;
                    apply_show_as(
                        raw_f64,
                        &data_field.show_as,
                        df_idx,
                        &grand_totals,
                        None,
                        &mut running_totals,
                        key_idx + 1,
                        sorted_keys.len(),
                    )
                };
                data_row.push(agg_value);
            }
        }
        result.push(data_row);
    }

    // Add grand totals if enabled
    if config.show_row_grand_totals && !data_fields.is_empty() {
        let caption = config
            .grand_total_caption
            .clone()
            .unwrap_or_else(|| "Grand Total".to_string());
        let mut total_row: Vec<String> = vec![caption];
        while total_row.len()
            < (if uses_compact {
                1
            } else {
                config.row_fields.len()
            })
        {
            total_row.push(String::new());
        }
        for cv in &col_values {
            for data_field in data_fields {
                let all_vals: Vec<f64> = data_rows
                    .iter()
                    .filter(|r| {
                        config
                            .column_fields
                            .iter()
                            .enumerate()
                            .all(|(i, f)| cell_value_to_string(r, f.column) == cv[i])
                    })
                    .filter_map(|r| cell_value_to_f64(r, data_field.column))
                    .collect();
                let total = aggregate_values(&all_vals, &data_field.aggregation);
                total_row.push(format!("{:.2}", total));
            }
        }
        result.push(total_row);
    }

    Ok(result)
}

/// Sort key vectors based on pivot sort configuration.
fn sort_keys(keys: &mut [Vec<String>], config: &PivotTableConfig, _data_rows: &[Vec<CellData>]) {
    match &config.sort {
        Some(sort_config) => match sort_config.order {
            PivotSortOrder::Ascending => {
                keys.sort();
            }
            PivotSortOrder::Descending => {
                keys.sort_by(|a, b| b.cmp(a));
            }
        },
        None => {
            keys.sort();
        }
    }
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
fn compute_aggregation(rows: &[&Vec<CellData>], col: u16, agg: &PivotAggregation) -> String {
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

// ── Calculated Field Expression Parser ──

/// Token types for the calculated field expression parser.
#[derive(Debug, Clone, PartialEq)]
enum Token {
    Number(f64),
    Identifier(String),
    Plus,
    Minus,
    Star,
    Slash,
    LParen,
    RParen,
}

/// AST node for a calculated field expression.
#[derive(Debug, Clone)]
enum Expr {
    Number(f64),
    Field(String),
    Binary(Box<Expr>, BinOp, Box<Expr>),
}

#[derive(Debug, Clone)]
enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
}

/// Tokenize a formula string like "=Revenue - Cost" or "Price * (Qty + 1)".
fn tokenize(formula: &str) -> Result<Vec<Token>> {
    let s = formula.trim();
    let s = s.strip_prefix('=').unwrap_or(s).trim();
    let mut tokens: Vec<Token> = Vec::new();
    let chars: Vec<char> = s.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        match c {
            '+' => {
                tokens.push(Token::Plus);
                i += 1;
            }
            '-' => {
                tokens.push(Token::Minus);
                i += 1;
            }
            '*' => {
                tokens.push(Token::Star);
                i += 1;
            }
            '/' => {
                tokens.push(Token::Slash);
                i += 1;
            }
            '(' => {
                tokens.push(Token::LParen);
                i += 1;
            }
            ')' => {
                tokens.push(Token::RParen);
                i += 1;
            }
            _ if c.is_ascii_digit()
                || (c == '.' && i + 1 < len && chars[i + 1].is_ascii_digit()) =>
            {
                let start = i;
                while i < len && (chars[i].is_ascii_digit() || chars[i] == '.') {
                    i += 1;
                }
                let num_str: String = chars[start..i].iter().collect();
                let num = num_str
                    .parse::<f64>()
                    .map_err(|_| AppError::InvalidInput(format!("Invalid number: {}", num_str)))?;
                tokens.push(Token::Number(num));
            }
            _ if c.is_alphabetic() || c == '_' => {
                let start = i;
                while i < len && (chars[i].is_alphanumeric() || chars[i] == '_' || chars[i] == '.')
                {
                    i += 1;
                }
                let ident: String = chars[start..i].iter().collect();
                tokens.push(Token::Identifier(ident));
            }
            _ => {
                return Err(AppError::InvalidInput(format!(
                    "Unexpected character '{}' in formula: {}",
                    c, formula
                )));
            }
        }
    }

    Ok(tokens)
}

/// Get operator precedence for precedence-climbing parser.
fn precedence(token: &Token) -> u8 {
    match token {
        Token::Plus | Token::Minus => 1,
        Token::Star | Token::Slash => 2,
        _ => 0,
    }
}

/// Parse expression with precedence climbing.
fn parse_expr(tokens: &[Token], pos: usize, min_prec: u8) -> Result<(Expr, usize)> {
    let (mut left, mut pos) = parse_prefix(tokens, pos)?;

    while pos < tokens.len() {
        let prec = precedence(&tokens[pos]);
        if prec < min_prec {
            break;
        }
        let op = match &tokens[pos] {
            Token::Plus => BinOp::Add,
            Token::Minus => BinOp::Sub,
            Token::Star => BinOp::Mul,
            Token::Slash => BinOp::Div,
            _ => break,
        };
        pos += 1;
        let (right, new_pos) = parse_expr(tokens, pos, prec + 1)?;
        left = Expr::Binary(Box::new(left), op, Box::new(right));
        pos = new_pos;
    }

    Ok((left, pos))
}

/// Parse a prefix expression (number, field name, parenthesized, or unary minus).
fn parse_prefix(tokens: &[Token], pos: usize) -> Result<(Expr, usize)> {
    if pos >= tokens.len() {
        return Err(AppError::InvalidInput(
            "Unexpected end of formula".to_string(),
        ));
    }
    match &tokens[pos] {
        Token::Number(n) => Ok((Expr::Number(*n), pos + 1)),
        Token::Identifier(name) => Ok((Expr::Field(name.clone()), pos + 1)),
        Token::LParen => {
            let (inner, pos) = parse_expr(tokens, pos + 1, 0)?;
            if pos >= tokens.len() || tokens[pos] != Token::RParen {
                return Err(AppError::InvalidInput(
                    "Missing closing parenthesis in formula".to_string(),
                ));
            }
            Ok((inner, pos + 1))
        }
        Token::Minus => {
            // Unary minus: parse with higher precedence
            let (inner, pos) = parse_expr(tokens, pos + 1, 3)?;
            Ok((
                Expr::Binary(Box::new(Expr::Number(0.0)), BinOp::Sub, Box::new(inner)),
                pos,
            ))
        }
        _ => Err(AppError::InvalidInput(format!(
            "Unexpected token in formula: {:?}",
            tokens[pos]
        ))),
    }
}

/// Parse a complete formula string into an AST.
fn parse_expression(formula: &str) -> Result<Expr> {
    let tokens = tokenize(formula)?;
    if tokens.is_empty() {
        return Err(AppError::InvalidInput("Empty formula".to_string()));
    }
    let (expr, pos) = parse_expr(&tokens, 0, 0)?;
    if pos < tokens.len() {
        return Err(AppError::InvalidInput(format!(
            "Unexpected trailing tokens in formula: {:?}",
            &tokens[pos..]
        )));
    }
    Ok(expr)
}

/// Evaluate a parsed expression against a single data row.
fn evaluate_expr(expr: &Expr, headers: &[String], row: &[CellData]) -> Result<f64> {
    match expr {
        Expr::Number(n) => Ok(*n),
        Expr::Field(name) => {
            let col_idx = headers.iter().position(|h| h == name).ok_or_else(|| {
                AppError::InvalidInput(format!("Field '{}' not found in column headers", name))
            })?;
            cell_value_to_f64_idx(row, col_idx).ok_or_else(|| {
                AppError::InvalidInput(format!(
                    "Field '{}' contains a non-numeric value in source data",
                    name
                ))
            })
        }
        Expr::Binary(left, op, right) => {
            let l = evaluate_expr(left, headers, row)?;
            let r = evaluate_expr(right, headers, row)?;
            match op {
                BinOp::Add => Ok(l + r),
                BinOp::Sub => Ok(l - r),
                BinOp::Mul => Ok(l * r),
                BinOp::Div => {
                    if r == 0.0 {
                        Err(AppError::InvalidInput(
                            "Division by zero in calculated field formula".to_string(),
                        ))
                    } else {
                        Ok(l / r)
                    }
                }
            }
        }
    }
}

/// Get cell value as f64 by usize index.
fn cell_value_to_f64_idx(row: &[CellData], col: usize) -> Option<f64> {
    row.get(col)
        .and_then(|c| c.value.as_ref())
        .and_then(|v| v.parse::<f64>().ok())
}

/// Process calculated fields: parse formulas, evaluate per row, and append results
/// as new columns to headers and data_rows.
/// Returns the mapping of calculated field name to its new column index.
fn process_calculated_fields(
    config: &PivotTableConfig,
    headers: &mut Vec<String>,
    data_rows: &mut [Vec<CellData>],
) -> Result<Vec<(String, u16)>> {
    let mut calc_cols: Vec<(String, u16)> = Vec::new();

    for cf in &config.calculated_fields {
        // Name conflict detection
        if headers.contains(&cf.name) {
            return Err(AppError::InvalidInput(format!(
                "Calculated field name '{}' conflicts with an existing column header",
                cf.name
            )));
        }

        // Parse expression (use current headers for field name resolution)
        let expr = parse_expression(&cf.formula)?;

        // Evaluate for each row and append result
        let col_idx = headers.len() as u16;
        for row in data_rows.iter_mut() {
            let val = evaluate_expr(&expr, headers, row)?;
            row.push(CellData {
                value: Some(format!("{}", val)),
                data_type: CellDataType::Float,
                formula: None,
            });
        }

        headers.push(cf.name.clone());
        calc_cols.push((cf.name.clone(), col_idx));
    }

    Ok(calc_cols)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_date_grouping_year() {
        let grouping = DateGrouping {
            column: 0,
            by_year: true,
            by_quarter: false,
            by_month: false,
            by_day: false,
        };
        assert_eq!(
            group_date_value("2024-03-15", &grouping),
            Some("2024".to_string())
        );
    }

    #[test]
    fn test_date_grouping_year_quarter() {
        let grouping = DateGrouping {
            column: 0,
            by_year: true,
            by_quarter: true,
            by_month: false,
            by_day: false,
        };
        assert_eq!(
            group_date_value("2024-03-15", &grouping),
            Some("2024-Q1".to_string())
        );
    }

    #[test]
    fn test_date_grouping_invalid() {
        let grouping = DateGrouping {
            column: 0,
            by_year: true,
            by_quarter: false,
            by_month: false,
            by_day: false,
        };
        assert_eq!(group_date_value("not-a-date", &grouping), None);
    }
}
