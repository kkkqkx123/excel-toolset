use crate::security::{compute_file_hash, create_backup};
use crate::types::{
    AppError, DataValidationConfig, PageSetupConfig, Result, SecurityParams, SheetProtectionConfig,
    SheetVisibility, WriteResult,
};
use std::fs::File;
use zip::ZipArchive;

use super::*;

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
    append_history(
        path,
        "set_freeze_panes",
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

/// Preserving clear freeze panes.
pub fn clear_freeze_panes_preserving(
    path: &str,
    params: &SecurityParams,
    sheet: &str,
) -> Result<WriteResult> {
    set_freeze_panes_preserving(path, params, sheet, 0, 0)
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
    append_history(
        path,
        "set_auto_filter",
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
    append_history(
        path,
        "remove_auto_filter",
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
    append_history(
        path,
        "add_data_validation",
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
    append_history(
        path,
        "unprotect_sheet",
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

/// Preserving page setup — not implemented yet, deferred to Phase 3.
pub fn configure_page_setup_preserving(
    _path: &str,
    _params: &SecurityParams,
    _sheet: &str,
    _config: &PageSetupConfig,
) -> Result<WriteResult> {
    Err(AppError::Custom(
        "configure_page_setup_preserving not implemented yet - use full transfer fallback".into(),
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
    append_history(
        path,
        "set_sheet_visibility",
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
