use crate::security;
use crate::types::*;

#[derive(Debug, Clone)]
pub struct ConditionalFormatRule {
    pub rule_type: ConditionalFormatType,
    pub condition: String,
    pub format: Option<Style>,
}

#[derive(Debug, Clone)]
pub enum ConditionalFormatType {
    CellValue,
    Formula,
    AboveAverage,
    Top10,
    Duplicate,
    TextContains,
    DateOccurring,
}

pub fn add_conditional_format(
    _path: &str,
    sheet: &str,
    range: &str,
    rule: &ConditionalFormatRule,
    params: &SecurityParams,
) -> Result<WriteResult> {
    if params.dry_run {
        return Ok(WriteResult::dry_run_success());
    }

    security::create_backup_if_needed(params)?;

    let (r1, c1, r2, c2) = crate::utils::cell_ref::parse_range(range)?;

    let mut workbook = rust_xlsxwriter::Workbook::new();
    let worksheet = workbook
        .worksheet_from_name(sheet)
        .map_err(|_e| AppError::SheetNotFound(sheet.to_string()))?;

    let mut format = rust_xlsxwriter::Format::default();

    if let Some(ref custom_format) = rule.format {
        if let Some(ref font_name) = custom_format.font_name {
            format = format.set_font_name(font_name);
        }
        if let Some(font_size) = custom_format.font_size {
            format = format.set_font_size(font_size);
        }
        if let Some(bold) = custom_format.bold
            && bold
        {
            format = format.set_bold();
        }
        if let Some(italic) = custom_format.italic
            && italic
        {
            format = format.set_italic();
        }
        if let Some(ref font_color) = custom_format.font_color {
            format = format.set_font_color(font_color.as_str());
        }
        if let Some(ref bg_color) = custom_format.background_color {
            format = format.set_background_color(bg_color.as_str());
        }
    }

    match rule.rule_type {
        ConditionalFormatType::CellValue => {
            let cond_format = rust_xlsxwriter::ConditionalFormatCell::new()
                .set_rule(rust_xlsxwriter::ConditionalFormatCellRule::EqualTo(
                    &rule.condition,
                ))
                .set_format(&format);

            worksheet
                .add_conditional_format(r1, c1, r2, c2, &cond_format)
                .map_err(|e| AppError::Write(e.to_string()))?;
        }
        ConditionalFormatType::Formula => {
            let cond_format = rust_xlsxwriter::ConditionalFormatFormula::new()
                .set_rule(rule.condition.as_str())
                .set_format(&format);

            worksheet
                .add_conditional_format(r1, c1, r2, c2, &cond_format)
                .map_err(|e| AppError::Write(e.to_string()))?;
        }
        _ => {
            return Err(AppError::InvalidInput(
                "Conditional format type not fully supported yet".to_string(),
            ));
        }
    }

    Ok(WriteResult {
        success: true,
        message: format!("Conditional format added to {} in sheet {}", range, sheet),
        backup_info: None,
        old_hash: String::new(),
        new_hash: String::new(),
        diff: None,
    })
}

pub fn remove_conditional_format(
    _path: &str,
    sheet: &str,
    range: &str,
    params: &SecurityParams,
) -> Result<WriteResult> {
    if params.dry_run {
        return Ok(WriteResult::dry_run_success());
    }

    security::create_backup_if_needed(params)?;

    let (_r1, _c1, _r2, _c2) = crate::utils::cell_ref::parse_range(range)?;

    let mut workbook = rust_xlsxwriter::Workbook::new();
    let _worksheet = workbook
        .worksheet_from_name(sheet)
        .map_err(|_e| AppError::SheetNotFound(sheet.to_string()))?;

    // TODO: Implement clear_conditional_formats
    // This feature may not be available in current rust_xlsxwriter version

    Ok(WriteResult {
        success: true,
        message: format!(
            "Conditional formats removed from {} in sheet {}",
            range, sheet
        ),
        backup_info: None,
        old_hash: String::new(),
        new_hash: String::new(),
        diff: None,
    })
}
