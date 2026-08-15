use crate::types::*;


/// Apply date grouping to source data.
/// Returns adjusted headers and new data rows with grouped date values.
pub(crate) fn apply_date_grouping(
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
pub(crate) fn group_date_value(value: &str, grouping: &DateGrouping) -> Option<String> {
    // Parse YYYY-MM-DD or YYYY/MM/DD
    let parts: Vec<&str> = value.split(['-', '/']).collect();

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
        let quarter = month.div_ceil(3);
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

