//! Preserving write: incremental rewriting at the zip level.
//!
//! Root cause: previously `modify_file` / `modify_file_with_wb` rebuilt the whole
//! workbook via `rust_xlsxwriter::Workbook::new()`, writing back only the "values + formulas"
//! that calamine read, and wiping out everything calamine did not read from the source file:
//! styles / merges / charts / comments / data validation / frozen panes / drawing layer.
//!
//! This module rewrites only the target sheet's `<sheetData>` **in place** inside the existing
//! xlsx zip package; every other part (`styles.xml`, `drawings/*`, `charts/*`,
//! `comments*.xml`, other sheets, `rels`, `[Content_Types].xml`) is copied byte-for-byte,
//! preserving 100% of the source file's features.
//!
//! Compiled only when the `zip` feature is enabled (on by default with `full`).

use std::collections::{BTreeMap, HashMap};
use std::fs::{self, File};
use std::io::{Cursor, Read, Write};
use std::path::Path;

use quick_xml::events::Event;
use quick_xml::escape::escape;
use quick_xml::Reader;
use zip::{ZipArchive, ZipWriter};

use crate::security::{append_history_entry, compute_file_hash, create_backup};
use crate::types::{
    AppError, CellData, CellDataType, DataValidationConfig, DataValidationType, PageSetupConfig,
    Result, SecurityParams, SheetProtectionConfig, SheetVisibility, WorkbookHistoryEntry,
    WriteResult,
};
use crate::utils::cell_ref::{index_to_col, parse_cell_ref};

// ───────────────────────────────────────────────────────────────────────────
// Public entry points
// ───────────────────────────────────────────────────────────────────────────

/// Preserving write: rewrites only the cells specified by `edits` in `sheet`,
/// keeping every other zip part byte-for-byte.
///
/// Each element of `edits` is `(0-based row index, 0-based column index, new cell data)`.
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
    r_start: u32, r_end: u32, c_start: u16, c_end: u16,
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

    append_history(path, "refresh_formulas", &old_hash, &new_hash, params.dry_run);

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
pub fn merge_cells_preserving(
    path: &str,
    params: &SecurityParams,
    sheet: &str,
    r1: u32, c1: u16, r2: u32, c2: u16,
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

/// Patches the `mergeCells` element using string operations.
fn patch_merge_cells_str(xml: &[u8], new_range: &str) -> Result<Vec<u8>> {
    let s = String::from_utf8(xml.to_vec())
        .map_err(|e| AppError::Custom(format!("XML is not valid UTF-8: {}", e)))?;
    let mut result = s;

    // Check whether a <mergeCells> element already exists (closed or self-closed)
    if let Some(pos) = result.find("</mergeCells>") {
        // Open tag exists -> insert the new entry before the closing tag and update count
        let mc = format!("    <mergeCell ref=\"{}\"/>\n", new_range);
        result.insert_str(pos, &mc);
        // Update the count attribute
        let old_count = count_merge_cells(&result);
        let new_count = old_count + 1;
        // Find count="N" and replace it with the new value
        let old_count_str = format!("count=\"{}\"", old_count);
        let new_count_str = format!("count=\"{}\"", new_count);
        result = result.replacen(&old_count_str, &new_count_str, 1);
    } else if let Some(pos) = result.find("<mergeCells/>") {
        // Self-closed tag -> convert to an open tag
        let replacement = format!(
            "<mergeCells count=\"1\">\n    <mergeCell ref=\"{}\"/>\n  </mergeCells>",
            new_range
        );
        result.replace_range(pos..pos + 13, &replacement);
    } else {
        // No mergeCells element -> insert before </worksheet>
        if let Some(pos) = result.find("</worksheet>") {
            let new_entry = format!(
                "  <mergeCells count=\"1\">\n    <mergeCell ref=\"{}\"/>\n  </mergeCells>\n",
                new_range
            );
            result.insert_str(pos, &new_entry);
        }
    }

    Ok(result.into_bytes())
}

/// Counts occurrences of `<mergeCell` in the current XML (used to update the count attribute).
fn count_merge_cells(xml: &str) -> usize {
    xml.matches("<mergeCell ").count()
}

/// Preserving freeze panes set: modifies the `<sheetViews>` element of the sheet XML,
/// keeping every other zip part byte-for-byte.
pub fn set_freeze_panes_preserving(
    path: &str,
    params: &SecurityParams,
    sheet: &str,
    rows: u32,
    cols: u16,
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

    let new_sheet_xml = patch_freeze_panes_str(&sheet_xml, rows, cols)?;

    repackage_zip(&mut archive, path, &part, &new_sheet_xml)?;
    let new_hash = compute_file_hash(path).map_err(AppError::Io)?;
    append_history(path, "set_freeze_panes", &old_hash, &new_hash, params.dry_run);

    Ok(WriteResult {
        success: true,
        message: String::new(),
        backup_info,
        old_hash,
        new_hash,
        diff: None,
    })
}

/// Preserving clear freeze panes.
pub fn clear_freeze_panes_preserving(
    path: &str,
    params: &SecurityParams,
    sheet: &str,
) -> Result<WriteResult> {
    set_freeze_panes_preserving(path, params, sheet, 0, 0)
}

/// Patches freeze panes using string operations.
///
/// Note: `<sheetView>` elements produced by rust_xlsxwriter are usually **self-closed**
/// (`<sheetView tabSelected="1" workbookViewId="0"/>`), so to insert a `<pane>` inside one
/// you must first expand the self-closed tag into open/close form; simply inserting before
/// `</sheetView>` would produce two `<sheetView>` elements and invalid XML.
fn patch_freeze_panes_str(xml: &[u8], rows: u32, cols: u16) -> Result<Vec<u8>> {
    let s = String::from_utf8(xml.to_vec())
        .map_err(|e| AppError::Custom(format!("XML is not valid UTF-8: {}", e)))?;
    let mut result = s;

    // 1) Remove all existing <pane .../> or <pane ...></pane>
    loop {
        let start = match result.find("<pane") {
            Some(s) => s,
            None => break,
        };
        let after = &result[start..];
        let gt = match after.find('>') {
            Some(g) => g,
            None => break,
        };
        // Only treat it as self-closed if "/>" appears before '>'
        let end_rel = if after[..gt + 1].rfind("/>").is_some() {
            after.find("/>").unwrap() + 2
        } else if let Some(e) = after.find("</pane>") {
            e + "</pane>".len()
        } else {
            break;
        };
        result.replace_range(start..start + end_rel, "");
    }

    if rows == 0 && cols == 0 {
        // Clear freeze: pane already removed, return directly
        return Ok(result.into_bytes());
    }

    let top_left_cell = format!("{}{}", index_to_col(cols), rows + 1);
    let active_pane = match (rows > 0, cols > 0) {
        (true, true) => "bottomRight",
        (true, false) => "bottomLeft",
        (false, true) => "topRight",
        (false, false) => "bottomRight",
    };

    // Only emit the corresponding Split attribute when >0, to avoid invalid `Split="0"`
    let mut splits = String::new();
    if cols > 0 {
        splits.push_str(&format!("xSplit=\"{}\" ", cols));
    }
    if rows > 0 {
        splits.push_str(&format!("ySplit=\"{}\" ", rows));
    }

    let pane_xml = format!(
        "<pane {splits}topLeftCell=\"{top_left_cell}\" activePane=\"{active_pane}\" state=\"frozen\"/>"
    );

    // Insert <pane> inside the existing <sheetView>
    // Note: must match "<sheetView " (with a space), otherwise the outer container "<sheetViews>" matches first
    if let Some(pos) = result.find("<sheetView ") {
        let open_end = pos + result[pos..].find('>').unwrap_or(0);
        if open_end > 0 && result.as_bytes()[open_end - 1] == b'/' {
            // Self-closed <sheetView .../>  ->  <sheetView ...> pane </sheetView>
            // head takes [pos, open_end-1), excluding the self-closing '/', then restores a normal '>'
            let head = &result[pos..open_end - 1];
            let repl = format!("{head}>\n      {pane_xml}\n    </sheetView>");
            result = format!("{}{}{}", &result[..pos], repl, &result[open_end + 1..]);
        } else {
            // Open tag <sheetView ...>: insert before the matching </sheetView>
            if let Some(close_rel) = result[pos..].find("</sheetView>") {
                let close = pos + close_rel;
                result.insert_str(close, &format!("\n      {pane_xml}"));
            }
        }
        return Ok(result.into_bytes());
    }

    // No <sheetView> but there is <sheetViews>
    if let Some(pos) = result.find("<sheetViews") {
        let open_end = pos + result[pos..].find('>').unwrap_or(0);
        if open_end > 0 && result.as_bytes()[open_end - 1] == b'/' {
            let head = &result[pos..open_end - 1];
            let new_sv = format!(
                "{head}>\n    <sheetView tabSelected=\"1\" workbookViewId=\"0\">\n      {pane_xml}\n    </sheetView>\n  </sheetViews>"
            );
            result = format!("{}{}{}", &result[..pos], new_sv, &result[open_end + 1..]);
        }
        return Ok(result.into_bytes());
    }

    // No sheetViews at all -> insert the full structure before </worksheet>
    if let Some(pos) = result.find("</worksheet>") {
        let new_xml = format!(
            "  <sheetViews>\n    <sheetView tabSelected=\"1\" workbookViewId=\"0\">\n      {pane_xml}\n    </sheetView>\n  </sheetViews>\n"
        );
        result.insert_str(pos, &new_xml);
    }

    Ok(result.into_bytes())
}

/// Preserving auto filter set.
pub fn set_auto_filter_preserving(
    path: &str,
    params: &SecurityParams,
    sheet: &str,
    range_ref: &str,
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
    let part = resolve_sheet_part(&mut archive, sheet)?;
    let sheet_xml = read_zip_entry(&mut archive, &part)?;

    let new_sheet_xml = patch_auto_filter_str(&sheet_xml, Some(range_ref))?;
    repackage_zip(&mut archive, path, &part, &new_sheet_xml)?;
    let new_hash = compute_file_hash(path).map_err(AppError::Io)?;
    append_history(path, "set_auto_filter", &old_hash, &new_hash, params.dry_run);

    Ok(WriteResult {
        success: true,
        message: String::new(),
        backup_info,
        old_hash,
        new_hash,
        diff: None,
    })
}

/// Preserving auto filter removal.
pub fn remove_auto_filter_preserving(
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
    let part = resolve_sheet_part(&mut archive, sheet)?;
    let sheet_xml = read_zip_entry(&mut archive, &part)?;

    let new_sheet_xml = patch_auto_filter_str(&sheet_xml, None)?;
    repackage_zip(&mut archive, path, &part, &new_sheet_xml)?;
    let new_hash = compute_file_hash(path).map_err(AppError::Io)?;
    append_history(path, "remove_auto_filter", &old_hash, &new_hash, params.dry_run);

    Ok(WriteResult {
        success: true,
        message: String::new(),
        backup_info,
        old_hash,
        new_hash,
        diff: None,
    })
}

/// Patches the autoFilter element using string operations.
fn patch_auto_filter_str(xml: &[u8], new_range: Option<&str>) -> Result<Vec<u8>> {
    let s = String::from_utf8(xml.to_vec())
        .map_err(|e| AppError::Custom(format!("XML is not valid UTF-8: {}", e)))?;
    let mut result = s;

    // Remove all existing <autoFilter .../> elements
    // Note: search for the terminator from the match position, otherwise any `/>` earlier in the file breaks the check
    loop {
        let start = match result.find("<autoFilter ") {
            Some(s) => s,
            None => break,
        };
        match result[start..].find("/>") {
            Some(rel) => {
                let af_end = start + rel + 2;
                result.replace_range(start..af_end, "");
            }
            None => break,
        }
    }

    // Remove all <autoFilter>...</autoFilter> blocks
    loop {
        let start = match result.find("<autoFilter ") {
            Some(s) => s,
            None => break,
        };
        match result[start..].find("</autoFilter>") {
            Some(rel) => {
                let af_end = start + rel + 13;
                result.replace_range(start..af_end, "");
            }
            None => break,
        }
    }

    // If a new range is provided, insert it
    if let Some(range) = new_range {
        if let Some(pos) = result.find("</worksheet>") {
            result.insert_str(pos, &format!("  <autoFilter ref=\"{}\"/>\n", range));
        }
    }

    Ok(result.into_bytes())
}

/// Preserving data validation add.
pub fn add_data_validation_preserving(
    path: &str,
    params: &SecurityParams,
    sheet: &str,
    config: &DataValidationConfig,
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
    let part = resolve_sheet_part(&mut archive, sheet)?;
    let sheet_xml = read_zip_entry(&mut archive, &part)?;

    let dv_xml = build_data_validation_xml_str(config);
    let new_sheet_xml = patch_data_validation_str(&sheet_xml, &dv_xml)?;

    repackage_zip(&mut archive, path, &part, &new_sheet_xml)?;
    let new_hash = compute_file_hash(path).map_err(AppError::Io)?;
    append_history(path, "add_data_validation", &old_hash, &new_hash, params.dry_run);

    Ok(WriteResult {
        success: true,
        message: String::new(),
        backup_info,
        old_hash,
        new_hash,
        diff: None,
    })
}

/// Builds the `<dataValidation>` XML element string.
fn build_data_validation_xml_str(config: &DataValidationConfig) -> String {
    let type_attr = match config.validation_type {
        DataValidationType::Whole => "whole",
        DataValidationType::Decimal => "decimal",
        DataValidationType::List => "list",
        DataValidationType::Date => "date",
        DataValidationType::Time => "time",
        DataValidationType::TextLength => "textLength",
        DataValidationType::Custom => "custom",
    };

    let mut dv = format!(
        "<dataValidation type=\"{}\" allowBlank=\"{}\" sqref=\"{}\"",
        type_attr,
        if config.allow_blank { "1" } else { "0" },
        config.range
    );

    if !config.show_dropdown {
        dv.push_str(" showDropDown=\"1\"");
    }

    let mut inner = String::new();
    if let (DataValidationType::List, Some(values)) = (&config.validation_type, &config.list_values) {
        if !values.is_empty() {
            let quoted: Vec<String> = values.iter()
                .map(|v| format!("\"{}\"", v))
                .collect();
            inner.push_str(&format!("<formula1>{}</formula1>", quoted.join(",")));
        }
    } else if let Some(f1) = &config.formula1 {
        inner.push_str(&format!("<formula1>{}</formula1>", f1));
    }
    if let Some(f2) = &config.formula2 {
        inner.push_str(&format!("<formula2>{}</formula2>", f2));
    }

    if inner.is_empty() {
        format!("{}/>", dv)
    } else {
        format!("{}>{}</dataValidation>", dv, inner)
    }
}

/// Patches the dataValidations element using string operations.
fn patch_data_validation_str(xml: &[u8], new_dv: &str) -> Result<Vec<u8>> {
    let s = String::from_utf8(xml.to_vec())
        .map_err(|e| AppError::Custom(format!("XML is not valid UTF-8: {}", e)))?;
    let mut result = s;

    if let Some(pos) = result.find("</dataValidations>") {
        // Existing <dataValidations> -> insert the new DV before the closing tag
        result.insert_str(pos, &format!("\n    {}", new_dv));
        // Update count
        let old_count = result.matches("<dataValidation ").count();
        let old_count_str = format!("count=\"{}\"", old_count - 1);
        let new_count_str = format!("count=\"{}\"", old_count);
        result = result.replacen(&old_count_str, &new_count_str, 1);
    } else if let Some(pos) = result.find("<dataValidations/>") {
        // Self-closed -> convert to open form
        let replacement = format!(
            "<dataValidations count=\"1\">\n    {}\n  </dataValidations>",
            new_dv
        );
        result.replace_range(pos..pos + 18, &replacement);
    } else {
        // Does not exist -> insert before </worksheet>
        if let Some(pos) = result.find("</worksheet>") {
            let new_entry = format!(
                "  <dataValidations count=\"1\">\n    {}\n  </dataValidations>\n",
                new_dv
            );
            result.insert_str(pos, &new_entry);
        }
    }

    Ok(result.into_bytes())
}

/// Preserving sheet protection set.
pub fn protect_sheet_preserving(
    path: &str,
    params: &SecurityParams,
    sheet: &str,
    config: &SheetProtectionConfig,
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
    let part = resolve_sheet_part(&mut archive, sheet)?;
    let sheet_xml = read_zip_entry(&mut archive, &part)?;

    let sp_xml = build_sheet_protection_xml_str(config);
    let new_sheet_xml = patch_sheet_protection_str(&sheet_xml, Some(&sp_xml))?;

    repackage_zip(&mut archive, path, &part, &new_sheet_xml)?;
    let new_hash = compute_file_hash(path).map_err(AppError::Io)?;
    append_history(path, "protect_sheet", &old_hash, &new_hash, params.dry_run);

    Ok(WriteResult {
        success: true,
        message: String::new(),
        backup_info,
        old_hash,
        new_hash,
        diff: None,
    })
}

/// Preserving sheet protection removal.
pub fn unprotect_sheet_preserving(
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
    let part = resolve_sheet_part(&mut archive, sheet)?;
    let sheet_xml = read_zip_entry(&mut archive, &part)?;

    let new_sheet_xml = patch_sheet_protection_str(&sheet_xml, None)?;
    repackage_zip(&mut archive, path, &part, &new_sheet_xml)?;
    let new_hash = compute_file_hash(path).map_err(AppError::Io)?;
    append_history(path, "unprotect_sheet", &old_hash, &new_hash, params.dry_run);

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

    let drawing_file = drawing_part.split('/').last().unwrap_or("drawing1.xml");
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
fn build_sheet_protection_xml_str(config: &SheetProtectionConfig) -> String {
    let opts = &config.options;
    format!(
        "<sheetProtection sheet=\"1\" password=\"{}\" selectLockedCells=\"{}\" selectUnlockedCells=\"{}\" \
         formatCells=\"{}\" formatColumns=\"{}\" formatRows=\"{}\" \
         insertColumns=\"{}\" insertRows=\"{}\" insertHyperlinks=\"{}\" \
         deleteColumns=\"{}\" deleteRows=\"{}\" \
         sort=\"{}\" autoFilter=\"{}\" pivotTables=\"{}\" \
         objects=\"{}\" scenarios=\"{}\"/>",
        config.password.as_deref().unwrap_or(""),
        bool_to_01(opts.select_locked_cells), bool_to_01(opts.select_unlocked_cells),
        bool_to_01(opts.format_cells), bool_to_01(opts.format_columns), bool_to_01(opts.format_rows),
        bool_to_01(opts.insert_columns), bool_to_01(opts.insert_rows), bool_to_01(opts.insert_links),
        bool_to_01(opts.delete_columns), bool_to_01(opts.delete_rows),
        bool_to_01(opts.sort), bool_to_01(opts.auto_filter), bool_to_01(opts.pivot_tables),
        bool_to_01(opts.edit_objects), bool_to_01(opts.edit_scenarios),
    )
}

fn bool_to_01(b: bool) -> &'static str {
    if b { "1" } else { "0" }
}

/// Patches the sheetProtection element using string operations.
fn patch_sheet_protection_str(xml: &[u8], new_sp: Option<&str>) -> Result<Vec<u8>> {
    let s = String::from_utf8(xml.to_vec())
        .map_err(|e| AppError::Custom(format!("XML is not valid UTF-8: {}", e)))?;
    let mut result = s;

    // Remove all existing <sheetProtection .../> elements
    loop {
        let start = result.find("<sheetProtection ");
        let end = result.find("/>");
        match (start, end) {
            (Some(s), Some(e)) if s < e => {
                let sp_end = e + 2;
                result.replace_range(s..sp_end, "");
            }
            _ => break,
        }
    }

    // Remove all <sheetProtection>...</sheetProtection> blocks
    loop {
        let start = result.find("<sheetProtection ");
        let end = result.find("</sheetProtection>");
        match (start, end) {
            (Some(s), Some(e)) if s < e => {
                let sp_end = e + 18;
                result.replace_range(s..sp_end, "");
            }
            _ => break,
        }
    }

    // If new protection is provided, insert it before </worksheet>
    if let Some(sp) = new_sp {
        if let Some(pos) = result.find("</worksheet>") {
            result.insert_str(pos, &format!("  {}\n", sp));
        }
    }

    Ok(result.into_bytes())
}

/// Preserving page setup — not implemented yet, deferred to Phase 3.
pub fn configure_page_setup_preserving(
    _path: &str,
    _params: &SecurityParams,
    _sheet: &str,
    _config: &PageSetupConfig,
) -> Result<WriteResult> {
    Err(AppError::Custom(
        "configure_page_setup_preserving not implemented yet - use full transfer fallback".into()
    ))
}

/// Preserving sheet visibility set: modifies the `state` attribute of the corresponding
/// sheet in workbook.xml.
pub fn set_sheet_visibility_preserving(
    path: &str,
    params: &SecurityParams,
    sheet_name: &str,
    visibility: &SheetVisibility,
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

    let wb_part = "xl/workbook.xml";
    let wb_xml = read_zip_entry(&mut archive, wb_part)?;

    let state_attr = match visibility {
        SheetVisibility::Visible => None,
        SheetVisibility::Hidden => Some("hidden"),
        SheetVisibility::VeryHidden => Some("veryHidden"),
    };

    let new_wb_xml = patch_sheet_visibility_str(&wb_xml, sheet_name, state_attr)?;

    repackage_zip(&mut archive, path, wb_part, &new_wb_xml)?;
    let new_hash = compute_file_hash(path).map_err(AppError::Io)?;
    append_history(path, "set_sheet_visibility", &old_hash, &new_hash, params.dry_run);

    Ok(WriteResult {
        success: true,
        message: String::new(),
        backup_info,
        old_hash,
        new_hash,
        diff: None,
    })
}

/// Patches sheet visibility using string operations.
fn patch_sheet_visibility_str(wb_xml: &[u8], sheet_name: &str, state: Option<&str>) -> Result<Vec<u8>> {
    let s = String::from_utf8(wb_xml.to_vec())
        .map_err(|e| AppError::Custom(format!("XML is not valid UTF-8: {}", e)))?;
    let mut result = s;

    // Find the target sheet's <sheet> tag and modify its state attribute
    let marker = format!("name=\"{}\"", sheet_name);
    if let Some(name_pos) = result.find(&marker) {
        // Search backwards for <sheet
        let prefix = &result[..name_pos];
        let tag_start = prefix.rfind("<sheet").ok_or_else(|| {
            AppError::Custom(format!("sheet tag not found: {}", sheet_name))
        })?;

        // From tag_start, find the tag end (> or />)
        let tag_end = result[tag_start..].find('>').ok_or_else(|| {
            AppError::Custom("cannot find sheet tag end".to_string())
        })? + tag_start;

        let tag = &result[tag_start..=tag_end];

        // Build the new tag
        let mut new_tag = format!("<sheet name=\"{}\"", sheet_name);

        // Extract sheetId and r:id
        if let Some(sid_start) = tag.find("sheetId=\"") {
            let sid_rest = &tag[sid_start + 9..];
            if let Some(sid_end) = sid_rest.find('"') {
                new_tag.push_str(&format!(" sheetId=\"{}\"", &sid_rest[..sid_end]));
            }
        }
        if let Some(rid_start) = tag.find("r:id=\"") {
            let rid_rest = &tag[rid_start + 6..];
            if let Some(rid_end) = rid_rest.find('"') {
                new_tag.push_str(&format!(" r:id=\"{}\"", &rid_rest[..rid_end]));
            }
        }

        // Add the state attribute
        if let Some(s) = state {
            new_tag.push_str(&format!(" state=\"{}\"", s));
        }

        new_tag.push(if tag.ends_with("/>") { '/' } else { ' ' });
        new_tag.push('>');

        result.replace_range(tag_start..=tag_end, &new_tag);
        Ok(result.into_bytes())
    } else {
        Err(AppError::SheetNotFound(sheet_name.into()))
    }
}

// ───────────────────────────────────────────────────────────────────────────
// R2.2 — add / delete / rename sheet preserving
// ───────────────────────────────────────────────────────────────────────────

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

/// Extracts a sheet's rid from the workbook.xml string.
fn extract_sheet_rid_str(wb: &[u8], sheet: &str) -> Result<String> {
    let s = String::from_utf8_lossy(wb);
    let marker = format!("name=\"{}\"", sheet);
    if let Some(name_pos) = s.find(&marker) {
        let prefix = &s[..name_pos];
        let tag_start = prefix.rfind("<sheet").ok_or_else(|| {
            AppError::Custom(format!("sheet tag not found: {}", sheet))
        })?;
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

/// Extracts the Target corresponding to `rid` from the rels XML string.
fn find_rel_target_str(rels: &[u8], rid: &str) -> Result<String> {
    let s = String::from_utf8_lossy(rels);
    let marker = format!("Id=\"{}\"", rid);
    if let Some(id_pos) = s.find(&marker) {
        let prefix = &s[..id_pos];
        let tag_start = prefix.rfind("<Relationship").ok_or_else(|| {
            AppError::Custom(format!("Relationship tag not found: {}", rid))
        })?;
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
        Err(AppError::Custom(format!("Target not found for rid {}", rid)))
    } else {
        Err(AppError::Custom(format!("rid not found: {}", rid)))
    }
}

/// Determines the next sheet number (max value found in the existing zip entries + 1).
fn next_sheet_number(wb_xml: &str) -> u32 {
    let mut max_n = 0u32;
    // Find all sheetId="N" patterns
    let mut pos = 0;
    while let Some(start) = wb_xml[pos..].find("sheetId=\"") {
        let rest = &wb_xml[pos + start + 9..];
        if let Some(end) = rest.find('"') {
            if let Ok(n) = rest[..end].parse::<u32>() {
                if n > max_n { max_n = n; }
            }
        }
        pos += start + 1;
    }
    max_n + 1
}

/// Determines the next rId (max rIdN value found in workbook.xml + 1).
fn next_rid(wb_xml: &str) -> String {
    let mut max_n = 0u32;
    let mut pos = 0;
    while let Some(start) = wb_xml[pos..].find("r:id=\"rId") {
        let rest = &wb_xml[pos + start + 9..];
        if let Some(end) = rest.find('"') {
            if let Ok(n) = rest[..end].parse::<u32>() {
                if n > max_n { max_n = n; }
            }
        }
        pos += start + 1;
    }
    format!("rId{}", max_n + 1)
}

/// Determines the next sheetId.
fn next_sheet_id(wb_xml: &str) -> u32 {
    next_sheet_number(wb_xml)
}

/// String-based: appends <sheet> to workbook.xml.
fn patch_add_sheet_str(wb: &[u8], sheet: &str, rid: &str, sheet_id: u32) -> Result<Vec<u8>> {
    let s = String::from_utf8(wb.to_vec())
        .map_err(|e| AppError::Custom(format!("XML is not valid UTF-8: {}", e)))?;
    let mut result = s;

    // Insert before </sheets>
    if let Some(pos) = result.find("</sheets>") {
        let new_sheet = format!("\n    <sheet name=\"{}\" sheetId=\"{}\" r:id=\"{}\"/>", sheet, sheet_id, rid);
        result.insert_str(pos, &new_sheet);
        Ok(result.into_bytes())
    } else {
        Err(AppError::Custom("cannot find </sheets> in workbook.xml".to_string()))
    }
}

/// String-based: removes <sheet> from workbook.xml.
fn patch_remove_sheet_str(wb: &[u8], sheet: &str) -> Result<Vec<u8>> {
    let s = String::from_utf8(wb.to_vec())
        .map_err(|e| AppError::Custom(format!("XML is not valid UTF-8: {}", e)))?;
    let mut result = s;

    let marker = format!("name=\"{}\"", sheet);
    if let Some(name_pos) = result.find(&marker) {
        let prefix = &result[..name_pos];
        let tag_start = prefix.rfind("<sheet").ok_or_else(|| {
            AppError::Custom(format!("sheet tag not found: {}", sheet))
        })?;
        // Find the tag end: > or />
        let rest = &result[tag_start..];
        let tag_end = rest.find('>').ok_or_else(|| {
            AppError::Custom("sheet tag end not found".to_string())
        })? + tag_start + 1;
        result.replace_range(tag_start..tag_end, "");
        Ok(result.into_bytes())
    } else {
        Err(AppError::SheetNotFound(sheet.into()))
    }
}

/// String-based: renames a sheet in workbook.xml.
fn patch_rename_sheet_str(wb: &[u8], old_name: &str, new_name: &str) -> Result<Vec<u8>> {
    let s = String::from_utf8(wb.to_vec())
        .map_err(|e| AppError::Custom(format!("XML is not valid UTF-8: {}", e)))?;
    let mut result = s;

    let old_marker = format!("name=\"{}\"", old_name);
    let new_marker = format!("name=\"{}\"", new_name);
    result = result.replacen(&old_marker, &new_marker, 1);

    Ok(result.into_bytes())
}

/// String-based: appends an Override to [Content_Types].xml.
fn patch_add_content_type_str(ct: &[u8], part_name: &str) -> Result<Vec<u8>> {
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
        Err(AppError::Custom("cannot find </Types> in [Content_Types].xml".to_string()))
    }
}

/// String-based: removes an Override from [Content_Types].xml.
fn patch_remove_content_type_str(ct: &[u8], part: &str) -> Result<Vec<u8>> {
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
        let tag_start = prefix.rfind("<Override").ok_or_else(|| {
            AppError::Custom(format!("Override tag not found: {}", part))
        })?;
        let rest = &result[tag_start..];
        let tag_end = rest.find("/>").ok_or_else(|| {
            AppError::Custom("Override tag end not found".to_string())
        })? + tag_start + 2;
        result.replace_range(tag_start..tag_end, "");
        Ok(result.into_bytes())
    } else {
        // Not found is fine, continue
        Ok(ct.to_vec())
    }
}

/// String-based: appends a Relationship to workbook.xml.rels.
fn patch_add_sheet_rel_str(rels: &[u8], rid: &str, target: &str) -> Result<Vec<u8>> {
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
        Err(AppError::Custom("cannot find </Relationships> in rels".to_string()))
    }
}

/// String-based: removes a Relationship from rels.
fn patch_remove_rel_str(rels: &[u8], rid: &str) -> Result<Vec<u8>> {
    let s = String::from_utf8(rels.to_vec())
        .map_err(|e| AppError::Custom(format!("XML is not valid UTF-8: {}", e)))?;
    let mut result = s;

    let marker = format!("Id=\"{}\"", rid);
    if let Some(id_pos) = result.find(&marker) {
        let prefix = &result[..id_pos];
        let tag_start = prefix.rfind("<Relationship").ok_or_else(|| {
            AppError::Custom(format!("Relationship tag not found: {}", rid))
        })?;
        let rest = &result[tag_start..];
        let tag_end = rest.find("/>").ok_or_else(|| {
            AppError::Custom("cannot find Relationship tag end".to_string())
        })? + tag_start + 2;
        result.replace_range(tag_start..tag_end, "");
        Ok(result.into_bytes())
    } else {
        Err(AppError::Custom(format!("rid not found: {}", rid)))
    }
}

// ───────────────────────────────────────────────────────────────────────────
// Internal helpers
// ───────────────────────────────────────────────────────────────────────────

/// Copies every zip entry except `part` byte-for-byte and replaces `part`'s content with `new_xml`.
fn repackage_zip(
    archive: &mut ZipArchive<File>,
    path: &str,
    part: &str,
    new_xml: &[u8],
) -> Result<()> {
    let tmp = Path::new(path).with_extension("patch_tmp");
    let tf = File::create(&tmp).map_err(AppError::Io)?;
    let mut zw = ZipWriter::new(tf);
    let mut buf = Vec::new();
    let n = archive.len();
    for i in 0..n {
        let mut zf = archive
            .by_index(i)
            .map_err(|e| AppError::Custom(format!("failed to read zip entry: {}", e)))?;
        let name = zf.name().to_string();
        let opts = zf.options();
        if name == part {
            zw.start_file(&name, opts)
                .map_err(|e| AppError::Custom(format!("failed to write zip entry: {}", e)))?;
            zw.write_all(new_xml).map_err(AppError::Io)?;
        } else {
            buf.clear();
            zf.read_to_end(&mut buf).map_err(AppError::Io)?;
            zw.start_file(&name, opts)
                .map_err(|e| AppError::Custom(format!("failed to write zip entry: {}", e)))?;
            zw.write_all(&buf).map_err(AppError::Io)?;
        }
    }
    zw.finish()
        .map_err(|e| AppError::Custom(format!("failed to finish zip write: {}", e)))?;
    fs::rename(&tmp, path).map_err(AppError::Io)?;
    Ok(())
}

/// Multi-entry variant of repackage: supports modifying/adding multiple parts at once
/// and skipping specified entries.
///
/// - `changes`: entry names to modify or add -> new content
/// - `skip_parts`: entry names to skip (not written to the new zip)
fn repackage_zip_multi(
    archive: &mut ZipArchive<File>,
    path: &str,
    changes: &HashMap<String, Vec<u8>>,
    skip_parts: &[String],
) -> Result<()> {
    let tmp = Path::new(path).with_extension("patch_tmp");
    let tf = File::create(&tmp).map_err(AppError::Io)?;
    let mut zw = ZipWriter::new(tf);
    #[cfg(feature = "flate2")]
    let default_opt = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    #[cfg(not(feature = "flate2"))]
    let default_opt = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Stored);
    let mut buf = Vec::new();
    let n = archive.len();

    // Pre-record all entries that need to be added (not present in the original zip)
    let mut added = std::collections::HashSet::new();

    for i in 0..n {
        let mut zf = archive
            .by_index(i)
            .map_err(|e| AppError::Custom(format!("failed to read zip entry: {}", e)))?;
        let name = zf.name().to_string();
        let opts = zf.options();

        // Check whether it is in the skip list
        if skip_parts.iter().any(|sp| *sp == name) {
            continue;
        }

        // Check whether there is a change
        if let Some(new_content) = changes.get(&name) {
            zw.start_file(&name, opts)
                .map_err(|e| AppError::Custom(format!("failed to write zip entry: {}", e)))?;
            zw.write_all(new_content).map_err(AppError::Io)?;
            added.insert(name);
        } else {
            buf.clear();
            zf.read_to_end(&mut buf).map_err(AppError::Io)?;
            zw.start_file(&name, opts)
                .map_err(|e| AppError::Custom(format!("failed to write zip entry: {}", e)))?;
            zw.write_all(&buf).map_err(AppError::Io)?;
        }
    }

    // Write the added entries (not present in the original zip)
    for (name, content) in changes {
        if !added.contains(name) {
            // Use default options
            zw.start_file(name, default_opt)
                .map_err(|e| AppError::Custom(format!("failed to write new zip entry: {}", e)))?;
            zw.write_all(content).map_err(AppError::Io)?;
        }
    }

    zw.finish()
        .map_err(|e| AppError::Custom(format!("failed to finish zip write: {}", e)))?;
    fs::rename(&tmp, path).map_err(AppError::Io)?;
    Ok(())
}

/// Records a write operation to the audit history (non-fatal).
fn append_history(path: &str, op: &str, old_hash: &str, new_hash: &str, dry_run: bool) {
    if dry_run {
        return;
    }
    let entry = WorkbookHistoryEntry {
        timestamp: chrono::Utc::now(),
        operation_type: op.to_string(),
        target_path: path.to_string(),
        old_hash: old_hash.to_string(),
        new_hash: new_hash.to_string(),
        result: "success".to_string(),
    };
    let _ = append_history_entry(path, &entry);
}

/// Determines whether the cell's raw bytes contain `<f` (formula marker).
fn has_formula(raw: &[u8]) -> bool {
    raw.windows(3).any(|w| w == b"<f>") || raw.windows(4).any(|w| w == b"<f ")
}

/// Removes the `<v>...</v>` element from the cell's raw bytes.
fn strip_v_element(raw: &[u8]) -> Vec<u8> {
    // Locate the start of <v or <v>
    let mut result = Vec::with_capacity(raw.len());
    let mut i = 0;
    while i < raw.len() {
        if (raw[i..].starts_with(b"<v>") || raw[i..].starts_with(b"<v ")) && i > 0 {
            // Confirm going backwards this is not part of <f> or another tag
            let prev = raw[i - 1];
            if prev == b'>' || prev == b'/' || prev == b'"' || prev == b'\'' {
                // Skip the <v...> tag
                i += 2; // skip "<v"
                while i < raw.len() && raw[i] != b'>' {
                    i += 1;
                }
                i += 1; // skip '>'
                // Skip content until </v>
                while i + 4 < raw.len() && !raw[i..].starts_with(b"</v>") {
                    i += 1;
                }
                // Skip </v> or </v...>
                if i + 4 <= raw.len() && raw[i..].starts_with(b"</v>") {
                    i += 4;
                } else {
                    // Abnormal case: no match, let later bytes pass through normally
                    // This rolls back too far; switch to a more conservative strategy
                    break;
                }
                continue;
            }
        }
        result.push(raw[i]);
        i += 1;
    }
    // If the loop above broke out due to an abnormal case, fall back to the safe path:
    // return the original bytes directly
    if i < raw.len() {
        return raw.to_vec();
    }
    result
}

// ───────────────────────────────────────────────────────────────────────────
// sheet name -> zip part resolution
// ───────────────────────────────────────────────────────────────────────────

fn read_zip_entry(archive: &mut ZipArchive<File>, name: &str) -> Result<Vec<u8>> {
    let mut zf = archive
        .by_name(name)
        .map_err(|e| AppError::Custom(format!("missing zip entry {}: {}", name, e)))?;
    let mut buf = Vec::new();
    zf.read_to_end(&mut buf).map_err(AppError::Io)?;
    Ok(buf)
}

/// Maps a sheet name to `xl/worksheets/sheetN.xml` via `xl/workbook.xml` +
/// `xl/_rels/workbook.xml.rels`.
fn resolve_sheet_part(archive: &mut ZipArchive<File>, sheet: &str) -> Result<String> {
    let wb = read_zip_entry(archive, "xl/workbook.xml")?;
    let rid = find_sheet_rid(&wb, sheet).ok_or_else(|| {
        AppError::Custom(format!("sheet '{}' not found in workbook", sheet))
    })?;
    let rels = read_zip_entry(archive, "xl/_rels/workbook.xml.rels")?;
    let target = find_rel_target(&rels, &rid)
        .ok_or_else(|| AppError::Custom(format!("target not found for relationship {}", rid)))?;
    // Relationship Target has two real forms, both must be normalized to a zip entry name
    // (no leading slash, starts with xl/):
    //   - Absolute path (produced by some tools, with or without leading slash): /xl/worksheets/sheet1.xml
    //   - Relative path (relative to xl/_rels/): worksheets/sheet1.xml or xl/worksheets/sheet1.xml
    // Note: zip entry names have no leading slash; returning "/xl/..." directly would make
    // by_name miss the entry, causing the write to error early and the value not to land
    // (observed as cell write then read back as None).
    let trimmed = target
        .strip_prefix('/')
        .or_else(|| target.strip_prefix("xl/"))
        .unwrap_or(&target);
    let part = if trimmed.starts_with("xl/") {
        trimmed.to_string()
    } else {
        format!("xl/{}", trimmed)
    };
    Ok(part)
}

fn find_sheet_rid(wb: &[u8], sheet: &str) -> Option<String> {
    let mut reader = Reader::from_reader(Cursor::new(wb));
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                // Note: quick-xml events expose only the inner tag content via Deref
                // (e.g. `sheet name=...`), without `<`, so match the prefix `sheet` not `<sheet`.
                let raw: &[u8] = &e;
                if raw.starts_with(b"sheet") {
                    if extract_attr(raw, b"name").as_deref() == Some(sheet) {
                        return extract_attr(raw, b"r:id");
                    }
                }
            }
            Ok(Event::Eof) => break,
            Ok(_) => {}
            Err(_) => break,
        }
    }
    None
}

fn find_rel_target(rels: &[u8], rid: &str) -> Option<String> {
    let mut reader = Reader::from_reader(Cursor::new(rels));
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                // Same as above: deref content is `Relationship Id=...`, without `<`.
                let raw: &[u8] = &e;
                if raw.starts_with(b"Relationship") {
                    if extract_attr(raw, b"Id").as_deref() == Some(rid) {
                        return extract_attr(raw, b"Target");
                    }
                }
            }
            Ok(Event::Eof) => break,
            Ok(_) => {}
            Err(_) => break,
        }
    }
    None
}

/// Finds the Id of the first Relationship whose Type ends with `type_suffix`.
fn find_rel_by_type(rels: &[u8], type_suffix: &str) -> Option<String> {
    let mut reader = Reader::from_reader(Cursor::new(rels));
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                let raw: &[u8] = &e;
                if raw.starts_with(b"Relationship") {
                    if let Some(ty) = extract_attr(raw, b"Type") {
                        if ty.ends_with(type_suffix) {
                            return extract_attr(raw, b"Id");
                        }
                    }
                }
            }
            Ok(Event::Eof) => break,
            Ok(_) => {}
            Err(_) => break,
        }
    }
    None
}

/// Finds the Targets of all Relationships whose Type ends with `type_suffix`.
fn find_all_rel_targets_by_type(rels: &[u8], type_suffix: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut reader = Reader::from_reader(Cursor::new(rels));
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                let raw: &[u8] = &e;
                if raw.starts_with(b"Relationship") {
                    if let (Some(ty), Some(tgt)) =
                        (extract_attr(raw, b"Type"), extract_attr(raw, b"Target"))
                    {
                        if ty.ends_with(type_suffix) {
                            out.push(tgt);
                        }
                    }
                }
            }
            Ok(Event::Eof) => break,
            Ok(_) => {}
            Err(_) => break,
        }
    }
    out
}

/// Normalizes a relative (possibly containing `..`) or absolute Target to a zip entry name
/// (starts with `xl/`, no leading slash).
fn normalize_rel_target(base_part: &str, target: &str) -> String {
    let t = target.trim_start_matches('/');
    if t.starts_with("xl/") {
        return t.to_string();
    }
    let base_dir: Vec<&str> = base_part.split('/').filter(|s| !s.is_empty()).collect();
    // base_dir's last segment is the file name; drop it to get the containing directory
    let mut segs: Vec<&str> = base_dir[..base_dir.len().saturating_sub(1)].to_vec();
    for seg in t.split('/') {
        if seg == ".." {
            segs.pop();
        } else if seg.is_empty() || seg == "." {
            // Skip
        } else {
            segs.push(seg);
        }
    }
    segs.join("/")
}

/// Deletes all `<Relationship .../>` elements with Id == `rid` (keeps the other relationships
/// and the closing tag).
fn remove_rel_by_id(rels: &[u8], rid: &str) -> String {
    let mut result = String::from_utf8_lossy(rels).into_owned();
    let mut spans: Vec<(usize, usize)> = Vec::new();
    let mut search_from = 0;
    // Note: must match "<Relationship " (with a space), otherwise the container tag "<Relationships>" matches first
    while let Some(rel_start) = result[search_from..].find("<Relationship ") {
        let start = search_from + rel_start;
        let rest = &result[start..];
        let end_rel = if let Some(e) = rest.find("/>") {
            e + 2
        } else if let Some(e) = rest.find("</Relationship>") {
            e + "</Relationship>".len()
        } else {
            break;
        };
        let end = start + end_rel;
        let elem = &result[start..end];
        if extract_attr(elem.as_bytes(), b"Id").as_deref() == Some(rid) {
            let mut tail = end;
            while tail < result.len()
                && matches!(result.as_bytes()[tail], b'\n' | b' ' | b'\r' | b'\t')
            {
                tail += 1;
            }
            spans.push((start, tail));
        }
        search_from = end;
    }
    // Delete right-to-left so indexes stay valid
    for (s, e) in spans.iter().rev() {
        result.replace_range(*s..*e, "");
    }
    result
}

/// Removes the `<drawing .../>` element with `r:id="rid"` from the sheet xml
/// (including trailing extra whitespace).
fn remove_drawing_elem(sheet_xml: &[u8], rid: &str) -> String {
    let mut result = String::from_utf8_lossy(sheet_xml).into_owned();
    let start = match result.find("<drawing") {
        Some(s) => s,
        None => return result,
    };
    let rest = &result[start..];
    let end_rel = if let Some(e) = rest.find("/>") {
        e + 2
    } else if let Some(e) = rest.find("</drawing>") {
        e + "</drawing>".len()
    } else {
        return result;
    };
    let elem = &result[start..start + end_rel];
    if extract_attr(elem.as_bytes(), b"r:id").as_deref() == Some(rid) {
        let mut end = start + end_rel;
        while end < result.len()
            && matches!(result.as_bytes()[end], b'\n' | b' ' | b'\r' | b'\t')
        {
            end += 1;
        }
        result.replace_range(start..end, "");
    }
    result
}

// ───────────────────────────────────────────────────────────────────────────
// sheetData "rewrite only" edits
// ───────────────────────────────────────────────────────────────────────────

/// (before, inner, after, self_closed)
/// - before: everything up to (and including) the `<sheetData ...>` start tag
///   (up to `<sheetData` when self-closed)
/// - inner: the cell data between `<sheetData>` and `</sheetData>`
/// - after: everything after `</sheetData>` (after `/>` when self-closed)
fn sheetdata_spans(xml: &[u8]) -> Option<(&[u8], &[u8], &[u8], bool)> {
    let s = xml.windows(10).position(|w| &w[..10] == b"<sheetData")?;
    let mut gt = s;
    while gt < xml.len() && xml[gt] != b'>' {
        gt += 1;
    }
    if gt >= xml.len() {
        return None;
    }
    if xml[gt - 1] == b'/' {
        // <sheetData/> self-closed: inner is empty
        let before = &xml[..s];
        let after = &xml[gt + 1..];
        return Some((before, &xml[gt + 1..gt + 1], after, true));
    }
    let inner_start = gt + 1;
    let close = xml[inner_start..]
        .windows(12)
        .position(|w| &w[..12] == b"</sheetData>")?
        + inner_start;
    let before = &xml[..gt + 1];
    let after = &xml[close..];
    let inner = &xml[inner_start..close];
    Some((before, inner, after, false))
}

#[derive(Default)]
struct SheetModel {
    rows: BTreeMap<u32, Row>,
}

struct Row {
    /// The raw start tag, e.g. `<row r="1" spans="1:26">` (normalized to an open tag ending with `>`)`
    open_tag: String,
    cells: BTreeMap<u16, Cell>,
}

struct Cell {
    /// Byte-for-byte original of `<c ...>...</c>` or `<c .../>` (replaced as a whole when editing)
    raw: Vec<u8>,
}

/// Rebuilds a quick-xml `Event` into full XML markup bytes.
///
/// quick-xml events expose only the tag **inner content** via `Deref` (e.g. `c r="A1" s="3"`),
/// without `<` `>`, so the brackets are reconstructed here manually to produce
/// `<c r="A1" s="3">` / `</c>` / `<c .../>` etc., for `starts_with` checks and byte-level
/// `cell_buf` rebuilding.
fn event_markup(event: &Event) -> Vec<u8> {
    // `Event` implements `Deref<Target = [u8]>`, giving the tag inner content (without brackets).
    let inner: &[u8] = event;
    match event {
        Event::Start(_) => {
            let mut v = Vec::with_capacity(inner.len() + 2);
            v.push(b'<');
            v.extend_from_slice(inner);
            v.push(b'>');
            v
        }
        Event::Empty(_) => {
            let mut v = Vec::with_capacity(inner.len() + 3);
            v.push(b'<');
            v.extend_from_slice(inner);
            v.extend_from_slice(b"/>");
            v
        }
        Event::End(_) => {
            let mut v = Vec::with_capacity(inner.len() + 3);
            v.extend_from_slice(b"</");
            v.extend_from_slice(inner);
            v.push(b'>');
            v
        }
        Event::CData(_) => {
            let mut v = Vec::with_capacity(inner.len() + 12);
            v.extend_from_slice(b"<![CDATA[");
            v.extend_from_slice(inner);
            v.extend_from_slice(b"]]>");
            v
        }
        // Others (Text / Comment / PI / Decl / DocType / Eof) returned as inner content as-is.
        _ => inner.to_vec(),
    }
}

fn parse_sheetdata(inner: &[u8]) -> SheetModel {
    let mut model = SheetModel::default();
    let mut reader = Reader::from_reader(Cursor::new(inner));
    let mut buf = Vec::new();

    let mut cur_row: Option<(u32, Row)> = None;
    let mut cur_col: u16 = 0;
    let mut in_cell: bool = false;
    let mut cell_buf: Vec<u8> = Vec::new();

    loop {
        let event = match reader.read_event_into(&mut buf) {
            Ok(e) => e,
            Err(_) => break,
        };
        let raw = event_markup(&event);
        let is_empty = matches!(event, Event::Empty(_));

        match &event {
            Event::Start(_) | Event::Empty(_) => {
                if raw.starts_with(b"<row") {
                    let mut open = String::from_utf8_lossy(&raw).into_owned();
                    if open.ends_with("/>") {
                        open.truncate(open.len() - 2);
                        open.push('>');
                    }
                    // Row number (1-based) -> 0-based as the map key
                    let rk = extract_attr(&raw, b"r")
                        .and_then(|s| s.parse::<u32>().ok())
                        .map(|n| n.saturating_sub(1))
                        .unwrap_or(0);
                    cur_row = Some((rk, Row { open_tag: open, cells: BTreeMap::new() }));
                } else if raw.starts_with(b"<c") {
                    let col = extract_col(&raw);
                    cur_col = col;
                    cell_buf.clear();
                    cell_buf.extend_from_slice(&raw);
                    if is_empty {
                        if let Some((_, row)) = &mut cur_row {
                            row.cells.insert(col, Cell { raw: cell_buf.clone() });
                        }
                    } else {
                        in_cell = true;
                    }
                } else if in_cell {
                    // `<f>` / `<v>` / `<is>` / `<t>` etc.: inner cell elements accumulated as-is
                    cell_buf.extend_from_slice(&raw);
                }
            }
            Event::End(_) => {
                if raw.starts_with(b"</row") {
                    if let Some((k, row)) = cur_row.take() {
                        model.rows.insert(k, row);
                    }
                } else if raw.starts_with(b"</c") {
                    cell_buf.extend_from_slice(&raw);
                    if let Some((_, row)) = &mut cur_row {
                        row.cells.insert(cur_col, Cell { raw: cell_buf.clone() });
                    }
                    in_cell = false;
                    cell_buf.clear();
                } else if in_cell {
                    cell_buf.extend_from_slice(&raw);
                }
            }
            Event::Text(_) | Event::CData(_) => {
                if in_cell {
                    cell_buf.extend_from_slice(&raw);
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }

    // Wrap-up: if the file has no explicit `</row>` before `</sheetData>` (non-conforming but occasional)
    if let Some((k, row)) = cur_row.take() {
        model.rows.insert(k, row);
    }

    model
}

fn apply_edits(mut model: SheetModel, edits: &[(u32, u16, CellData)]) -> SheetModel {
    for (row, col, cd) in edits {
        let rref = format!("{}{}", index_to_col(*col), *row + 1);
        match model.rows.get_mut(row) {
            Some(row_model) => {
                if let Some(cell) = row_model.cells.get_mut(col) {
                    // Existing cell: keep its style index `s`, rebuild the content as a whole
                    let style = extract_attr(&cell.raw, b"s");
                    cell.raw = rebuild_cell(&rref, style.as_deref(), cd).into_bytes();
                } else {
                    // New column in the same row: insert (no style)
                    row_model.cells.insert(
                        *col,
                        Cell {
                            raw: rebuild_cell(&rref, None, cd).into_bytes(),
                        },
                    );
                }
            }
            None => {
                // Row does not exist at all: create the row + cell
                let mut cells = BTreeMap::new();
                cells.insert(
                    *col,
                    Cell {
                        raw: rebuild_cell(&rref, None, cd).into_bytes(),
                    },
                );
                model.rows.insert(
                    *row,
                    Row {
                        open_tag: format!("<row r=\"{}\">", *row + 1),
                        cells,
                    },
                );
            }
        }
    }
    model
}

fn serialize_sheet(model: &SheetModel) -> String {
    let mut out = String::new();
    for (_rk, row) in &model.rows {
        out.push_str(&row.open_tag);
        for (_ck, cell) in &row.cells {
            out.push_str(&String::from_utf8_lossy(&cell.raw));
        }
        out.push_str("</row>");
    }
    out
}

/// Computes the `<dimension>` bounding-box reference from the final model (e.g. `A1:Z50`).
fn dimension_ref(model: &SheetModel) -> String {
    if model.rows.is_empty() {
        return "A1".to_string();
    }
    let min_r = model.rows.keys().next().map(|r| *r + 1).unwrap_or(1);
    let max_r = model.rows.keys().next_back().map(|r| *r + 1).unwrap_or(1);
    let mut min_c: u16 = u16::MAX;
    let mut max_c: u16 = 0;
    for row in model.rows.values() {
        for &c in row.cells.keys() {
            if c < min_c {
                min_c = c;
            }
            if c > max_c {
                max_c = c;
            }
        }
    }
    if min_c == u16::MAX {
        min_c = 0;
        max_c = 0;
    }
    format!(
        "{}{}:{}{}",
        index_to_col(min_c),
        min_r,
        index_to_col(max_c),
        max_r
    )
}

/// Replaces the ref in the existing `<dimension ref="..."/>`; if absent, inserts before `<sheetData`.
fn replace_dimension(before: &mut Vec<u8>, refstr: &str) {
    if let Some(start) = before.windows(10).position(|w| &w[..10] == b"<dimension") {
        if let Some(rel) = before[start..].windows(5).position(|w| &w[..5] == b"ref=\"") {
            let abs = start + rel + 5;
            if let Some(end) = before[abs..].iter().position(|&b| b == b'"') {
                let end = abs + end;
                let new = refstr.as_bytes();
                before.splice(abs..end, new.iter().cloned());
                return;
            }
        }
    }
    // No dimension element: insert before <sheetData (follows schema order).
    if let Some(pos) = before.windows(10).position(|w| &w[..10] == b"<sheetData") {
        let ins = format!("<dimension ref=\"{}\"/>", refstr);
        before.splice(pos..pos, ins.into_bytes());
    }
}

/// Rewrites only the inside of `<sheetData>`; other parts remain byte-identical.
fn patch_sheet_xml(xml: &[u8], edits: &[(u32, u16, CellData)]) -> Result<Vec<u8>> {
    let (before, inner, after, self_closed) = match sheetdata_spans(xml) {
        Some(x) => x,
        None => return Ok(xml.to_vec()),
    };
    let model = parse_sheetdata(inner);
    let model = apply_edits(model, edits);
    let new_inner = serialize_sheet(&model);

    // T5.16: update <dimension ref="..."> to the bounding box of the final cells, so the
    // dimension does not stay at the original range after writing far cells (e.g. Z50).
    let refstr = dimension_ref(&model);
    let mut before_vec = before.to_vec();
    replace_dimension(&mut before_vec, &refstr);

    let mut out = Vec::with_capacity(xml.len() + new_inner.len());
    if self_closed {
        out.extend_from_slice(&before_vec);
        out.extend_from_slice(b"<sheetData>");
        out.extend_from_slice(new_inner.as_bytes());
        out.extend_from_slice(b"</sheetData>");
        out.extend_from_slice(after);
    } else {
        out.extend_from_slice(&before_vec);
        out.extend_from_slice(new_inner.as_bytes());
        out.extend_from_slice(after);
    }
    Ok(out)
}

// ───────────────────────────────────────────────────────────────────────────
// Cell rebuilding (uses inline strings to avoid touching sharedStrings.xml)
// ───────────────────────────────────────────────────────────────────────────

/// Returns `(type t, inner XML)`. Strings uniformly use `inlineStr` so sharedStrings stays untouched.
///
/// When the value starts with `=`, it is recognized as a **formula** per Excel semantics
/// (CLI `cell write =SUM(...)` does not set the `formula` field explicitly); in that case the
/// leading `=` is stripped and written to `<f>`, with no cached value.
fn cell_xml(cd: &CellData) -> (Option<&'static str>, String) {
    // Resolve "formula / value": the formula field takes priority; otherwise a value starting
    // with "=" is treated as a formula.
    let (formula, cached): (Option<String>, &str) = if let Some(f) = &cd.formula {
        (Some(f.clone()), cd.value.as_deref().unwrap_or(""))
    } else if let Some(v) = &cd.value {
        if let Some(stripped) = v.strip_prefix('=') {
            (Some(stripped.to_string()), "")
        } else {
            (None, v)
        }
    } else {
        (None, "")
    };

    if let Some(f) = &formula {
        let inner = format!("<f>{}</f><v>{}</v>", escape(f), escape(cached));
        let t = if cached.parse::<f64>().is_ok() {
            Some("n")
        } else if cached.eq_ignore_ascii_case("TRUE") || cached.eq_ignore_ascii_case("FALSE") {
            Some("b")
        } else {
            Some("str")
        };
        (t, inner)
    } else if !cached.is_empty() {
        match cd.data_type {
            CellDataType::Float | CellDataType::Int | CellDataType::DateTime => {
                (Some("n"), format!("<v>{}</v>", escape(cached)))
            }
            CellDataType::Bool => {
                let b = if cached == "true" || cached == "1" || cached.eq_ignore_ascii_case("True")
                {
                    "1"
                } else {
                    "0"
                };
                (Some("b"), format!("<v>{}</v>", b))
            }
            CellDataType::Error => (Some("e"), format!("<v>{}</v>", escape(cached))),
            _ => (
                Some("inlineStr"),
                format!("<is><t>{}</t></is>", escape(cached)),
            ),
        }
    } else {
        (None, String::new())
    }
}

fn rebuild_cell(rref: &str, style: Option<&str>, cd: &CellData) -> String {
    let (t, inner) = cell_xml(cd);
    let mut s = String::new();
    s.push_str("<c r=\"");
    s.push_str(rref);
    s.push('"');
    if let Some(st) = style {
        s.push_str(" s=\"");
        s.push_str(st);
        s.push('"');
    }
    if let Some(ty) = t {
        s.push_str(" t=\"");
        s.push_str(ty);
        s.push('"');
    }
    if inner.is_empty() {
        s.push_str("/>");
    } else {
        s.push('>');
        s.push_str(&inner);
        s.push_str("</c>");
    }
    s
}

// ───────────────────────────────────────────────────────────────────────────
// Tiny utilities
// ───────────────────────────────────────────────────────────────────────────

/// Extracts the value of `key="value"` from the raw tag text (supports single/double quotes).
fn extract_attr(raw: &[u8], key: &[u8]) -> Option<String> {
    let mut i = 0;
    while i + key.len() + 1 < raw.len() {
        if &raw[i..i + key.len()] == key && raw[i + key.len()] == b'=' {
            let q = raw[i + key.len() + 1];
            if q == b'"' || q == b'\'' {
                let start = i + key.len() + 2;
                if let Some(rel) = raw[start..].iter().position(|&b| b == q) {
                    let end = start + rel;
                    return Some(String::from_utf8_lossy(&raw[start..end]).into_owned());
                }
            }
        }
        i += 1;
    }
    None
}

/// Parses the 0-based column index from the `<c r="A1">` raw text.
fn extract_col(raw: &[u8]) -> u16 {
    extract_attr(raw, b"r")
        .and_then(|r| parse_cell_ref(&r).ok())
        .map(|(_, col)| col)
        .unwrap_or(0)
}

// ───────────────────────────────────────────────────────────────────────────
// Phase 3 — generic fallback: preserve_all_parts_transfer
// ───────────────────────────────────────────────────────────────────────────

/// Prefix patterns of non-data zip parts.
/// Zip entries starting with these patterns are copied from the source zip to the rebuilt zip.
const NON_DATA_PREFIXES: &[&str] = &[
    "xl/styles.xml",
    "xl/worksheets/_rels/",
    "xl/charts/",
    "xl/drawings/",
    "xl/comments",
    "xl/media/",
    "xl/theme/",
    "xl/vbaProject",
    "xl/calcChain",
    "xl/pivotTables/",
    "xl/pivotCache/",
    "xl/tables/",
    "xl/activeX/",
    "xl/richData/",
    "xl/printerSettings/",
    "xl/connectors/",
    "xl/chartsheets/",
    "xl/dialogsheet",
    "xl/ctrlProps/",
    "xl/queryTable/",
    "xl/peopleList/",
    "xl/attachments/",
    "xl/notes",
    "xl/metadata",
    "xl/volatileDependencies",
    "xl/dbPr",
];

/// Determines whether a zip entry is a non-data part (must be preserved from the source zip).
fn is_non_data_part(name: &str) -> bool {
    NON_DATA_PREFIXES.iter().any(|p| name.starts_with(p))
}

/// Reads all entries from the source zip into a HashMap.
fn read_zip_map(path: &str) -> Result<HashMap<String, Vec<u8>>> {
    use std::io::Read;
    let file = File::open(path).map_err(AppError::Io)?;
    let mut archive = ZipArchive::new(file)
        .map_err(|e| AppError::Custom(format!("failed to open zip: {}", e)))?;
    let mut map = HashMap::new();
    for i in 0..archive.len() {
        let mut zf = archive
            .by_index(i)
            .map_err(|e| AppError::Custom(format!("failed to read zip entry: {}", e)))?;
        let name = zf.name().to_string();
        let mut buf = Vec::new();
        zf.read_to_end(&mut buf).map_err(AppError::Io)?;
        map.insert(name, buf);
    }
    Ok(map)
}

/// Writes zip entries to a file.
fn write_zip_map(path: &str, entries: &HashMap<String, Vec<u8>>) -> Result<()> {
    use std::io::Write;
    let tmp = Path::new(path).with_extension("transfer_tmp");
    let file = File::create(&tmp).map_err(AppError::Io)?;
    let mut zw = ZipWriter::new(file);
    #[cfg(feature = "flate2")]
    let opt = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    #[cfg(not(feature = "flate2"))]
    let opt = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Stored);
    // Keep the original order: emit the source's order, but iterate the sorted key list
    for (name, content) in entries {
        zw.start_file(name, opt)
            .map_err(|e| AppError::Custom(format!("failed to write zip entry: {}", e)))?;
        zw.write_all(content).map_err(AppError::Io)?;
    }
    zw.finish()
        .map_err(|e| AppError::Custom(format!("failed to finish zip write: {}", e)))?;
    fs::rename(&tmp, path).map_err(AppError::Io)?;
    Ok(())
}

/// Parses the mapping of sheet names to part paths from workbook.xml.
fn parse_sheet_name_to_part(wb_xml: &[u8], rels_xml: &[u8]) -> HashMap<String, String> {
    let wb_str = String::from_utf8_lossy(wb_xml);
    let mut result = HashMap::new();

    // Parse all <sheet name="..." r:id="...">
    let mut pos = 0;
    while let Some(sheet_start) = wb_str[pos..].find("<sheet ") {
        let tag = &wb_str[pos + sheet_start..];
        let tag_end = tag.find('>').unwrap_or(tag.len());
        let tag_content = &tag[..tag_end];

        // Extract name and r:id
        let name = extract_attr_str(tag_content, "name");
        let rid = extract_attr_str(tag_content, "r:id");

        if let (Some(name), Some(rid)) = (name, rid) {
            // Find the Target for rid from the rels
            if let Some(target) = find_rel_target_str(rels_xml, &rid).ok() {
                result.insert(name, target);
            }
        }
        pos += sheet_start + 1;
    }
    result
}

/// Extracts an attribute value from a string.
fn extract_attr_str(s: &str, key: &str) -> Option<String> {
    let marker = format!("{}=\"", key);
    if let Some(start) = s.find(&marker) {
        let val_start = start + marker.len();
        if let Some(end) = s[val_start..].find('"') {
            return Some(s[val_start..val_start + end].to_string());
        }
    }
    None
}

/// Extracts all **top-level** child elements under the `<worksheet>` root element,
/// except `<sheetData>`, returning `(tag name, full XML fragment)`.
///
/// Implementation notes (the root cause of the previously corrupted files):
/// - Only the **direct children** of `<worksheet>` must be captured; the `<worksheet>` open/
///   close tags themselves must never be included, otherwise the whole root XML gets spliced
///   back into the rebuilt file, producing multiple `<worksheet>` roots and corrupting it.
/// - A depth-aware scan is used to correctly skip declarations/comments and fully capture
///   top-level fragments that contain nesting (e.g. `<mergeCells>...</mergeCells>`), instead
///   of an ever-growing text prefix.
fn extract_non_data_elements(xml: &[u8]) -> Vec<(String, String)> {
    let s = match std::str::from_utf8(xml) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };
    let root_open = match s.find("<worksheet") {
        Some(i) => i,
        None => return Vec::new(),
    };
    // Skip the <worksheet ...> open tag itself
    let open_end = match s[root_open..].find('>') {
        Some(p) => root_open + p + 1,
        None => return Vec::new(),
    };
    let root_close = match s.find("</worksheet>") {
        Some(i) => i,
        None => s.len(),
    };
    let body = &s[open_end..root_close];
    let bytes = body.as_bytes();
    let n = bytes.len();
    let mut out: Vec<(String, String)> = Vec::new();
    let mut i = 0;
    while i < n {
        if bytes[i] == b'<' {
            // Declaration / comment / processing instruction: skip the whole block
            if i + 1 < n && (bytes[i + 1] == b'?' || bytes[i + 1] == b'!') {
                let mut k = i + 1;
                while k < n && bytes[k] != b'>' {
                    k += 1;
                }
                i = if k < n { k + 1 } else { n };
                continue;
            }
            // Closing tag </tag>: skip
            if i + 1 < n && bytes[i + 1] == b'/' {
                let mut k = i + 2;
                while k < n && bytes[k] != b'>' {
                    k += 1;
                }
                i = if k < n { k + 1 } else { n };
                continue;
            }
            // Take the tag name (may include a namespace prefix; matching uses the local name)
            let mut j = i + 1;
            while j < n && bytes[j] != b' ' && bytes[j] != b'>' && bytes[j] != b'/' {
                j += 1;
            }
            let tag = &body[i + 1..j];
            let local = tag
                .split(|c: char| c == ' ' || c == ':')
                .last()
                .unwrap_or(tag);
            let local_str = local;
            if local_str.eq_ignore_ascii_case("sheetData") {
                // Skip the whole <sheetData>...</sheetData> (including nesting)
                i = skip_element(body, i);
                continue;
            }
            // Capture the complete fragment of this top-level element (including nested children)
            let (span, next) = capture_element(body, i);
            out.push((local_str.to_string(), span));
            i = next;
        } else {
            i += 1;
        }
    }
    out
}

/// Captures the complete fragment of the element starting at `body[start]` (pointing to `<tag`),
/// returning `(fragment string, next byte position)`. Handles self-closed tags and nesting depth.
fn capture_element(body: &str, start: usize) -> (String, usize) {
    let bytes = body.as_bytes();
    let n = bytes.len();
    // Tag name (local name)
    let mut j = start + 1;
    while j < n && bytes[j] != b' ' && bytes[j] != b'>' && bytes[j] != b'/' {
        j += 1;
    }
    let tag = &body[start + 1..j];
    let local = tag
        .split(|c: char| c == ' ' || c == ':')
        .last()
        .unwrap_or(tag);
    // Find the open tag's terminating '>'
    let mut k = start + 1;
    while k < n && bytes[k] != b'>' {
        k += 1;
    }
    if k < n && bytes[k - 1] == b'/' {
        // Self-closed <tag .../>
        return (body[start..=k].to_string(), k + 1);
    }
    // Depth-match </tag>
    let mut depth: i32 = 1;
    let mut p = k + 1;
    while p < n && depth > 0 {
        if bytes[p] == b'<' {
            if p + 1 < n && bytes[p + 1] == b'/' {
                // Closing tag
                let mut q = p + 2;
                while q < n && bytes[q] != b'>' {
                    q += 1;
                }
                let close_tag = &body[p + 2..q];
                let close_local = close_tag
                    .split(|c: char| c == ' ' || c == ':')
                    .next()
                    .unwrap_or(close_tag);
                if close_local.eq_ignore_ascii_case(local) {
                    depth -= 1;
                    if depth == 0 {
                        let end = if q < n { q + 1 } else { n };
                        return (body[start..end].to_string(), end);
                    }
                }
                p = if q < n { q + 1 } else { n };
            } else if p + 1 < n && (bytes[p + 1] == b'?' || bytes[p + 1] == b'!') {
                let mut q = p + 1;
                while q < n && bytes[q] != b'>' {
                    q += 1;
                }
                p = if q < n { q + 1 } else { n };
            } else {
                // Open tag
                let mut q = p + 1;
                while q < n && bytes[q] != b'>' {
                    q += 1;
                }
                if q < n && bytes[q - 1] == b'/' {
                    // Self-closed, does not count toward depth
                } else {
                    depth += 1;
                }
                p = if q < n { q + 1 } else { n };
            }
        } else {
            p += 1;
        }
    }
    (body[start..n].to_string(), n)
}

/// Skips the whole element starting at `start` (pointing to `<tag`), including nesting;
/// returns the next byte position.
fn skip_element(body: &str, start: usize) -> usize {
    let (_, next) = capture_element(body, start);
    next
}

/// Checks whether the rebuilt XML already contains the given element name.
fn has_element(xml: &str, tag_name: &str) -> bool {
    // Look for an open tag (excluding </)
    let open = format!("<{} ", tag_name);
    let open2 = format!("<{}>", tag_name);
    let open3 = format!("<{}/>", tag_name);
    xml.contains(&open) || xml.contains(&open2) || xml.contains(&open3)
}

/// Merges non-data elements from the source worksheet XML: inserts the style-related
/// top-level elements that the rebuilt version does not yet have, right after `</sheetData>`
/// of the rebuilt XML.
///
/// Only elements on the whitelist are merged (merge cells / data validation / conditional
/// formatting / auto filter / sheet protection / protected ranges / extLst), and the rebuilt
/// version must not already contain the same-named field, to avoid duplicates or splicing the
/// root tag back in (the latter would create multiple `<worksheet>` roots and corrupt the file).
fn merge_worksheet_xml(source_xml: &[u8], rebuilt_xml: &[u8]) -> Vec<u8> {
    let rebuilt_str = String::from_utf8_lossy(rebuilt_xml);
    let source_elements = extract_non_data_elements(source_xml);

    let sd_end_pos = rebuilt_str.find("</sheetData>");
    match sd_end_pos {
        Some(pos) => {
            let insert_pos = pos + 12; // after </sheetData>
            let after_sd = &rebuilt_str[insert_pos..];
            let ws_end_pos = after_sd.find("</worksheet>").unwrap_or(after_sd.len());
            let existing_after = &after_sd[..ws_end_pos];

            const PRESERVE: &[&str] = &[
                "mergeCells",
                "dataValidations",
                "conditionalFormatting",
                "autoFilter",
                "sheetProtection",
                "protectedRanges",
                "extLst",
            ];

            let mut to_insert = String::new();
            for (tag_name, xml_fragment) in &source_elements {
                if !PRESERVE.contains(&tag_name.as_str()) {
                    continue;
                }
                if !has_element(existing_after, tag_name) {
                    to_insert.push_str(xml_fragment);
                    to_insert.push('\n');
                }
            }

            if to_insert.is_empty() {
                rebuilt_xml.to_vec()
            } else {
                let mut result = rebuilt_str[..insert_pos].to_string();
                result.push('\n');
                result.push_str(&to_insert);
                result.push_str(after_sd);
                result.into_bytes()
            }
        }
        None => rebuilt_xml.to_vec(),
    }
}

/// Preserves all non-data zip parts during a full rebuild.
///
/// Flow:
/// 1. Open the source zip and the rebuilt new zip
/// 2. Copy non-data parts from the source zip to the new zip (overwriting same-named parts):
///    - styles.xml, charts/*.xml, drawings/*.xml, comments*.xml, media/*, etc.
/// 3. Copy non-data elements of the worksheet XML from the source zip into the new zip's
///    worksheet XML:
///    - <mergeCells>, <dataValidations>, <conditionalFormatting>, <autoFilter>, etc.
/// 4. Update the new zip's [Content_Types].xml
/// 5. Save the new zip
///
/// # Arguments
/// * `src_path` - the original xlsx file path (before modification)
/// * `rebuilt_path` - the xlsx file path rebuilt by rust_xlsxwriter (which lost the
///                    non-data parts); this file is modified in place
pub fn preserve_all_parts_transfer(src_path: &str, rebuilt_path: &str) -> Result<()> {
    // 1. Read the source zip and the rebuilt zip
    let src_entries = read_zip_map(src_path)?;
    let rebuilt_entries = read_zip_map(rebuilt_path)?;

    // Build the output entries
    let mut output = HashMap::new();

    // 2. Collect the data-part names of the rebuilt zip
    let mut data_part_names: Vec<String> = Vec::new();

    // 3. Process the rebuilt entries
    for (name, content) in &rebuilt_entries {
        // For worksheet XML, merge the non-data elements
        if name.starts_with("xl/worksheets/sheet") && name.ends_with(".xml") {
            // Find the corresponding source worksheet XML
            let merged = if let Some(src_content) = src_entries.get(name) {
                merge_worksheet_xml(src_content, content)
            } else {
                content.clone()
            };
            output.insert(name.clone(), merged);
            data_part_names.push(name.clone());
        } else {
            output.insert(name.clone(), content.clone());
            data_part_names.push(name.clone());
        }
    }

    // 4. Copy non-data parts from the source zip (those not in the rebuilt zip)
    let mut added_content_types: Vec<String> = Vec::new();
    for (name, content) in &src_entries {
        if is_non_data_part(name) && !output.contains_key(name) {
            output.insert(name.clone(), content.clone());
            // Record parts that need to be added to Content_Types
            // Compute the PartName (in the form /xl/...)
            let part_name = if name.starts_with("xl/") {
                format!("/{}", name)
            } else {
                format!("/xl/{}", name)
            };
            added_content_types.push(part_name);
        }
    }

    // 5. styles.xml uses the version rebuilt by rust_xlsxwriter.
    //    Note: during a full rebuild rust_xlsxwriter already generates a styles.xml that is
    //    fully consistent with the rebuilt worksheet (new style indexes included).
    //    Overwriting it with the source styles.xml here would make the style indexes referenced
    //    by the worksheet missing from styles.xml, producing a corrupted file (unparseable by
    //    Excel/openpyxl). So the source file is no longer used to overwrite, preserving the
    //    styles actually written by this operation.
    //    (Known limitation: old styles in the source file that this rebuild did not re-apply
    //     are lost with the full rebuild — an inherent limitation of the rust_xlsxwriter full
    //     rebuild architecture, not corruption.)

    // 6. Update [Content_Types].xml: add the newly added non-data parts
    if !added_content_types.is_empty() {
        if let Some(ct_content) = output.get("[Content_Types].xml") {
            let mut ct_str = String::from_utf8_lossy(ct_content).to_string();
            let mut modified = false;
            for part_name in &added_content_types {
                // Check whether it already exists
                if !ct_str.contains(&format!("PartName=\"{}\"", part_name)) {
                    // Determine the ContentType
                    let content_type = guess_content_type(part_name);
                    let override_xml = format!(
                        "  <Override PartName=\"{}\" ContentType=\"{}\"/>\n",
                        part_name, content_type
                    );
                    if let Some(pos) = ct_str.rfind("</Types>") {
                        ct_str.insert_str(pos, &override_xml);
                        modified = true;
                    }
                }
            }
            if modified {
                output.insert("[Content_Types].xml".to_string(), ct_str.into_bytes());
            }
        }
    }

    // 7. Write the output zip
    write_zip_map(rebuilt_path, &output)?;

    Ok(())
}

/// Guesses the ContentType from the part path.
fn guess_content_type(part_name: &str) -> &'static str {
    if part_name.ends_with(".xml") {
        if part_name.contains("/charts/") {
            "application/vnd.openxmlformats-officedocument.drawingml.chart+xml"
        } else if part_name.contains("/drawings/") {
            "application/vnd.openxmlformats-officedocument.drawing+xml"
        } else if part_name.contains("/comments") {
            "application/vnd.openxmlformats-officedocument.spreadsheetml.comments+xml"
        } else if part_name.contains("/styles.xml") {
            "application/vnd.openxmlformats-officedocument.spreadsheetml.styles+xml"
        } else if part_name.contains("/theme/") {
            "application/vnd.openxmlformats-officedocument.theme+xml"
        } else if part_name.contains("/calcChain") {
            "application/vnd.openxmlformats-officedocument.spreadsheetml.calcChain+xml"
        } else if part_name.contains("/pivotTables/") {
            "application/vnd.openxmlformats-officedocument.spreadsheetml.pivotTable+xml"
        } else if part_name.contains("/pivotCache/") {
            "application/vnd.openxmlformats-officedocument.spreadsheetml.pivotCacheRecords+xml"
        } else if part_name.contains("/tables/") {
            "application/vnd.openxmlformats-officedocument.spreadsheetml.table+xml"
        } else if part_name.contains("/activeX/") {
            "application/vnd.ms-office.activeX+xml"
        } else if part_name.contains("/ctrlProps/") {
            "application/vnd.ms-office.controlProperties+xml"
        } else if part_name.contains("/queryTable/") {
            "application/vnd.openxmlformats-officedocument.spreadsheetml.queryTable+xml"
        } else if part_name.contains("/metadata") {
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheetMetadata+xml"
        } else if part_name.contains("/volatileDependencies") {
            "application/vnd.openxmlformats-officedocument.spreadsheetml.volatileDependencies+xml"
        } else if part_name.contains("/peopleList") {
            "application/vnd.ms-office.peopleList+xml"
        } else if part_name.contains("/attachments") {
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheetAttachments+xml"
        } else if part_name.contains("/notes") {
            "application/vnd.openxmlformats-officedocument.spreadsheetml.notes+xml"
        } else if part_name.contains("/dbPr") {
            "application/vnd.openxmlformats-officedocument.spreadsheetml.databaseProperties+xml"
        } else if part_name.contains("/chartsheets/") {
            "application/vnd.openxmlformats-officedocument.spreadsheetml.chartsheet+xml"
        } else if part_name.contains("/dialogsheet") {
            "application/vnd.openxmlformats-officedocument.spreadsheetml.dialogsheet+xml"
        } else if part_name.contains("/connections") {
            "application/vnd.openxmlformats-officedocument.spreadsheetml.connections+xml"
        } else if part_name.contains("/externalLinks") {
            "application/vnd.openxmlformats-officedocument.spreadsheetml.externalLink+xml"
        } else {
            "application/xml"
        }
    } else if part_name.ends_with(".bin") {
        if part_name.contains("vbaProject") {
            "application/vnd.ms-office.vbaProject"
        } else if part_name.contains("printerSettings") {
            "application/vnd.openxmlformats-officedocument.spreadsheetml.printerSettings"
        } else {
            "application/octet-stream"
        }
    } else if part_name.contains("/media/") {
        // Determine by file extension
        if part_name.ends_with(".png") {
            "image/png"
        } else if part_name.ends_with(".jpg") || part_name.ends_with(".jpeg") {
            "image/jpeg"
        } else if part_name.ends_with(".gif") {
            "image/gif"
        } else if part_name.ends_with(".svg") {
            "image/svg+xml"
        } else if part_name.ends_with(".bmp") {
            "image/bmp"
        } else {
            "application/octet-stream"
        }
    } else {
        "application/octet-stream"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn patch_inserts_new_cell_and_preserves_existing() {
        let xml = b"<?xml version=\"1.0\"?><worksheet xmlns=\"x\"><sheetData>\
<row r=\"1\"><c r=\"A1\" s=\"3\"><v>1</v></c><c r=\"B1\" s=\"3\"><v>2</v></c></row>\
<row r=\"2\"><c r=\"A2\"><v>3</v></c></row>\
</sheetData></worksheet>";
        let edits = vec![(
            0u32,
            25u16, // Z
            CellData {
                value: Some("x".to_string()),
                data_type: CellDataType::String,
                formula: None,
            },
        )];
        let out = patch_sheet_xml(xml, &edits).unwrap();
        let s = String::from_utf8_lossy(&out);
        assert!(s.contains("r=\"A1\" s=\"3\""), "A1 style lost: {}", s);
        assert!(
            s.contains("r=\"Z1\"") && s.contains("t=\"inlineStr\"") && s.contains("<t>x</t>"),
            "Z1 not inserted: {}",
            s
        );
        assert!(s.contains("r=\"B1\"") && s.contains("<v>2</v>"), "B1 lost: {}", s);
        assert!(s.contains("r=\"A2\"") && s.contains("<v>3</v>"), "A2 lost: {}", s);
        let p_a = s.find("r=\"A1\"").unwrap();
        let p_b = s.find("r=\"B1\"").unwrap();
        let p_z = s.find("r=\"Z1\"").unwrap();
        assert!(p_a < p_b && p_b < p_z, "cell order wrong");
    }

    #[test]
    fn patch_edits_existing_cell_keeps_style() {
        let xml = b"<worksheet><sheetData><row r=\"3\"><c r=\"C3\" s=\"7\"><v>9</v></c></row></sheetData></worksheet>";
        let edits = vec![(
            2u32,
            2u16,
            CellData {
                value: Some("hi".to_string()),
                data_type: CellDataType::String,
                formula: None,
            },
        )];
        let out = patch_sheet_xml(xml, &edits).unwrap();
        let s = String::from_utf8_lossy(&out);
        assert!(
            s.contains("r=\"C3\"") && s.contains("s=\"7\"") && s.contains("t=\"inlineStr\"") && s.contains("<t>hi</t>"),
            "style lost after edit: {}",
            s
        );
        assert!(!s.contains("<v>9</v>"), "old value not cleared");
    }

    #[test]
    fn patch_no_sheetdata_returns_unchanged() {
        let xml = b"<worksheet><sheetViews/></worksheet>";
        let edits = vec![(
            0u32,
            0u16,
            CellData {
                value: Some("z".to_string()),
                data_type: CellDataType::String,
                formula: None,
            },
        )];
        let out = patch_sheet_xml(xml, &edits).unwrap();
        assert_eq!(out, xml);
    }

    // ─────────────────────────────────────────────────────────────────────
    // End-to-end: preserving write must keep every non-target zip part byte-for-byte, and
    // styles / merges / data validation / frozen panes inside the target sheet must not be
    // wiped out. This is the root cause behind T3.03-09 and T4.10 (the old
    // modify_file_with_wb full rebuild lost those features).
    // ─────────────────────────────────────────────────────────────────────

    const FIXTURE_CONTENT_TYPES: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/>
  <Override PartName="/xl/worksheets/sheet1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/>
  <Override PartName="/xl/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.styles+xml"/>
  <Override PartName="/xl/sharedStrings.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sharedStrings+xml"/>
  <Override PartName="/xl/comments1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.comments+xml"/>
  <Override PartName="/xl/drawings/drawing1.xml" ContentType="application/vnd.openxmlformats-officedocument.drawing+xml"/>
  <Override PartName="/xl/charts/chart1.xml" ContentType="application/vnd.openxmlformats-officedocument.drawingml.chart+xml"/>
</Types>"#;

    const FIXTURE_ROOT_RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/>
</Relationships>"#;

    const FIXTURE_WORKBOOK: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <sheets>
    <sheet name="Sales" sheetId="1" r:id="rId1"/>
  </sheets>
</workbook>"#;

    const FIXTURE_WORKBOOK_RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/>
  <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/>
  <Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/sharedStrings" Target="sharedStrings.xml"/>
  <Relationship Id="rId4" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/comments" Target="comments1.xml"/>
</Relationships>"#;

    // Contains the recognizable marker DISTINCT_STYLE_9999 to verify styles.xml is kept intact.
    const FIXTURE_STYLES: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <numFmts count="1"><numFmt numFmtId="9999" formatCode="DISTINCT_STYLE_9999"/></numFmts>
  <fonts count="1"><font><sz val="11"/></font></fonts>
  <fills count="1"><fill><patternFill patternType="none"/></fill></fills>
  <borders count="1"><border/></borders>
  <cellStyleXfs count="1"><xf numFmtId="0" fontId="0" fillId="0" borderId="0"/></cellStyleXfs>
  <cellXfs count="4">
    <xf numFmtId="0" fontId="0" fillId="0" borderId="0" xfId="0"/>
    <xf numFmtId="0" fontId="0" fillId="0" borderId="0" xfId="0" applyFont="1"/>
    <xf numFmtId="9999" fontId="0" fillId="0" borderId="0" xfId="0" applyNumberFormat="1"/>
    <xf numFmtId="0" fontId="0" fillId="0" borderId="0" xfId="0" applyFont="1" applyFill="1"/>
  </cellXfs>
</styleSheet>"#;

    const FIXTURE_SHARED_STRINGS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<sst xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" count="1" uniqueCount="1"><si><t>Hello</t></si></sst>"#;

    // Contains the recognizable marker DISTINCT_COMMENT_A1.
    const FIXTURE_COMMENTS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<comments xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <authors><author>tester</author></authors>
  <commentList>
    <comment ref="A1" authorId="0"><text><t>DISTINCT_COMMENT_A1</t></text></comment>
  </commentList>
</comments>"#;

    // Contains the recognizable markers DISTINCT_DRAWING / DISTINCT_CHART_TITLE.
    const FIXTURE_DRAWING: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<xdr:wsDr xmlns:xdr="http://schemas.openxmlformats.org/drawingml/2006/spreadsheetDrawing" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <xdr:twoCellAnchor><xdr:from><xdr:col>0</xdr:col></xdr:from><xdr:to><xdr:col>5</xdr:col></xdr:to>
  <xdr:graphicFrame><xdr:nvGraphicFramePr><xdr:cNvPr id="2" name="Chart 1"/><xdr:cNvGraphicFramePr/></xdr:nvGraphicFramePr>
  <xdr:graphicFrameLocks noGrp="1"/></xdr:graphicFrame><xdr:clientData/>
  <DISTINCT_DRAWING/></xdr:twoCellAnchor>
</xdr:wsDr>"#;

    const FIXTURE_CHART: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart">
  <c:chart><c:title><c:tx><c:rich><a:p><a:r><a:t>DISTINCT_CHART_TITLE</a:t></a:r></c:rich></c:tx></c:title></c:chart>
</c:chartSpace>"#;

    const FIXTURE_SHEET_RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/comments" Target="../comments1.xml"/>
  <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/drawing" Target="../drawings/drawing1.xml"/>
</Relationships>"#;

    // Target sheet: contains frozen panes (pane state=frozen), merged cells, data validation,
    // and a cell with style index s="3".
    const FIXTURE_SHEET: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <sheetViews>
    <sheetView workbookViewId="0">
      <pane xSplit="1" ySplit="2" topLeftCell="B3" activePane="bottomRight" state="frozen"/>
    </sheetView>
  </sheetViews>
  <sheetData>
    <row r="1"><c r="A1" s="3" t="s"><v>0</v></c><c r="B1" s="1"><v>100</v></c></row>
    <row r="2"><c r="A2" s="2"><v>200</v></c></row>
  </sheetData>
  <mergeCells count="1">
    <mergeCell ref="C1:E1"/>
  </mergeCells>
  <dataValidations count="1">
    <dataValidation type="whole" allowBlank="1" sqref="F1:F10"><formula1>1</formula1><formula2>100</formula2></dataValidation>
  </dataValidations>
</worksheet>"#;

    fn build_fixture(path: &std::path::Path) {
        use std::io::Write;
        use zip::write::SimpleFileOptions;
        let file = File::create(path).unwrap();
        let mut zw = ZipWriter::new(file);
        let opt = SimpleFileOptions::default();
        let parts: &[(&str, &str)] = &[
            ("[Content_Types].xml", FIXTURE_CONTENT_TYPES),
            ("_rels/.rels", FIXTURE_ROOT_RELS),
            ("xl/workbook.xml", FIXTURE_WORKBOOK),
            ("xl/_rels/workbook.xml.rels", FIXTURE_WORKBOOK_RELS),
            ("xl/styles.xml", FIXTURE_STYLES),
            ("xl/sharedStrings.xml", FIXTURE_SHARED_STRINGS),
            ("xl/comments1.xml", FIXTURE_COMMENTS),
            ("xl/drawings/drawing1.xml", FIXTURE_DRAWING),
            ("xl/charts/chart1.xml", FIXTURE_CHART),
            ("xl/worksheets/sheet1.xml", FIXTURE_SHEET),
            ("xl/worksheets/_rels/sheet1.xml.rels", FIXTURE_SHEET_RELS),
        ];
        for (name, data) in parts {
            zw.start_file(*name, opt).unwrap();
            zw.write_all(data.as_bytes()).unwrap();
        }
        zw.finish().unwrap();
    }

    // Same as build_fixture, but the worksheet Target in workbook.xml.rels uses an
    // **absolute path** (with a leading slash): Target="/xl/worksheets/sheet1.xml".
    // This is the form produced by some real tools (e.g. this project's verify/data/sales.xlsx);
    // resolve_sheet_part used to return the leading-slash path verbatim, so by_name could not
    // find the entry, the write errored early and the value never landed (observed as cell
    // write then read back as None). This test locks in that fix.
    const FIXTURE_WORKBOOK_RELS_ABS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="/xl/worksheets/sheet1.xml"/>
  <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/>
  <Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/sharedStrings" Target="sharedStrings.xml"/>
  <Relationship Id="rId4" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/comments" Target="comments1.xml"/>
</Relationships>"#;

    fn build_fixture_abs_target(path: &std::path::Path) {
        use std::io::Write;
        use zip::write::SimpleFileOptions;
        let file = File::create(path).unwrap();
        let mut zw = ZipWriter::new(file);
        let opt = SimpleFileOptions::default();
        let parts: &[(&str, &str)] = &[
            ("[Content_Types].xml", FIXTURE_CONTENT_TYPES),
            ("_rels/.rels", FIXTURE_ROOT_RELS),
            ("xl/workbook.xml", FIXTURE_WORKBOOK),
            ("xl/_rels/workbook.xml.rels", FIXTURE_WORKBOOK_RELS_ABS),
            ("xl/styles.xml", FIXTURE_STYLES),
            ("xl/sharedStrings.xml", FIXTURE_SHARED_STRINGS),
            ("xl/comments1.xml", FIXTURE_COMMENTS),
            ("xl/drawings/drawing1.xml", FIXTURE_DRAWING),
            ("xl/charts/chart1.xml", FIXTURE_CHART),
            ("xl/worksheets/sheet1.xml", FIXTURE_SHEET),
            ("xl/worksheets/_rels/sheet1.xml.rels", FIXTURE_SHEET_RELS),
        ];
        for (name, data) in parts {
            zw.start_file(*name, opt).unwrap();
            zw.write_all(data.as_bytes()).unwrap();
        }
        zw.finish().unwrap();
    }

    fn read_zip_map(path: &str) -> std::collections::HashMap<String, Vec<u8>> {
        use std::io::Read;
        let f = File::open(path).unwrap();
        let mut za = ZipArchive::new(f).unwrap();
        let mut map = std::collections::HashMap::new();
        for i in 0..za.len() {
            let mut zf = za.by_index(i).unwrap();
            let name = zf.name().to_string();
            let mut buf = Vec::new();
            zf.read_to_end(&mut buf).unwrap();
            map.insert(name, buf);
        }
        map
    }

    #[test]
    fn e2e_preserves_non_target_parts_and_rich_features() {
        use std::collections::HashMap;
        use crate::types::{CellData, CellDataType, SecurityParams};

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("fixture.xlsx");
        build_fixture(&path);
        let path_str = path.to_str().unwrap().to_string();

        let before: HashMap<String, Vec<u8>> = read_zip_map(&path_str);

        let params = SecurityParams {
            dry_run: false,
            create_backup: false,
            file_path: path_str.clone(),
        };
        // Write string "x" at Z10 (row index 9, column index 25), simulating cell write
        // commands such as T3.03/05.
        let edits = vec![(
            9u32,
            25u16,
            CellData {
                value: Some("x".to_string()),
                data_type: CellDataType::String,
                formula: None,
            },
        )];
        let result = write_cells_preserving(&path_str, &params, "Sales", &edits).unwrap();
        assert!(result.success, "preserving write should succeed");

        let after: HashMap<String, Vec<u8>> = read_zip_map(&path_str);
        let target = "xl/worksheets/sheet1.xml";

        // 1) Every non-target part decompresses byte-for-byte identical.
        for (name, content) in &before {
            if name == target {
                continue;
            }
            let a = after
                .get(name)
                .unwrap_or_else(|| panic!("non-target part missing: {}", name));
            assert_eq!(a, content, "non-target part content changed (lost source feature): {}", name);
        }

        // 2) All rich-feature markers survive (styles / comments / drawing / chart).
        let styles = String::from_utf8_lossy(after.get("xl/styles.xml").unwrap());
        assert!(styles.contains("DISTINCT_STYLE_9999"), "styles.xml not preserved");
        let comments = String::from_utf8_lossy(after.get("xl/comments1.xml").unwrap());
        assert!(comments.contains("DISTINCT_COMMENT_A1"), "comments1.xml not preserved");
        let drawing = String::from_utf8_lossy(after.get("xl/drawings/drawing1.xml").unwrap());
        assert!(drawing.contains("DISTINCT_DRAWING"), "drawing1.xml not preserved");
        let chart = String::from_utf8_lossy(after.get("xl/charts/chart1.xml").unwrap());
        assert!(chart.contains("DISTINCT_CHART_TITLE"), "chart1.xml not preserved");

        // 3) Target sheet: styled cells, merges, data validation and frozen panes are all
        // preserved, and the new cell has been written.
        let sheet = String::from_utf8_lossy(after.get(target).unwrap());
        assert!(sheet.contains("s=\"3\"") && sheet.contains("t=\"s\"") && sheet.contains("<v>0</v>"),
            "A1 style/shared-string reference lost: {}", sheet);
        assert!(sheet.contains("mergeCells") && sheet.contains("C1:E1"), "merged cells lost");
        assert!(sheet.contains("dataValidation") && sheet.contains("F1:F10"), "data validation lost");
        assert!(sheet.contains("state=\"frozen\""), "frozen panes lost");
        assert!(
            sheet.contains("r=\"Z10\"") && sheet.contains("t=\"inlineStr\"") && sheet.contains("<t>x</t>"),
            "Z10 new cell not written: {}", sheet
        );
        // Old values 100 / 200 are still present (not cleared).
        assert!(sheet.contains("<v>100</v>") && sheet.contains("<v>200</v>"), "original values lost");
        // Row order is valid: r=1 before r=2, r=2 before r=10.
        let p1 = sheet.find("r=\"1\"").unwrap();
        let p2 = sheet.find("r=\"2\"").unwrap();
        let p10 = sheet.find("r=\"10\"").unwrap();
        assert!(p1 < p2 && p2 < p10, "row order invalid");
    }

    // Absolute-path Target (/xl/worksheets/sheet1.xml) form: the leading slash used to make
    // the write fail; now it must succeed with rich features and the new cell correctly
    // persisted (regression locked in).
    #[test]
    fn e2e_preserves_with_absolute_rel_target() {
        use std::collections::HashMap;
        use crate::types::{CellData, CellDataType, SecurityParams};

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("fixture_abs.xlsx");
        build_fixture_abs_target(&path);
        let path_str = path.to_str().unwrap().to_string();

        let params = SecurityParams {
            dry_run: false,
            create_backup: false,
            file_path: path_str.clone(),
        };
        let edits = vec![(
            9u32,
            25u16,
            CellData {
                value: Some("x".to_string()),
                data_type: CellDataType::String,
                formula: None,
            },
        )];
        let result = write_cells_preserving(&path_str, &params, "Sales", &edits)
            .expect("preserving write should succeed with absolute-path Target");
        assert!(result.success);

        let after: HashMap<String, Vec<u8>> = read_zip_map(&path_str);
        let target = "xl/worksheets/sheet1.xml";
        let sheet = String::from_utf8_lossy(after.get(target).unwrap());
        // The new cell was written successfully (proving resolve correctly normalized the leading slash).
        assert!(
            sheet.contains("r=\"Z10\"") && sheet.contains("t=\"inlineStr\"") && sheet.contains("<t>x</t>"),
            "Z10 not written with absolute-path Target: {}", sheet
        );
        // Rich features and styled cells are all preserved.
        assert!(sheet.contains("s=\"3\"") && sheet.contains("state=\"frozen\""),
            "styles/frozen panes lost with absolute-path Target: {}", sheet);
        assert!(String::from_utf8_lossy(after.get("xl/styles.xml").unwrap()).contains("DISTINCT_STYLE_9999"),
            "styles.xml not preserved");
    }

    // T5.16: after writing a far cell (Z50), <dimension> must expand to cover it instead of
    // staying at the original range (e.g. A1:G11).
    #[test]
    fn patch_updates_dimension_to_cover_far_cell() {
        use crate::types::{CellData, CellDataType, SecurityParams};
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("dim.xlsx");
        build_fixture(&path);
        let path_str = path.to_str().unwrap().to_string();
        let params = SecurityParams {
            dry_run: false,
            create_backup: false,
            file_path: path_str.clone(),
        };
        let edits = vec![(
            49u32,
            25u16,
            CellData {
                value: Some("far".to_string()),
                data_type: CellDataType::String,
                formula: None,
            },
        )];
        write_cells_preserving(&path_str, &params, "Sales", &edits).unwrap();
        let after = read_zip_map(&path_str);
        let sheet = String::from_utf8_lossy(after.get("xl/worksheets/sheet1.xml").unwrap());
        let start = sheet.find("<dimension").expect("dimension element expected");
        let end = start + sheet[start..].find("/>").expect("dimension should be self-closed");
        let dim_tag = &sheet[start..=end];
        assert!(
            dim_tag.contains("Z50"),
            "dimension did not expand to cover far cell Z50: {}",
            dim_tag
        );
    }

    // T5.19: a value starting with "=" is recognized as a formula per Excel semantics,
    // written to <f> instead of literal text.
    #[test]
    fn patch_treats_leading_equals_as_formula() {
        use crate::types::{CellData, CellDataType, SecurityParams};
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("eq.xlsx");
        build_fixture(&path);
        let path_str = path.to_str().unwrap().to_string();
        let params = SecurityParams {
            dry_run: false,
            create_backup: false,
            file_path: path_str.clone(),
        };
        let edits = vec![(
            0u32,
            25u16,
            CellData {
                value: Some("=SUM(A1:A2)".to_string()),
                data_type: CellDataType::String,
                formula: None,
            },
        )];
        write_cells_preserving(&path_str, &params, "Sales", &edits).unwrap();
        let after = read_zip_map(&path_str);
        let sheet = String::from_utf8_lossy(after.get("xl/worksheets/sheet1.xml").unwrap());
        assert!(
            sheet.contains("r=\"Z1\"") && sheet.contains("<f>SUM(A1:A2)</f>"),
            "leading = value not recognized as formula: {}",
            sheet
        );
    }
}
