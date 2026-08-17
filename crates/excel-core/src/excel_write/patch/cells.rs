use crate::security::{compute_file_hash, create_backup};
use crate::types::{AppError, CellData, CellDataType, Result, SecurityParams, WriteResult};
use crate::utils::cell_ref::index_to_col;
use std::collections::HashMap;
use std::fs::File;
use zip::ZipArchive;

use super::*;

pub fn write_cells_preserving(
    path: &str,
    params: &SecurityParams,
    sheet: &str,
    edits: &[(u32, u16, CellData)],
) -> Result<WriteResult> {
    let old_hash = compute_file_hash(path).map_err(AppError::Io)?;
    let backup_info = if params.create_backup {
        Some(create_backup(path, &old_hash).map_err(AppError::Io)?)
    } else {
        None
    };

    // Empty edits or dry-run: leave the file untouched and keep the old hash.
    if edits.is_empty() || params.dry_run {
        return Ok(WriteResult {
            success: true,
            message: String::new(),
            backup_info,
            old_hash: old_hash.clone(),
            new_hash: old_hash,
            diff: None,
        });
    }

    let file = File::open(path).map_err(AppError::Io)?;
    let mut archive = ZipArchive::new(file)
        .map_err(|e| AppError::Custom(format!("failed to open xlsx as zip: {}", e)))?;

    let part = resolve_sheet_part(&mut archive, sheet)?;
    let sheet_xml = read_zip_entry(&mut archive, &part)?;
    let new_xml = patch_sheet_xml(&sheet_xml, edits)?;

    // Repackage: copy every part except the target sheet byte-for-byte.
    repackage_zip(&mut archive, path, &part, &new_xml)?;

    let new_hash = compute_file_hash(path).map_err(AppError::Io)?;

    append_history(path, "write_cells", &old_hash, &new_hash, params.dry_run);

    Ok(WriteResult {
        success: true,
        message: String::new(),
        backup_info,
        old_hash,
        new_hash,
        diff: None,
    })
}

/// Preserving formula set: rewrites only the formula of cell `(row, col)` in `sheet`,
/// keeping every other zip part byte-for-byte. `row`/`col` are both 0-based.
pub fn set_formula_preserving(
    path: &str,
    params: &SecurityParams,
    sheet: &str,
    row: u32,
    col: u16,
    formula: &str,
) -> Result<WriteResult> {
    let cd = CellData {
        value: None,
        data_type: CellDataType::String,
        formula: Some(formula.to_string()),
    };
    write_cells_preserving(path, params, sheet, &[(row, col, cd)])
}

/// Preserving formula + cached value set: writes both `<f>` and `<v>`,
/// keeping every other zip part byte-for-byte.
#[allow(clippy::too_many_arguments)]
pub fn set_formula_with_value_preserving(
    path: &str,
    params: &SecurityParams,
    sheet: &str,
    row: u32,
    col: u16,
    formula: &str,
    cached_value: &str,
    data_type: CellDataType,
) -> Result<WriteResult> {
    let cd = CellData {
        value: Some(cached_value.to_string()),
        data_type,
        formula: Some(formula.to_string()),
    };
    write_cells_preserving(path, params, sheet, &[(row, col, cd)])
}

/// Preserving range clear: removes the `<f>`/`<v>` of every existing cell inside the range,
/// keeping every other zip part byte-for-byte.
/// `r_start/r_end` are 0-based row indexes, `c_start/c_end` are 0-based column indexes.
pub fn clear_range_preserving(
    path: &str,
    params: &SecurityParams,
    sheet: &str,
    r_start: u32,
    r_end: u32,
    c_start: u16,
    c_end: u16,
) -> Result<WriteResult> {
    let old_hash = compute_file_hash(path).map_err(AppError::Io)?;
    let backup_info = if params.create_backup {
        Some(create_backup(path, &old_hash).map_err(AppError::Io)?)
    } else {
        None
    };

    if params.dry_run {
        return Ok(WriteResult {
            success: true,
            message: String::new(),
            backup_info,
            old_hash: old_hash.clone(),
            new_hash: old_hash,
            diff: None,
        });
    }

    let file = File::open(path).map_err(AppError::Io)?;
    let mut archive = ZipArchive::new(file)
        .map_err(|e| AppError::Custom(format!("failed to open xlsx as zip: {}", e)))?;
    let part = resolve_sheet_part(&mut archive, sheet)?;
    let sheet_xml = read_zip_entry(&mut archive, &part)?;

    let (before, inner, after, self_closed) = match sheetdata_spans(&sheet_xml) {
        Some(x) => x,
        None => {
            // No sheetData element: no cells to clear.
            return Ok(WriteResult {
                success: true,
                message: "empty sheet, nothing to clear".to_string(),
                backup_info,
                old_hash: old_hash.clone(),
                new_hash: old_hash,
                diff: None,
            });
        }
    };

    let mut model = parse_sheetdata(inner);
    let mut modified = false;
    model.rows.retain(|&rk, row| {
        if rk >= r_start && rk <= r_end {
            row.cells.retain(|&ck, _| {
                if ck >= c_start && ck <= c_end {
                    modified = true;
                    false // remove this cell (clearing effect)
                } else {
                    true
                }
            });
        }
        !row.cells.is_empty() // drop empty rows
    });

    if !modified {
        return Ok(WriteResult {
            success: true,
            message: "no cells in range to clear".to_string(),
            backup_info,
            old_hash: old_hash.clone(),
            new_hash: old_hash,
            diff: None,
        });
    }

    let new_inner = serialize_sheet(&model);
    let refstr = dimension_ref(&model);
    let mut before_vec = before.to_vec();
    replace_dimension(&mut before_vec, &refstr);

    let mut new_xml = Vec::with_capacity(sheet_xml.len());
    if self_closed {
        new_xml.extend_from_slice(&before_vec);
        new_xml.extend_from_slice(b"<sheetData>");
        new_xml.extend_from_slice(new_inner.as_bytes());
        new_xml.extend_from_slice(b"</sheetData>");
        new_xml.extend_from_slice(after);
    } else {
        new_xml.extend_from_slice(&before_vec);
        new_xml.extend_from_slice(new_inner.as_bytes());
        new_xml.extend_from_slice(after);
    }

    repackage_zip(&mut archive, path, &part, &new_xml)?;
    let new_hash = compute_file_hash(path).map_err(AppError::Io)?;

    append_history(path, "clear_range", &old_hash, &new_hash, params.dry_run);

    Ok(WriteResult {
        success: true,
        message: String::new(),
        backup_info,
        old_hash,
        new_hash,
        diff: None,
    })
}

/// Preserving formula cached-value clear: removes the `<v>` element of every formula cell in
/// `sheet` so formulas are recomputed on next open.
pub fn clear_formula_values_preserving(
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
        return Ok(WriteResult {
            success: true,
            message: String::new(),
            backup_info,
            old_hash: old_hash.clone(),
            new_hash: old_hash,
            diff: None,
        });
    }

    let file = File::open(path).map_err(AppError::Io)?;
    let mut archive = ZipArchive::new(file)
        .map_err(|e| AppError::Custom(format!("failed to open xlsx as zip: {}", e)))?;
    let part = resolve_sheet_part(&mut archive, sheet)?;
    let sheet_xml = read_zip_entry(&mut archive, &part)?;

    let (before, inner, after, self_closed) = match sheetdata_spans(&sheet_xml) {
        Some(x) => x,
        None => {
            return Ok(WriteResult {
                success: true,
                message: "empty sheet, no formulas to refresh".to_string(),
                backup_info,
                old_hash: old_hash.clone(),
                new_hash: old_hash,
                diff: None,
            });
        }
    };

    let mut model = parse_sheetdata(inner);
    let mut modified = false;
    for row in model.rows.values_mut() {
        for cell in row.cells.values_mut() {
            if has_formula(&cell.raw) {
                cell.raw = strip_v_element(&cell.raw);
                modified = true;
            }
        }
    }

    if !modified {
        return Ok(WriteResult {
            success: true,
            message: "no formulas to refresh".to_string(),
            backup_info,
            old_hash: old_hash.clone(),
            new_hash: old_hash,
            diff: None,
        });
    }

    let new_inner = serialize_sheet(&model);
    let mut new_xml = Vec::with_capacity(sheet_xml.len());
    if self_closed {
        new_xml.extend_from_slice(before);
        new_xml.extend_from_slice(b"<sheetData>");
        new_xml.extend_from_slice(new_inner.as_bytes());
        new_xml.extend_from_slice(b"</sheetData>");
        new_xml.extend_from_slice(after);
    } else {
        new_xml.extend_from_slice(before);
        new_xml.extend_from_slice(new_inner.as_bytes());
        new_xml.extend_from_slice(after);
    }

    repackage_zip(&mut archive, path, &part, &new_xml)?;
    let new_hash = compute_file_hash(path).map_err(AppError::Io)?;

    append_history(
        path,
        "refresh_formulas",
        &old_hash,
        &new_hash,
        params.dry_run,
    );

    Ok(WriteResult {
        success: true,
        message: String::new(),
        backup_info,
        old_hash,
        new_hash,
        diff: None,
    })
}

/// Preserving merge: appends the merged range to the target sheet,
/// keeping every other zip part byte-for-byte.
/// `r1/r2` are 0-based row indexes, `c1/c2` are 0-based column indexes.
/// Writes the given value into the top-left cell.
#[allow(clippy::too_many_arguments)]
pub fn merge_cells_preserving(
    path: &str,
    params: &SecurityParams,
    sheet: &str,
    r1: u32,
    c1: u16,
    r2: u32,
    c2: u16,
    value: &str,
) -> Result<WriteResult> {
    let range_ref = format!(
        "{}{}:{}{}",
        index_to_col(c1),
        r1 + 1,
        index_to_col(c2),
        r2 + 1
    );

    let old_hash = compute_file_hash(path).map_err(AppError::Io)?;
    let backup_info = if params.create_backup {
        Some(create_backup(path, &old_hash).map_err(AppError::Io)?)
    } else {
        None
    };

    if params.dry_run {
        return Ok(WriteResult {
            success: true,
            message: String::new(),
            backup_info,
            old_hash: old_hash.clone(),
            new_hash: old_hash,
            diff: None,
        });
    }

    let file = File::open(path).map_err(AppError::Io)?;
    let mut archive = ZipArchive::new(file)
        .map_err(|e| AppError::Custom(format!("failed to open xlsx as zip: {}", e)))?;
    let part = resolve_sheet_part(&mut archive, sheet)?;
    let sheet_xml = read_zip_entry(&mut archive, &part)?;

    let new_sheet_xml = patch_merge_cells_str(&sheet_xml, &range_ref)?;

    repackage_zip(&mut archive, path, &part, &new_sheet_xml)?;

    // If a value is provided, write it to the top-left cell
    if !value.is_empty() {
        let cd = CellData {
            value: Some(value.to_string()),
            data_type: CellDataType::String,
            formula: None,
        };
        // archive is consumed by repackage_zip, so we need to reopen
        write_cells_preserving(path, params, sheet, &[(r1, c1, cd)])?;
    }

    let new_hash = compute_file_hash(path).map_err(AppError::Io)?;
    append_history(path, "merge_cells", &old_hash, &new_hash, params.dry_run);

    Ok(WriteResult {
        success: true,
        message: String::new(),
        backup_info,
        old_hash,
        new_hash,
        diff: None,
    })
}

/// Preserving removal of all images on the sheet (including the drawing layer).
///
/// Root cause: `remove_image` previously used `modify_file_with_wb` full rebuild, but
/// `preserve_all_parts_transfer` kept the source file's existing `xl/drawings/*` and
/// `xl/media/*` parts intact, so the "removal" had no effect (images remained).
///
/// This instead rewrites in place at the zip level: it deletes the sheet's drawing part,
/// its media files and corresponding relationships, and strips `<drawing>` and the drawing
/// relationship from the sheet xml / rels.
pub fn remove_images_preserving(
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

    let sheet_part = resolve_sheet_part(&mut archive, sheet)?;
    let sheet_rels = sheet_part
        .replacen("worksheets/", "worksheets/_rels/", 1)
        .replace(".xml", ".xml.rels");

    // No sheet rels (hence no images) -> succeed immediately
    let sheet_rels_xml = match read_zip_entry(&mut archive, &sheet_rels) {
        Ok(b) => b,
        Err(_) => {
            return Ok(WriteResult {
                success: true,
                message: format!("Sheet '{}' has no images to remove", sheet),
                backup_info,
                old_hash: old_hash.clone(),
                new_hash: old_hash,
                diff: None,
            });
        }
    };

    // Find the sheet's drawing relationship; if absent, succeed immediately
    let drawing_rid = match find_rel_by_type(&sheet_rels_xml, "drawing") {
        Some(r) => r,
        None => {
            return Ok(WriteResult {
                success: true,
                message: format!("Sheet '{}' has no images to remove", sheet),
                backup_info,
                old_hash: old_hash.clone(),
                new_hash: old_hash,
                diff: None,
            });
        }
    };

    let drawing_target = find_rel_target(&sheet_rels_xml, &drawing_rid)
        .ok_or_else(|| AppError::Custom("drawing relationship target not found".into()))?;
    let drawing_part = normalize_rel_target(&sheet_part, &drawing_target);

    let drawing_file = drawing_part
        .split('/')
        .next_back()
        .unwrap_or("drawing1.xml");
    let drawing_rels = format!(
        "xl/drawings/_rels/{}.xml.rels",
        drawing_file.trim_end_matches(".xml")
    );

    let mut skip: Vec<String> = vec![drawing_part.clone(), drawing_rels.clone()];
    if let Ok(dr) = read_zip_entry(&mut archive, &drawing_rels) {
        for tgt in find_all_rel_targets_by_type(&dr, "image") {
            skip.push(normalize_rel_target(&drawing_part, &tgt));
        }
    }

    // Modify sheet rels (remove the drawing relationship) and sheet xml (remove <drawing>)
    let sheet_rels_new = remove_rel_by_id(&sheet_rels_xml, &drawing_rid);
    let sheet_xml = read_zip_entry(&mut archive, &sheet_part)?;
    let sheet_xml_new = remove_drawing_elem(&sheet_xml, &drawing_rid);

    let mut changes: HashMap<String, Vec<u8>> = HashMap::new();
    changes.insert(sheet_rels, sheet_rels_new.into_bytes());
    changes.insert(sheet_part.clone(), sheet_xml_new.into_bytes());

    repackage_zip_multi(&mut archive, path, &changes, &skip)?;

    let new_hash = compute_file_hash(path).map_err(AppError::Io)?;
    append_history(path, "remove_images", &old_hash, &new_hash, params.dry_run);

    Ok(WriteResult {
        success: true,
        message: format!("Removed images from sheet '{}'", sheet),
        backup_info,
        old_hash,
        new_hash,
        diff: None,
    })
}
