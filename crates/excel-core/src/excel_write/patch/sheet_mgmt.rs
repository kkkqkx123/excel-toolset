use crate::types::{AppError, Result};

pub(crate) fn extract_sheet_rid_str(wb: &[u8], sheet: &str) -> Result<String> {
    let s = String::from_utf8_lossy(wb);
    let marker = format!("name=\"{}\"", sheet);
    if let Some(name_pos) = s.find(&marker) {
        let prefix = &s[..name_pos];
        let tag_start = prefix
            .rfind("<sheet")
            .ok_or_else(|| AppError::Custom(format!("sheet tag not found: {}", sheet)))?;
        let tag = &s[tag_start..];
        if let Some(rid_start) = tag.find("r:id=\"") {
            let rest = &tag[rid_start + 6..];
            if let Some(rid_end) = rest.find('"') {
                return Ok(rest[..rid_end].to_string());
            }
        }
        Err(AppError::Custom(format!("sheet r:id not found: {}", sheet)))
    } else {
        Err(AppError::SheetNotFound(sheet.into()))
    }
}

pub(crate) fn find_rel_target_str(rels: &[u8], rid: &str) -> Result<String> {
    let s = String::from_utf8_lossy(rels);
    let marker = format!("Id=\"{}\"", rid);
    if let Some(id_pos) = s.find(&marker) {
        let prefix = &s[..id_pos];
        let tag_start = prefix
            .rfind("<Relationship")
            .ok_or_else(|| AppError::Custom(format!("Relationship tag not found: {}", rid)))?;
        let tag = &s[tag_start..];
        if let Some(t_start) = tag.find("Target=\"") {
            let rest = &tag[t_start + 8..];
            if let Some(t_end) = rest.find('"') {
                let target = rest[..t_end].to_string();
                // Normalize
                let trimmed = target
                    .strip_prefix('/')
                    .or_else(|| target.strip_prefix("xl/"))
                    .unwrap_or(&target);
                let part = if trimmed.starts_with("xl/") {
                    trimmed.to_string()
                } else {
                    format!("xl/{}", trimmed)
                };
                return Ok(part);
            }
        }
        Err(AppError::Custom(format!(
            "Target not found for rid {}",
            rid
        )))
    } else {
        Err(AppError::Custom(format!("rid not found: {}", rid)))
    }
}

pub(crate) fn next_sheet_number(wb_xml: &str) -> u32 {
    let mut max_n = 0u32;
    // Find all sheetId="N" patterns
    let mut pos = 0;
    while let Some(start) = wb_xml[pos..].find("sheetId=\"") {
        let rest = &wb_xml[pos + start + 9..];
        if let Some(end) = rest.find('"')
            && let Ok(n) = rest[..end].parse::<u32>()
            && n > max_n
        {
            max_n = n;
        }
        pos += start + 1;
    }
    max_n + 1
}

pub(crate) fn next_rid(wb_xml: &str) -> String {
    let mut max_n = 0u32;
    let mut pos = 0;
    while let Some(start) = wb_xml[pos..].find("r:id=\"rId") {
        let rest = &wb_xml[pos + start + 9..];
        if let Some(end) = rest.find('"')
            && let Ok(n) = rest[..end].parse::<u32>()
            && n > max_n
        {
            max_n = n;
        }
        pos += start + 1;
    }
    format!("rId{}", max_n + 1)
}

pub(crate) fn next_sheet_id(wb_xml: &str) -> u32 {
    next_sheet_number(wb_xml)
}

pub(crate) fn patch_add_sheet_str(
    wb: &[u8],
    sheet: &str,
    rid: &str,
    sheet_id: u32,
) -> Result<Vec<u8>> {
    let s = String::from_utf8(wb.to_vec())
        .map_err(|e| AppError::Custom(format!("XML is not valid UTF-8: {}", e)))?;
    let mut result = s;

    // Insert before </sheets>
    if let Some(pos) = result.find("</sheets>") {
        let new_sheet = format!(
            "\n    <sheet name=\"{}\" sheetId=\"{}\" r:id=\"{}\"/>",
            sheet, sheet_id, rid
        );
        result.insert_str(pos, &new_sheet);
        Ok(result.into_bytes())
    } else {
        Err(AppError::Custom(
            "cannot find </sheets> in workbook.xml".to_string(),
        ))
    }
}

pub(crate) fn patch_remove_sheet_str(wb: &[u8], sheet: &str) -> Result<Vec<u8>> {
    let s = String::from_utf8(wb.to_vec())
        .map_err(|e| AppError::Custom(format!("XML is not valid UTF-8: {}", e)))?;
    let mut result = s;

    let marker = format!("name=\"{}\"", sheet);
    if let Some(name_pos) = result.find(&marker) {
        let prefix = &result[..name_pos];
        let tag_start = prefix
            .rfind("<sheet")
            .ok_or_else(|| AppError::Custom(format!("sheet tag not found: {}", sheet)))?;
        // Find the tag end: > or />
        let rest = &result[tag_start..];
        let tag_end = rest
            .find('>')
            .ok_or_else(|| AppError::Custom("sheet tag end not found".to_string()))?
            + tag_start
            + 1;
        result.replace_range(tag_start..tag_end, "");
        Ok(result.into_bytes())
    } else {
        Err(AppError::SheetNotFound(sheet.into()))
    }
}

pub(crate) fn patch_rename_sheet_str(wb: &[u8], old_name: &str, new_name: &str) -> Result<Vec<u8>> {
    let s = String::from_utf8(wb.to_vec())
        .map_err(|e| AppError::Custom(format!("XML is not valid UTF-8: {}", e)))?;
    let mut result = s;

    let old_marker = format!("name=\"{}\"", old_name);
    let new_marker = format!("name=\"{}\"", new_name);
    result = result.replacen(&old_marker, &new_marker, 1);

    Ok(result.into_bytes())
}

pub(crate) fn patch_add_content_type_str(ct: &[u8], part_name: &str) -> Result<Vec<u8>> {
    let s = String::from_utf8(ct.to_vec())
        .map_err(|e| AppError::Custom(format!("XML is not valid UTF-8: {}", e)))?;
    let mut result = s;

    let content_type = "application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml";
    let new_override = format!(
        "\n  <Override PartName=\"{}\" ContentType=\"{}\"/>",
        part_name, content_type
    );

    if let Some(pos) = result.find("</Types>") {
        result.insert_str(pos, &new_override);
        Ok(result.into_bytes())
    } else {
        Err(AppError::Custom(
            "cannot find </Types> in [Content_Types].xml".to_string(),
        ))
    }
}

pub(crate) fn patch_remove_content_type_str(ct: &[u8], part: &str) -> Result<Vec<u8>> {
    let s = String::from_utf8(ct.to_vec())
        .map_err(|e| AppError::Custom(format!("XML is not valid UTF-8: {}", e)))?;
    let mut result = s;

    // part may be "xl/worksheets/sheet2.xml"; it needs to match "/xl/worksheets/sheet2.xml"
    let rel_part = if part.starts_with("xl/") {
        format!("/{}", part)
    } else {
        part.to_string()
    };

    let marker = format!("PartName=\"{}\"", rel_part);
    if let Some(pn_pos) = result.find(&marker) {
        let prefix = &result[..pn_pos];
        let tag_start = prefix
            .rfind("<Override")
            .ok_or_else(|| AppError::Custom(format!("Override tag not found: {}", part)))?;
        let rest = &result[tag_start..];
        let tag_end = rest
            .find("/>")
            .ok_or_else(|| AppError::Custom("Override tag end not found".to_string()))?
            + tag_start
            + 2;
        result.replace_range(tag_start..tag_end, "");
        Ok(result.into_bytes())
    } else {
        // Not found is fine, continue
        Ok(ct.to_vec())
    }
}

pub(crate) fn patch_add_sheet_rel_str(rels: &[u8], rid: &str, target: &str) -> Result<Vec<u8>> {
    let s = String::from_utf8(rels.to_vec())
        .map_err(|e| AppError::Custom(format!("XML is not valid UTF-8: {}", e)))?;
    let mut result = s;

    // target may be "xl/worksheets/sheet3.xml", but rels usually use the relative path "worksheets/sheet3.xml"
    let rel_target = target.strip_prefix("xl/").unwrap_or(target);
    let rel_type = "http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet";
    let new_rel = format!(
        "\n  <Relationship Id=\"{}\" Type=\"{}\" Target=\"{}\"/>",
        rid, rel_type, rel_target
    );

    if let Some(pos) = result.find("</Relationships>") {
        result.insert_str(pos, &new_rel);
        Ok(result.into_bytes())
    } else {
        Err(AppError::Custom(
            "cannot find </Relationships> in rels".to_string(),
        ))
    }
}

pub(crate) fn patch_remove_rel_str(rels: &[u8], rid: &str) -> Result<Vec<u8>> {
    let s = String::from_utf8(rels.to_vec())
        .map_err(|e| AppError::Custom(format!("XML is not valid UTF-8: {}", e)))?;
    let mut result = s;

    let marker = format!("Id=\"{}\"", rid);
    if let Some(id_pos) = result.find(&marker) {
        let prefix = &result[..id_pos];
        let tag_start = prefix
            .rfind("<Relationship")
            .ok_or_else(|| AppError::Custom(format!("Relationship tag not found: {}", rid)))?;
        let rest = &result[tag_start..];
        let tag_end = rest
            .find("/>")
            .ok_or_else(|| AppError::Custom("cannot find Relationship tag end".to_string()))?
            + tag_start
            + 2;
        result.replace_range(tag_start..tag_end, "");
        Ok(result.into_bytes())
    } else {
        Err(AppError::Custom(format!("rid not found: {}", rid)))
    }
}

// ───────────────────────────────────────────────────────────────────────────
// Internal helpers
// ───────────────────────────────────────────────────────────────────────────
