use std::collections::HashMap;
use std::fs::File;
use zip::ZipArchive;
use crate::security::{compute_file_hash, create_backup};
use crate::types::{
    AppError,
    Result, SecurityParams,
    WriteResult,
};

use super::*;

/// Preserving sheet add: modifies workbook.xml, [Content_Types].xml and workbook.xml.rels,
/// writes an empty sheet XML, and keeps every other zip part byte-for-byte.
pub fn add_sheet_preserving(
    path: &str,
    params: &SecurityParams,
    sheet: &str,
) -> Result<WriteResult> {
    let old_hash = compute_file_hash(path).map_err(AppError::Io)?;
    let backup_info = if params.create_backup {
        Some(create_backup(path, &old_hash).map_err(AppError::Io)?)
    } else {
        None
    };

    if params.dry_run {
        return Ok(WriteResult::dry_run_success());
    }

    let file = File::open(path).map_err(AppError::Io)?;
    let mut archive = ZipArchive::new(file)
        .map_err(|e| AppError::Custom(format!("failed to open xlsx as zip: {}", e)))?;

    // Read the existing files
    let wb_xml = read_zip_entry(&mut archive, "xl/workbook.xml")?;
    let ct_xml = read_zip_entry(&mut archive, "[Content_Types].xml")?;
    let rels_xml = read_zip_entry(&mut archive, "xl/_rels/workbook.xml.rels")?;

    // Check whether the sheet already exists
    let wb_str = String::from_utf8_lossy(&wb_xml);
    if wb_str.contains(&format!("name=\"{}\"", sheet)) {
        return Err(AppError::SheetAlreadyExists(sheet.into()));
    }

    // Determine the next sheet number, rId and sheetId
    let next_sheet_num = next_sheet_number(&wb_str);
    let next_rid = next_rid(&wb_str);
    let next_sheet_id = next_sheet_id(&wb_str);

    let sheet_part = format!("xl/worksheets/sheet{}.xml", next_sheet_num);
    let sheet_part_name = format!("/xl/worksheets/sheet{}.xml", next_sheet_num);

    // 1. Modify workbook.xml — append <sheet>
    let new_wb = patch_add_sheet_str(&wb_xml, sheet, &next_rid, next_sheet_id)?;

    // 2. Modify [Content_Types].xml — append the Override
    let new_ct = patch_add_content_type_str(&ct_xml, &sheet_part_name)?;

    // 3. Modify workbook.xml.rels — append the Relationship
    let new_rels = patch_add_sheet_rel_str(&rels_xml, &next_rid, &sheet_part)?;

    // 4. Create an empty sheet XML
    let empty_sheet = b"<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>
<worksheet xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\">
  <sheetData/>
</worksheet>
";

    // Build the change map
    let mut changes = HashMap::new();
    changes.insert("xl/workbook.xml".to_string(), new_wb);
    changes.insert("[Content_Types].xml".to_string(), new_ct);
    changes.insert("xl/_rels/workbook.xml.rels".to_string(), new_rels);
    changes.insert(sheet_part, empty_sheet.to_vec());

    repackage_zip_multi(&mut archive, path, &changes, &[])?;

    let new_hash = compute_file_hash(path).map_err(AppError::Io)?;
    append_history(path, "add_sheet", &old_hash, &new_hash, params.dry_run);

    Ok(WriteResult {
        success: true,
        message: format!("Added sheet '{}'", sheet),
        backup_info,
        old_hash,
        new_hash,
        diff: None,
    })
}


/// Preserving sheet delete: removes the corresponding entries from workbook.xml,
/// [Content_Types].xml and workbook.xml.rels, skips the sheet's XML entry,
/// and keeps every other zip part byte-for-byte.
pub fn delete_sheet_preserving(
    path: &str,
    params: &SecurityParams,
    sheet: &str,
) -> Result<WriteResult> {
    let old_hash = compute_file_hash(path).map_err(AppError::Io)?;
    let backup_info = if params.create_backup {
        Some(create_backup(path, &old_hash).map_err(AppError::Io)?)
    } else {
        None
    };

    if params.dry_run {
        return Ok(WriteResult::dry_run_success());
    }

    let file = File::open(path).map_err(AppError::Io)?;
    let mut archive = ZipArchive::new(file)
        .map_err(|e| AppError::Custom(format!("failed to open xlsx as zip: {}", e)))?;

    let wb_xml = read_zip_entry(&mut archive, "xl/workbook.xml")?;
    let ct_xml = read_zip_entry(&mut archive, "[Content_Types].xml")?;
    let rels_xml = read_zip_entry(&mut archive, "xl/_rels/workbook.xml.rels")?;

    // Check whether the sheet exists and get its rid
    let wb_str = String::from_utf8_lossy(&wb_xml);
    if !wb_str.contains(&format!("name=\"{}\"", sheet)) {
        return Err(AppError::SheetNotFound(sheet.into()));
    }

    // Extract the sheet's rid
    let rid = extract_sheet_rid_str(&wb_xml, sheet)?;

    // Check whether any sheet remains after deletion
    let sheet_count = wb_str.matches("<sheet ").count();
    if sheet_count <= 1 {
        return Err(AppError::Custom("Cannot delete all sheets from a workbook".to_string()));
    }

    // Find the sheet's part path via its rid
    let sheet_part = find_rel_target_str(&rels_xml, &rid)?;
    let part = if sheet_part.starts_with("xl/") {
        sheet_part.clone()
    } else if sheet_part.starts_with('/') {
        sheet_part.trim_start_matches('/').to_string()
    } else {
        format!("xl/{}", sheet_part)
    };

    // 1. Modify workbook.xml — remove <sheet>
    let new_wb = patch_remove_sheet_str(&wb_xml, sheet)?;

    // 2. Modify [Content_Types].xml — remove the corresponding Override
    let new_ct = patch_remove_content_type_str(&ct_xml, &part)?;

    // 3. Modify workbook.xml.rels — remove the corresponding Relationship
    let new_rels = patch_remove_rel_str(&rels_xml, &rid)?;

    let mut changes = HashMap::new();
    changes.insert("xl/workbook.xml".to_string(), new_wb);
    changes.insert("[Content_Types].xml".to_string(), new_ct);
    changes.insert("xl/_rels/workbook.xml.rels".to_string(), new_rels);

    // Skip the sheet's XML entry from the zip
    let skip_parts = vec![part];

    repackage_zip_multi(&mut archive, path, &changes, &skip_parts)?;

    let new_hash = compute_file_hash(path).map_err(AppError::Io)?;
    append_history(path, "delete_sheet", &old_hash, &new_hash, params.dry_run);

    Ok(WriteResult {
        success: true,
        message: format!("Deleted sheet '{}'", sheet),
        backup_info,
        old_hash,
        new_hash,
        diff: None,
    })
}


/// Preserving sheet rename: modifies the name attribute of the corresponding sheet
/// in workbook.xml, keeping every other zip part byte-for-byte.
pub fn rename_sheet_preserving(
    path: &str,
    params: &SecurityParams,
    old_name: &str,
    new_name: &str,
) -> Result<WriteResult> {
    let old_hash = compute_file_hash(path).map_err(AppError::Io)?;
    let backup_info = if params.create_backup {
        Some(create_backup(path, &old_hash).map_err(AppError::Io)?)
    } else {
        None
    };

    if params.dry_run {
        return Ok(WriteResult::dry_run_success());
    }

    let file = File::open(path).map_err(AppError::Io)?;
    let mut archive = ZipArchive::new(file)
        .map_err(|e| AppError::Custom(format!("failed to open xlsx as zip: {}", e)))?;

    let wb_xml = read_zip_entry(&mut archive, "xl/workbook.xml")?;

    let wb_str = String::from_utf8_lossy(&wb_xml);
    if !wb_str.contains(&format!("name=\"{}\"", old_name)) {
        return Err(AppError::SheetNotFound(old_name.into()));
    }
    if wb_str.contains(&format!("name=\"{}\"", new_name)) {
        return Err(AppError::SheetAlreadyExists(new_name.into()));
    }

    let new_wb = patch_rename_sheet_str(&wb_xml, old_name, new_name)?;

    let mut changes = HashMap::new();
    changes.insert("xl/workbook.xml".to_string(), new_wb);

    repackage_zip_multi(&mut archive, path, &changes, &[])?;

    let new_hash = compute_file_hash(path).map_err(AppError::Io)?;
    append_history(path, "rename_sheet", &old_hash, &new_hash, params.dry_run);

    Ok(WriteResult {
        success: true,
        message: format!("Renamed sheet '{}' to '{}'", old_name, new_name),
        backup_info,
        old_hash,
        new_hash,
        diff: None,
    })
}

// ───────────────────────────────────────────────────────────────────────────
// R2.2 internal helpers
// ───────────────────────────────────────────────────────────────────────────

