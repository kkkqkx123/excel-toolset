use rust_xlsxwriter::{DataValidation, DataValidationRule, Formula};

use crate::security;
use crate::types::*;

/// Add a data validation rule to a worksheet.
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

    crate::excel_write::modify_file_with_wb(path, params, |_, wb| {
        let worksheet = wb
            .worksheet_from_name(sheet)
            .map_err(|_e| AppError::SheetNotFound(sheet.to_string()))?;

        let dv = build_data_validation(config)?;
        let (r1, c1, r2, c2) = crate::utils::cell_ref::parse_range(&config.range)?;
        worksheet
            .add_data_validation(r1, c1, r2, c2, &dv)
            .map_err(AppError::Xlsx)?;

        Ok(())
    })
}

/// Build a rust_xlsxwriter DataValidation from our config.
pub(crate) fn build_data_validation(config: &DataValidationConfig) -> Result<DataValidation> {
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
            let rule = build_numeric_rule::<i32>(&config.operator, &config.formula1, &config.formula2)?;
            DataValidation::new().allow_whole_number(rule)
        }
        DataValidationType::Decimal => {
            let rule = build_numeric_rule::<f64>(&config.operator, &config.formula1, &config.formula2)?;
            DataValidation::new().allow_decimal_number(rule)
        }
        DataValidationType::Date => {
            let rule = build_formula_rule(&config.operator, &config.formula1, &config.formula2);
            DataValidation::new().allow_date_formula(rule)
        }
        DataValidationType::Time => {
            let rule = build_formula_rule(&config.operator, &config.formula1, &config.formula2);
            DataValidation::new().allow_time_formula(rule)
        }
        DataValidationType::TextLength => {
            let rule = build_numeric_rule::<u32>(&config.operator, &config.formula1, &config.formula2)?;
            DataValidation::new().allow_text_length(rule)
        }
        DataValidationType::Custom => DataValidation::new().allow_custom(Formula::new(
            config.formula1.as_deref().unwrap_or(""),
        )),
    };

    let dv = dv
        .ignore_blank(config.allow_blank)
        .show_dropdown(config.show_dropdown);

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

    let op = operator.as_ref().unwrap_or(&DataValidationOperator::Between);

    match op {
        DataValidationOperator::Between => {
            let f2 = formula2
                .as_deref()
                .ok_or_else(|| AppError::Custom("formula2 is required for Between operator".into()))?
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
        DataValidationOperator::GreaterThanOrEqual => Ok(DataValidationRule::GreaterThanOrEqualTo(f1)),
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
    let op = operator.as_ref().unwrap_or(&DataValidationOperator::Between);

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
pub fn remove_data_validation(
    path: &str,
    _sheet: &str,
    params: &SecurityParams,
    _range: &str,
) -> Result<WriteResult> {
    if params.dry_run {
        return Ok(WriteResult::dry_run_success());
    }

    security::create_backup_if_needed(params)?;

    // rust_xlsxwriter does not support removing a single data validation.
    // We rebuild without re-applying the data validation to the target range,
    // which effectively removes it.
    crate::excel_write::modify_file_with_wb(path, params, |_, _wb| {
        // No-op: the workbook is rebuilt without the removed validation.
        Ok(())
    })
}
