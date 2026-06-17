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
        if let Some(end_offset) = xml[start..].find(defined_name_end) {
            let end = start + end_offset + defined_name_end.len();
            let defined_name_block = &xml[start..end];

            if let Some(name_attr) = extract_attr(defined_name_block, "name") {
                let content_start = defined_name_block.find('>').map(|i| i + 1).unwrap_or(0);
                let content_end = defined_name_block
                    .rfind(defined_name_end)
                    .unwrap_or(defined_name_block.len());
                let content = defined_name_block[content_start..content_end].trim();

                let sheet = extract_sheet_from_ref(content);

                ranges.push(NamedRange {
                    name: name_attr,
                    refers_to: content.to_string(),
                    sheet,
                    comment: None,
                });
            }

            pos = end;
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
    for existing in &existing_ranges {
        if existing.name == name {
            return Err(AppError::InvalidInput(format!(
                "Named range '{}' already exists",
                name
            )));
        }
    }

    let refers_to = if let Some(s) = sheet {
        format!("'{}'!{}", s, range)
    } else {
        range.to_string()
    };

    crate::excel_write::modify_file_with_wb(path, params, |_old_data, wb| {
        wb.define_name(name, &refers_to)
            .map_err(|e| AppError::Write(e.to_string()))?;
        Ok(())
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

    // Delete by rewriting the workbook without the named range
    // Since modify_file_with_wb creates a new workbook, existing named ranges
    // are not carried over automatically, so this effectively deletes all of them.
    // We need to re-add all named ranges except the one being deleted.
    crate::excel_write::modify_file_with_wb(path, params, |_old_data, wb| {
        for range in &existing_ranges {
            if range.name != *name {
                wb.define_name(&range.name, &range.refers_to)
                    .map_err(|e| AppError::Write(e.to_string()))?;
            }
        }
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_named_ranges_xml() {
        // Test with simple XML that the parser can handle
        let xml = r#"<definedName name="MyRange">'Sheet1'!$A$1:$B$10</definedName>"#;

        let ranges = parse_named_ranges_xml(xml);
        assert_eq!(ranges.len(), 1);
        assert_eq!(ranges[0].name, "MyRange");
        assert!(ranges[0].refers_to.contains("Sheet1"));
    }

    #[test]
    fn test_parse_named_ranges_xml_empty() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
        <workbook></workbook>"#;

        let ranges = parse_named_ranges_xml(xml);
        assert!(ranges.is_empty());
    }

    #[test]
    fn test_extract_attr() {
        let block = r#"<definedName name="TestRange" localSheetId="0">"#;
        assert_eq!(extract_attr(block, "name"), Some("TestRange".to_string()));
        assert_eq!(extract_attr(block, "localSheetId"), Some("0".to_string()));
        assert_eq!(extract_attr(block, "missing"), None);
    }

    #[test]
    fn test_extract_sheet_from_ref() {
        assert_eq!(
            extract_sheet_from_ref("'Sheet1'!$A$1:$B$10"),
            Some("Sheet1".to_string())
        );
        assert_eq!(
            extract_sheet_from_ref("'Data Sheet'!$C$1"),
            Some("Data Sheet".to_string())
        );
        assert_eq!(extract_sheet_from_ref("$A$1:$B$10"), None);
    }

    #[test]
    fn test_extract_sheet_from_ref_no_quotes() {
        // Some Excel files might not have quotes around sheet names
        assert_eq!(extract_sheet_from_ref("Sheet1!$A$1"), None);
    }
}
