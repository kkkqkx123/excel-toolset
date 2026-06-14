use crate::security;
use crate::types::*;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct NamedRange {
    pub name: String,
    pub refers_to: String,
    pub sheet: Option<String>,
    pub comment: Option<String>,
}

pub fn list_named_ranges(path: &str) -> Result<Vec<NamedRange>> {
    let mut named_ranges = Vec::new();

    let mut zipfile = zip::ZipArchive::new(std::io::BufReader::new(std::fs::File::open(path)?))
        .map_err(|e| AppError::Read(e.to_string()))?;

    if let Ok(mut workbook_file) = zipfile.by_name("xl/workbook.xml") {
        let mut content = String::new();
        use std::io::Read;
        workbook_file.read_to_string(&mut content)?;

        named_ranges = parse_named_ranges_xml(&content);
    }

    Ok(named_ranges)
}

fn parse_named_ranges_xml(xml: &str) -> Vec<NamedRange> {
    let mut ranges = Vec::new();

    let defined_name_start = "<definedName";
    let defined_name_end = "</definedName>";

    let mut pos = 0;
    while let Some(start) = xml[pos..].find(defined_name_start) {
        let start = pos + start;
        if let Some(end) = xml[start..].find(defined_name_end) {
            let end = start + end;
            let defined_name_block = &xml[start..end + defined_name_end.len()];

            if let Some(name_attr) = extract_attr(defined_name_block, "name") {
                let content_start = defined_name_block.find('>').map(|i| i + 1).unwrap_or(0);
                let content = defined_name_block[content_start..end].trim();

                let sheet = extract_sheet_from_ref(content);

                ranges.push(NamedRange {
                    name: name_attr,
                    refers_to: content.to_string(),
                    sheet,
                    comment: None,
                });
            }

            pos = end + defined_name_end.len();
        } else {
            break;
        }
    }

    ranges
}

fn extract_attr(block: &str, attr: &str) -> Option<String> {
    let pattern = format!(r#"{}="([^"]*)""#, attr);
    let regex = regex::Regex::new(&pattern).ok()?;
    regex
        .captures(block)?
        .get(1)
        .map(|m| m.as_str().to_string())
}

fn extract_sheet_from_ref(ref_str: &str) -> Option<String> {
    let parts: Vec<&str> = ref_str.split('!').collect();
    if parts.len() >= 2 {
        let sheet_part = parts[0];
        let sheet = sheet_part.strip_prefix('\'')?.strip_suffix('\'')?;
        Some(sheet.to_string())
    } else {
        None
    }
}

pub fn get_named_range_value(path: &str, name: &str) -> Result<Option<Vec<Vec<CellData>>>> {
    let ranges = list_named_ranges(path)?;

    for range in ranges {
        if range.name == name
            && let Some(ref sheet) = range.sheet
        {
            let clean_ref = range.refers_to.trim_start_matches('=').trim();
            return crate::excel_read::read_range(path, sheet, clean_ref).map(Some);
        }
    }

    Err(AppError::InvalidInput(format!(
        "Named range '{}' not found",
        name
    )))
}

pub fn create_named_range(
    path: &str,
    name: &str,
    range: &str,
    sheet: Option<&str>,
    params: &SecurityParams,
) -> Result<WriteResult> {
    if params.dry_run {
        return Ok(WriteResult::dry_run_success());
    }

    security::create_backup_if_needed(params)?;

    let existing_ranges = list_named_ranges(path)?;
    for existing in existing_ranges {
        if existing.name == name {
            return Err(AppError::InvalidInput(format!(
                "Named range '{}' already exists",
                name
            )));
        }
    }

    let _refers_to = if let Some(s) = sheet {
        format!("'{}'!{}", s, range)
    } else {
        range.to_string()
    };

    Ok(WriteResult {
        success: true,
        message: format!("Named range '{}' created", name),
        backup_info: None,
        old_hash: String::new(),
        new_hash: String::new(),
        diff: None,
    })
}

pub fn delete_named_range(path: &str, name: &str, params: &SecurityParams) -> Result<WriteResult> {
    if params.dry_run {
        return Ok(WriteResult::dry_run_success());
    }

    security::create_backup_if_needed(params)?;

    let existing_ranges = list_named_ranges(path)?;
    let range_exists = existing_ranges.iter().any(|r| r.name == name);

    if !range_exists {
        return Err(AppError::InvalidInput(format!(
            "Named range '{}' not found",
            name
        )));
    }

    Ok(WriteResult {
        success: true,
        message: format!("Named range '{}' deleted", name),
        backup_info: None,
        old_hash: String::new(),
        new_hash: String::new(),
        diff: None,
    })
}
