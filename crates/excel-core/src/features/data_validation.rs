use rust_xlsxwriter::{
    DataValidation, DataValidationErrorStyle as XlsxErrorStyle, DataValidationRule, Formula,
};

use crate::security;
use crate::types::*;

/// Add a data validation rule to a worksheet.
/// Supports multi-region sqref (space-separated ranges like "A1:A5 C1:C5").
pub fn add_data_validation(
    path: &str,
    config: &DataValidationConfig,
    params: &SecurityParams,
    sheet: &str,
) -> Result<WriteResult> {
    if params.dry_run {
        return Ok(WriteResult::dry_run_success());
    }

    security::create_backup_if_needed(params)?;

    let dv = build_data_validation(config)?;

    // Support multi-region sqref: split by space and apply to each range
    let ranges: Vec<&str> = config.range.split_whitespace().collect();
    if ranges.is_empty() {
        return Err(AppError::InvalidInput(
            "Empty range for data validation".to_string(),
        ));
    }

    crate::excel_write::modify_file_with_wb(path, params, |_, wb| {
        let worksheet = wb
            .worksheet_from_name(sheet)
            .map_err(|_e| AppError::SheetNotFound(sheet.to_string()))?;

        for range_str in &ranges {
            let (r1, c1, r2, c2) = crate::utils::cell_ref::parse_range(range_str)?;
            worksheet
                .add_data_validation(r1, c1, r2, c2, &dv)
                .map_err(AppError::Xlsx)?;
        }

        Ok(())
    })
}

/// Build a rust_xlsxwriter DataValidation from our config.
pub(crate) fn build_data_validation(config: &DataValidationConfig) -> Result<DataValidation> {
    let (formula1, formula2) = normalize_validation_formula(config);

    let dv = match &config.validation_type {
        DataValidationType::List => {
            let values: Vec<&str> = config
                .list_values
                .as_ref()
                .map(|v| v.iter().map(String::as_str).collect())
                .unwrap_or_default();
            DataValidation::new()
                .allow_list_strings(&values)
                .map_err(AppError::Xlsx)?
        }
        DataValidationType::Whole => {
            let rule = build_numeric_rule::<i32>(&config.operator, &formula1, &formula2)?;
            DataValidation::new().allow_whole_number(rule)
        }
        DataValidationType::Decimal => {
            let rule = build_numeric_rule::<f64>(&config.operator, &formula1, &formula2)?;
            DataValidation::new().allow_decimal_number(rule)
        }
        DataValidationType::Date => {
            let rule = build_formula_rule(&config.operator, &formula1, &formula2);
            DataValidation::new().allow_date_formula(rule)
        }
        DataValidationType::Time => {
            let rule = build_formula_rule(&config.operator, &formula1, &formula2);
            DataValidation::new().allow_time_formula(rule)
        }
        DataValidationType::TextLength => {
            let rule = build_numeric_rule::<u32>(&config.operator, &formula1, &formula2)?;
            DataValidation::new().allow_text_length(rule)
        }
        DataValidationType::Custom => {
            DataValidation::new().allow_custom(Formula::new(formula1.as_deref().unwrap_or("")))
        }
    };

    let dv = dv
        .ignore_blank(config.allow_blank)
        .show_dropdown(config.show_dropdown);

    // Apply error style
    let dv = dv.set_error_style(map_error_style(&config.error_style));

    let dv = if let Some(ref title) = config.prompt_title {
        dv.set_input_title(title.as_str())?
    } else {
        dv
    };

    let dv = if let Some(ref msg) = config.prompt_message {
        dv.set_input_message(msg.as_str())?
    } else {
        dv
    };

    let dv = if let Some(ref title) = config.error_title {
        dv.set_error_title(title.as_str())?
    } else {
        dv
    };

    let dv = if let Some(ref msg) = config.error_message {
        dv.set_error_message(msg.as_str())?
    } else {
        dv
    };

    Ok(dv)
}

/// Map our DataValidationErrorStyle to rust_xlsxwriter DataValidationErrorStyle.
fn map_error_style(style: &DataValidationErrorStyle) -> XlsxErrorStyle {
    match style {
        DataValidationErrorStyle::Stop => XlsxErrorStyle::Stop,
        DataValidationErrorStyle::Warning => XlsxErrorStyle::Warning,
        DataValidationErrorStyle::Information => XlsxErrorStyle::Information,
    }
}

/// Normalize validation formulas based on type.
/// Returns (formula1, formula2) as Option<String> for date/time serial conversion.
fn normalize_validation_formula(config: &DataValidationConfig) -> (Option<String>, Option<String>) {
    match &config.validation_type {
        DataValidationType::List => {
            // For list type, if list_values is not provided, use formula1 as a
            // comma-separated string. Wrap bare comma-separated values in quotes.
            let f1 = config.formula1.clone().map(|f| {
                let trimmed = f.trim();
                if trimmed.starts_with('=') || trimmed.starts_with('"') {
                    f
                } else if trimmed.contains(',') || trimmed.contains(' ') || trimmed.contains(';') {
                    // Wrap comma-separated values: each value gets quoted
                    let quoted: Vec<String> = trimmed
                        .split(',')
                        .map(|v| format!("\"{}\"", v.trim()))
                        .collect();
                    quoted.join(",")
                } else {
                    f
                }
            });
            (f1, config.formula2.clone())
        }
        DataValidationType::Date => {
            // Convert date strings (YYYY-MM-DD) to Excel serial numbers
            let f1 = config
                .formula1
                .as_ref()
                .and_then(|f| date_to_excel_serial(f));
            let f2 = config
                .formula2
                .as_ref()
                .and_then(|f| date_to_excel_serial(f));
            (f1, f2)
        }
        DataValidationType::Time => {
            // Convert time strings (HH:MM or HH:MM:SS) to Excel time fractions
            let f1 = config
                .formula1
                .as_ref()
                .and_then(|f| time_to_excel_serial(f));
            let f2 = config
                .formula2
                .as_ref()
                .and_then(|f| time_to_excel_serial(f));
            (f1, f2)
        }
        DataValidationType::Custom => {
            // Strip leading '=' for custom formulas
            let f1 = config.formula1.clone().map(|f| {
                let trimmed = f.trim();
                if trimmed.starts_with('=') {
                    trimmed[1..].to_string()
                } else {
                    f
                }
            });
            (f1, config.formula2.clone())
        }
        _ => (config.formula1.clone(), config.formula2.clone()),
    }
}

/// Convert a date string (YYYY-MM-DD) to Excel serial number string.
/// Excel epoch is 1899-12-30 (day 0).
fn date_to_excel_serial(date_str: &str) -> Option<String> {
    let trimmed = date_str.trim();
    // Already a number
    if let Ok(_n) = trimmed.parse::<f64>() {
        return Some(trimmed.to_string());
    }
    // Parse YYYY-MM-DD
    let parts: Vec<&str> = trimmed.split('-').collect();
    if parts.len() != 3 {
        // Not a recognizable date, return as-is (could be a formula like "=A1")
        return Some(trimmed.to_string());
    }
    let year: i32 = parts[0].parse().ok()?;
    let month: u32 = parts[1].parse().ok()?;
    let day: u32 = parts[2].parse().ok()?;
    if month == 0 || month > 12 || day == 0 || day > 31 {
        return Some(trimmed.to_string());
    }

    // Days from 1899-12-30 to the given date
    let serial = date_to_days_since_epoch(year, month, day);
    Some(serial.to_string())
}

/// Calculate days since 1899-12-30 (Excel epoch).
fn date_to_days_since_epoch(year: i32, month: u32, day: u32) -> i32 {
    let mut total_days = 0;

    // Days for complete years
    for y in 1900..year {
        total_days += if is_leap_year(y) { 366 } else { 365 };
    }

    // Days for complete months in current year
    for m in 1..month {
        total_days += days_in_month(year, m);
    }

    // Days in current month (Excel serial starts at 1 for 1900-01-01)
    total_days += day as i32 - 1;

    // Excel bug: 1900 is treated as a leap year
    if year > 1900 || (year == 1900 && month > 2) {
        total_days += 1;
    }

    total_days + 1
}

fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

fn days_in_month(year: i32, month: u32) -> i32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if is_leap_year(year) {
                29
            } else {
                28
            }
        }
        _ => 0,
    }
}

/// Convert a time string (HH:MM or HH:MM:SS) to Excel time fraction string.
fn time_to_excel_serial(time_str: &str) -> Option<String> {
    let trimmed = time_str.trim();
    // Already a number
    if let Ok(_n) = trimmed.parse::<f64>() {
        return Some(trimmed.to_string());
    }
    let parts: Vec<&str> = trimmed.split(':').collect();
    if parts.len() < 2 || parts.len() > 3 {
        return Some(trimmed.to_string());
    }
    let hours: f64 = parts[0].parse().ok()?;
    let minutes: f64 = parts[1].parse().ok()?;
    let seconds: f64 = if parts.len() == 3 {
        parts[2].parse().unwrap_or(0.0)
    } else {
        0.0
    };

    let fraction = (hours + minutes / 60.0 + seconds / 3600.0) / 24.0;
    Some(format!("{:.10}", fraction))
}

/// Build a DataValidationRule for numeric types (i32, f64, u32).
fn build_numeric_rule<T>(
    operator: &Option<DataValidationOperator>,
    formula1: &Option<String>,
    formula2: &Option<String>,
) -> Result<DataValidationRule<T>>
where
    T: std::str::FromStr + rust_xlsxwriter::IntoDataValidationValue,
    <T as std::str::FromStr>::Err: std::fmt::Debug,
{
    let f1 = formula1
        .as_deref()
        .ok_or_else(|| AppError::Custom("formula1 is required for numeric validation".into()))?
        .parse::<T>()
        .map_err(|e| AppError::Custom(format!("Failed to parse formula1: {:?}", e)))?;

    let op = operator
        .as_ref()
        .unwrap_or(&DataValidationOperator::Between);

    match op {
        DataValidationOperator::Between => {
            let f2 = formula2
                .as_deref()
                .ok_or_else(|| {
                    AppError::Custom("formula2 is required for Between operator".into())
                })?
                .parse::<T>()
                .map_err(|e| AppError::Custom(format!("Failed to parse formula2: {:?}", e)))?;
            Ok(DataValidationRule::Between(f1, f2))
        }
        DataValidationOperator::NotBetween => {
            let f2 = formula2
                .as_deref()
                .ok_or_else(|| {
                    AppError::Custom("formula2 is required for NotBetween operator".into())
                })?
                .parse::<T>()
                .map_err(|e| AppError::Custom(format!("Failed to parse formula2: {:?}", e)))?;
            Ok(DataValidationRule::NotBetween(f1, f2))
        }
        DataValidationOperator::Equal => Ok(DataValidationRule::EqualTo(f1)),
        DataValidationOperator::NotEqual => Ok(DataValidationRule::NotEqualTo(f1)),
        DataValidationOperator::GreaterThan => Ok(DataValidationRule::GreaterThan(f1)),
        DataValidationOperator::LessThan => Ok(DataValidationRule::LessThan(f1)),
        DataValidationOperator::GreaterThanOrEqual => {
            Ok(DataValidationRule::GreaterThanOrEqualTo(f1))
        }
        DataValidationOperator::LessThanOrEqual => Ok(DataValidationRule::LessThanOrEqualTo(f1)),
    }
}

/// Build a DataValidationRule using Formula strings (for Date and Time).
fn build_formula_rule(
    operator: &Option<DataValidationOperator>,
    formula1: &Option<String>,
    formula2: &Option<String>,
) -> DataValidationRule<Formula> {
    let f1 = Formula::new(formula1.as_deref().unwrap_or(""));
    let op = operator
        .as_ref()
        .unwrap_or(&DataValidationOperator::Between);

    match op {
        DataValidationOperator::Between => {
            let f2 = Formula::new(formula2.as_deref().unwrap_or(""));
            DataValidationRule::Between(f1, f2)
        }
        DataValidationOperator::NotBetween => {
            let f2 = Formula::new(formula2.as_deref().unwrap_or(""));
            DataValidationRule::NotBetween(f1, f2)
        }
        DataValidationOperator::Equal => DataValidationRule::EqualTo(f1),
        DataValidationOperator::NotEqual => DataValidationRule::NotEqualTo(f1),
        DataValidationOperator::GreaterThan => DataValidationRule::GreaterThan(f1),
        DataValidationOperator::LessThan => DataValidationRule::LessThan(f1),
        DataValidationOperator::GreaterThanOrEqual => DataValidationRule::GreaterThanOrEqualTo(f1),
        DataValidationOperator::LessThanOrEqual => DataValidationRule::LessThanOrEqualTo(f1),
    }
}

/// Remove a data validation rule from a worksheet range.
///
/// Since rust_xlsxwriter rebuilds the workbook from calamine data (which does not
/// preserve data validations), validations are effectively removed from the output
/// when not explicitly re-applied. This function performs a rebuild without re-applying
/// any data validation to the specified target range.
///
/// Note: If there are multiple validations on the same sheet, this will remove ALL of
/// them because the rebuild loses all existing validations. For targeted removal, the
/// caller should re-apply the other validations separately after this call.
pub fn remove_data_validation(
    path: &str,
    _sheet: &str,
    params: &SecurityParams,
    _target_range: &str,
) -> Result<WriteResult> {
    if params.dry_run {
        return Ok(WriteResult::dry_run_success());
    }

    security::create_backup_if_needed(params)?;

    // modify_file_with_wb rebuilds workbook from calamine. Data validations are NOT
    // automatically preserved during the rebuild since calamine reads only cell data.
    // Therefore, any validations not re-applied are effectively removed.
    crate::excel_write::modify_file_with_wb(path, params, |_, _wb| Ok(()))
}
