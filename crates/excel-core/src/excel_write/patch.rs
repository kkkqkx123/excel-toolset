//! 保留式写入：zip 级增量改写。
//!
//! 根因：原先 `modify_file` / `modify_file_with_wb` 用 `rust_xlsxwriter::Workbook::new()`
//! 全量重建，只写回 calamine 读到的「值+公式」，把源文件中 calamine 没读到的
//! 样式 / 合并 / 图表 / 注释 / 数据验证 / 冻结窗格 / 绘图层 全部抹掉。
//!
//! 本模块只在已有的 xlsx zip 包中**就地重写字目标 sheet 的 `<sheetData>`**，其余所有
//! part（`styles.xml`、`drawings/*`、`charts/*`、`comments*.xml`、其它 sheet、`rels`、
//! `[Content_Types].xml`）逐字节原样复制，从而 100% 保留源文件特性。
//!
//! 仅当 `zip` feature 启用时编译（随 `full` 默认开启）。

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
// 公开入口
// ───────────────────────────────────────────────────────────────────────────

/// 保留式写入：仅改写 `sheet` 中 `edits` 指定的单元格，其余 zip part 逐字节保留。
///
/// `edits` 中每个元素为 `(行索引 0-based, 列索引 0-based, 新单元格数据)`。
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

    // 空编辑或 dry-run：不触碰文件，沿用旧 hash。
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
        .map_err(|e| AppError::Custom(format!("无法以 zip 打开 xlsx: {}", e)))?;

    let part = resolve_sheet_part(&mut archive, sheet)?;
    let sheet_xml = read_zip_entry(&mut archive, &part)?;
    let new_xml = patch_sheet_xml(&sheet_xml, edits)?;

    // 重新打包：除目标 sheet 外，其余 part 逐字节复制。
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

/// 保留式设置公式：仅改写 `sheet` 中 `(row, col)` 单元格的公式，其余 zip part 逐字节保留。
/// `row`/`col` 均为 0-based。
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

/// 保留式设置公式 + 缓存值：同时写入 `<f>` 和 `<v>`，其余 zip part 逐字节保留。
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

/// 保留式清空单元格范围：移除 range 内所有现存单元格的 `<f>`/`<v>`，其余 zip part 逐字节保留。
/// `r_start/r_end` 0-based 行索引，`c_start/c_end` 0-based 列索引。
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
        .map_err(|e| AppError::Custom(format!("无法以 zip 打开 xlsx: {}", e)))?;
    let part = resolve_sheet_part(&mut archive, sheet)?;
    let sheet_xml = read_zip_entry(&mut archive, &part)?;

    let (before, inner, after, self_closed) = match sheetdata_spans(&sheet_xml) {
        Some(x) => x,
        None => {
            // 无 sheetData 元素：无单元格可清空。
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
                    false // 移除该单元格（清空效果）
                } else {
                    true
                }
            });
        }
        !row.cells.is_empty() // 移除空行
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

/// 保留式清除公式缓存值：移除指定 sheet 中所有公式单元格的 `<v>` 元素，使公式下次打开时重新计算。
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
        .map_err(|e| AppError::Custom(format!("无法以 zip 打开 xlsx: {}", e)))?;
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

/// 保留式合并单元格：在目标 sheet 中追加合并单元格范围，其余 zip part 逐字节保留。
/// `r1/r2` 0-based 行索引，`c1/c2` 0-based 列索引。
/// 在左上角单元格写入指定值。
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
        .map_err(|e| AppError::Custom(format!("无法以 zip 打开 xlsx: {}", e)))?;
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

/// 字符串方式 patch mergeCells 元素。
fn patch_merge_cells_str(xml: &[u8], new_range: &str) -> Result<Vec<u8>> {
    let s = String::from_utf8(xml.to_vec())
        .map_err(|e| AppError::Custom(format!("XML 不是合法 UTF-8: {}", e)))?;
    let mut result = s;

    // 检查是否已存在 <mergeCells>（闭合或自闭合）
    if let Some(pos) = result.find("</mergeCells>") {
        // 有开放标签 → 在闭合前插入新条目，并更新 count
        let mc = format!("    <mergeCell ref=\"{}\"/>\n", new_range);
        result.insert_str(pos, &mc);
        // 更新 count 属性
        let old_count = count_merge_cells(&result);
        let new_count = old_count + 1;
        // 找到 count="N" 并用新值替换
        let old_count_str = format!("count=\"{}\"", old_count);
        let new_count_str = format!("count=\"{}\"", new_count);
        result = result.replacen(&old_count_str, &new_count_str, 1);
    } else if let Some(pos) = result.find("<mergeCells/>") {
        // 自闭合标签 → 转为开放标签
        let replacement = format!(
            "<mergeCells count=\"1\">\n    <mergeCell ref=\"{}\"/>\n  </mergeCells>",
            new_range
        );
        result.replace_range(pos..pos + 13, &replacement);
    } else {
        // 不存在 mergeCells → 在 </worksheet> 前插入
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

/// 统计当前 XML 中 `<mergeCell` 的出现次数（用于更新 count 属性）。
fn count_merge_cells(xml: &str) -> usize {
    xml.matches("<mergeCell ").count()
}

/// 保留式设置冻结窗格：修改 sheet XML 的 `<sheetViews>` 元素，其余 zip part 逐字节保留。
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
        .map_err(|e| AppError::Custom(format!("无法以 zip 打开 xlsx: {}", e)))?;
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

/// 保留式清除冻结窗格。
pub fn clear_freeze_panes_preserving(
    path: &str,
    params: &SecurityParams,
    sheet: &str,
) -> Result<WriteResult> {
    set_freeze_panes_preserving(path, params, sheet, 0, 0)
}

/// 字符串方式 patch freeze panes。
fn patch_freeze_panes_str(xml: &[u8], rows: u32, cols: u16) -> Result<Vec<u8>> {
    let s = String::from_utf8(xml.to_vec())
        .map_err(|e| AppError::Custom(format!("XML 不是合法 UTF-8: {}", e)))?;
    let mut result = s;

    if rows == 0 && cols == 0 {
        // 清除冻结：移除所有 <pane .../> 元素
        loop {
            let start = result.find("<pane ");
            let end = result.find("/>");
            match (start, end) {
                (Some(s), Some(e)) if s < e => {
                    // 找到 pane 标签，移除它
                    let pane_end = e + 2; // include "/>"
                    result.replace_range(s..pane_end, "");
                }
                _ => break,
            }
        }
        return Ok(result.into_bytes());
    }

    let top_left_cell = format!("{}{}", index_to_col(cols), rows + 1);
    let active_pane = match (rows > 0, cols > 0) {
        (true, true) => "bottomRight",
        (true, false) => "bottomLeft",
        (false, true) => "topRight",
        (false, false) => "bottomRight",
    };

    let pane_xml = format!(
        "<pane {}Split=\"{}\" {}Split=\"{}\" topLeftCell=\"{}\" activePane=\"{}\" state=\"frozen\"/>",
        if cols > 0 { "x" } else { "" },
        cols,
        if rows > 0 { "y" } else { "" },
        rows,
        top_left_cell,
        active_pane
    );

    // 如果已有 <pane> 元素，替换它
    if let Some(start) = result.find("<pane ") {
        if let Some(end) = result[start..].find("/>") {
            let pane_end = start + end + 2;
            result.replace_range(start..pane_end, &pane_xml);
            return Ok(result.into_bytes());
        }
    }

    // 没有 <pane>：在 <sheetView> 内插入
    if let Some(pos) = result.find("</sheetView>") {
        result.insert_str(pos, &format!("\n      {}", pane_xml));
    } else if let Some(pos) = result.find("</sheetViews>") {
        // 有 sheetViews 但无 sheetView 不太可能，但兜底
        result.insert_str(pos, &format!(
            "\n    <sheetView tabSelected=\"1\" workbookViewId=\"0\">\n      {}\n    </sheetView>\n",
            pane_xml
        ));
    } else {
        // 无 sheetViews → 在 </worksheet> 前插入
        if let Some(pos) = result.find("</worksheet>") {
            let new_xml = format!(
                "  <sheetViews>\n    <sheetView tabSelected=\"1\" workbookViewId=\"0\">\n      {}\n    </sheetView>\n  </sheetViews>\n",
                pane_xml
            );
            result.insert_str(pos, &new_xml);
        }
    }

    Ok(result.into_bytes())
}

/// 保留式设置自动筛选。
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
        .map_err(|e| AppError::Custom(format!("无法以 zip 打开 xlsx: {}", e)))?;
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

/// 保留式移除自动筛选。
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
        .map_err(|e| AppError::Custom(format!("无法以 zip 打开 xlsx: {}", e)))?;
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

/// 字符串方式 patch autoFilter。
fn patch_auto_filter_str(xml: &[u8], new_range: Option<&str>) -> Result<Vec<u8>> {
    let s = String::from_utf8(xml.to_vec())
        .map_err(|e| AppError::Custom(format!("XML 不是合法 UTF-8: {}", e)))?;
    let mut result = s;

    // 移除所有现有的 <autoFilter .../> 元素
    loop {
        let start = result.find("<autoFilter ");
        let end = result.find("/>");
        match (start, end) {
            (Some(s), Some(e)) if s < e => {
                let af_end = e + 2;
                result.replace_range(s..af_end, "");
            }
            _ => break,
        }
    }

    // 移除所有 <autoFilter>...</autoFilter> 块
    loop {
        let start = result.find("<autoFilter ");
        let end = result.find("</autoFilter>");
        match (start, end) {
            (Some(s), Some(e)) if s < e => {
                let af_end = e + 13;
                result.replace_range(s..af_end, "");
            }
            _ => break,
        }
    }

    // 如果提供了新范围，插入
    if let Some(range) = new_range {
        if let Some(pos) = result.find("</worksheet>") {
            result.insert_str(pos, &format!("  <autoFilter ref=\"{}\"/>\n", range));
        }
    }

    Ok(result.into_bytes())
}

/// 保留式添加数据验证。
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
        .map_err(|e| AppError::Custom(format!("无法以 zip 打开 xlsx: {}", e)))?;
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

/// 构建 <dataValidation> XML 元素字符串。
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

/// 字符串方式 patch dataValidations。
fn patch_data_validation_str(xml: &[u8], new_dv: &str) -> Result<Vec<u8>> {
    let s = String::from_utf8(xml.to_vec())
        .map_err(|e| AppError::Custom(format!("XML 不是合法 UTF-8: {}", e)))?;
    let mut result = s;

    if let Some(pos) = result.find("</dataValidations>") {
        // 有现有 <dataValidations>，在闭合前插入新 DV
        result.insert_str(pos, &format!("\n    {}", new_dv));
        // 更新 count
        let old_count = result.matches("<dataValidation ").count();
        let old_count_str = format!("count=\"{}\"", old_count - 1);
        let new_count_str = format!("count=\"{}\"", old_count);
        result = result.replacen(&old_count_str, &new_count_str, 1);
    } else if let Some(pos) = result.find("<dataValidations/>") {
        // 自闭合 → 转为开放
        let replacement = format!(
            "<dataValidations count=\"1\">\n    {}\n  </dataValidations>",
            new_dv
        );
        result.replace_range(pos..pos + 18, &replacement);
    } else {
        // 不存在 → 在 </worksheet> 前插入
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

/// 保留式设置工作表保护。
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
        .map_err(|e| AppError::Custom(format!("无法以 zip 打开 xlsx: {}", e)))?;
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

/// 保留式移除工作表保护。
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
        .map_err(|e| AppError::Custom(format!("无法以 zip 打开 xlsx: {}", e)))?;
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

/// 构建 <sheetProtection> XML 元素字符串。
fn build_sheet_protection_xml_str(config: &SheetProtectionConfig) -> String {
    let opts = &config.options;
    format!(
        "<sheetProtection password=\"{}\" selectLockedCells=\"{}\" selectUnlockedCells=\"{}\" \
         formatCells=\"{}\" formatColumns=\"{}\" formatRows=\"{}\" \
         insertColumns=\"{}\" insertRows=\"{}\" insertHyperlinks=\"{}\" \
         deleteColumns=\"{}\" deleteRows=\"{}\" \
         sort=\"{}\" autoFilter=\"{}\" pivotTables=\"{}\" \
         editObjects=\"{}\" editScenarios=\"{}\"/>",
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

/// 字符串方式 patch sheetProtection。
fn patch_sheet_protection_str(xml: &[u8], new_sp: Option<&str>) -> Result<Vec<u8>> {
    let s = String::from_utf8(xml.to_vec())
        .map_err(|e| AppError::Custom(format!("XML 不是合法 UTF-8: {}", e)))?;
    let mut result = s;

    // 移除所有现有 <sheetProtection .../> 元素
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

    // 移除所有 <sheetProtection>...</sheetProtection> 块
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

    // 如果提供了新保护，在 </worksheet> 前插入
    if let Some(sp) = new_sp {
        if let Some(pos) = result.find("</worksheet>") {
            result.insert_str(pos, &format!("  {}\n", sp));
        }
    }

    Ok(result.into_bytes())
}

/// 保留式设置页面设置 — 暂未实现，留待 Phase 3。
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

/// 保留式设置工作表可见性：修改 workbook.xml 中对应 sheet 的 `state` 属性。
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
        .map_err(|e| AppError::Custom(format!("无法以 zip 打开 xlsx: {}", e)))?;

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

/// 字符串方式 patch sheet visibility。
fn patch_sheet_visibility_str(wb_xml: &[u8], sheet_name: &str, state: Option<&str>) -> Result<Vec<u8>> {
    let s = String::from_utf8(wb_xml.to_vec())
        .map_err(|e| AppError::Custom(format!("XML 不是合法 UTF-8: {}", e)))?;
    let mut result = s;

    // 找到目标 sheet 的 <sheet> 标签并修改 state 属性
    let marker = format!("name=\"{}\"", sheet_name);
    if let Some(name_pos) = result.find(&marker) {
        // 往回找 <sheet
        let prefix = &result[..name_pos];
        let tag_start = prefix.rfind("<sheet").ok_or_else(|| {
            AppError::Custom(format!("找不到 sheet 标签: {}", sheet_name))
        })?;

        // 从 tag_start 找到标签结束（> 或 />）
        let tag_end = result[tag_start..].find('>').ok_or_else(|| {
            AppError::Custom("无法找到 sheet 标签结束".to_string())
        })? + tag_start;

        let tag = &result[tag_start..=tag_end];

        // 构建新标签
        let mut new_tag = format!("<sheet name=\"{}\"", sheet_name);

        // 提取 sheetId 和 r:id
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

        // 添加 state 属性
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

/// 保留式添加工作表：修改 workbook.xml、[Content_Types].xml、workbook.xml.rels，
/// 并写入一个空的 sheet XML，其余 zip part 逐字节保留。
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
        .map_err(|e| AppError::Custom(format!("无法以 zip 打开 xlsx: {}", e)))?;

    // 读取现有文件
    let wb_xml = read_zip_entry(&mut archive, "xl/workbook.xml")?;
    let ct_xml = read_zip_entry(&mut archive, "[Content_Types].xml")?;
    let rels_xml = read_zip_entry(&mut archive, "xl/_rels/workbook.xml.rels")?;

    // 检查 sheet 是否已存在
    let wb_str = String::from_utf8_lossy(&wb_xml);
    if wb_str.contains(&format!("name=\"{}\"", sheet)) {
        return Err(AppError::SheetAlreadyExists(sheet.into()));
    }

    // 确定下一个 sheet 编号、rId、sheetId
    let next_sheet_num = next_sheet_number(&wb_str);
    let next_rid = next_rid(&wb_str);
    let next_sheet_id = next_sheet_id(&wb_str);

    let sheet_part = format!("xl/worksheets/sheet{}.xml", next_sheet_num);
    let sheet_part_name = format!("/xl/worksheets/sheet{}.xml", next_sheet_num);

    // 1. 修改 workbook.xml — 追加 <sheet>
    let new_wb = patch_add_sheet_str(&wb_xml, sheet, &next_rid, next_sheet_id)?;

    // 2. 修改 [Content_Types].xml — 追加 Override
    let new_ct = patch_add_content_type_str(&ct_xml, &sheet_part_name)?;

    // 3. 修改 workbook.xml.rels — 追加 Relationship
    let new_rels = patch_add_sheet_rel_str(&rels_xml, &next_rid, &sheet_part)?;

    // 4. 创建空的 sheet XML
    let empty_sheet = b"<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>
<worksheet xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\">
  <sheetData/>
</worksheet>
";

    // 构建变更映射
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

/// 保留式删除工作表：从 workbook.xml、[Content_Types].xml、workbook.xml.rels 中移除
/// 对应条目，并跳过该 sheet 的 XML 条目，其余 zip part 逐字节保留。
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
        .map_err(|e| AppError::Custom(format!("无法以 zip 打开 xlsx: {}", e)))?;

    let wb_xml = read_zip_entry(&mut archive, "xl/workbook.xml")?;
    let ct_xml = read_zip_entry(&mut archive, "[Content_Types].xml")?;
    let rels_xml = read_zip_entry(&mut archive, "xl/_rels/workbook.xml.rels")?;

    // 检查 sheet 是否存在并获取其 rid
    let wb_str = String::from_utf8_lossy(&wb_xml);
    if !wb_str.contains(&format!("name=\"{}\"", sheet)) {
        return Err(AppError::SheetNotFound(sheet.into()));
    }

    // 提取 sheet 的 rid
    let rid = extract_sheet_rid_str(&wb_xml, sheet)?;

    // 检查删除后是否还有 sheet
    let sheet_count = wb_str.matches("<sheet ").count();
    if sheet_count <= 1 {
        return Err(AppError::Custom("Cannot delete all sheets from a workbook".to_string()));
    }

    // 通过 rid 找到 sheet 的 part 路径
    let sheet_part = find_rel_target_str(&rels_xml, &rid)?;
    let part = if sheet_part.starts_with("xl/") {
        sheet_part.clone()
    } else if sheet_part.starts_with('/') {
        sheet_part.trim_start_matches('/').to_string()
    } else {
        format!("xl/{}", sheet_part)
    };

    // 1. 修改 workbook.xml — 移除 <sheet>
    let new_wb = patch_remove_sheet_str(&wb_xml, sheet)?;

    // 2. 修改 [Content_Types].xml — 移除对应 Override
    let new_ct = patch_remove_content_type_str(&ct_xml, &part)?;

    // 3. 修改 workbook.xml.rels — 移除对应 Relationship
    let new_rels = patch_remove_rel_str(&rels_xml, &rid)?;

    let mut changes = HashMap::new();
    changes.insert("xl/workbook.xml".to_string(), new_wb);
    changes.insert("[Content_Types].xml".to_string(), new_ct);
    changes.insert("xl/_rels/workbook.xml.rels".to_string(), new_rels);

    // 从 zip 中跳过该 sheet 的 XML 条目
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

/// 保留式重命名工作表：修改 workbook.xml 中对应 sheet 的 name 属性，
/// 其余 zip part 逐字节保留。
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
        .map_err(|e| AppError::Custom(format!("无法以 zip 打开 xlsx: {}", e)))?;

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
// R2.2 内部辅助函数
// ───────────────────────────────────────────────────────────────────────────

/// 从 workbook.xml 字符串中提取 sheet 的 rid。
fn extract_sheet_rid_str(wb: &[u8], sheet: &str) -> Result<String> {
    let s = String::from_utf8_lossy(wb);
    let marker = format!("name=\"{}\"", sheet);
    if let Some(name_pos) = s.find(&marker) {
        let prefix = &s[..name_pos];
        let tag_start = prefix.rfind("<sheet").ok_or_else(|| {
            AppError::Custom(format!("找不到 sheet 标签: {}", sheet))
        })?;
        let tag = &s[tag_start..];
        if let Some(rid_start) = tag.find("r:id=\"") {
            let rest = &tag[rid_start + 6..];
            if let Some(rid_end) = rest.find('"') {
                return Ok(rest[..rid_end].to_string());
            }
        }
        Err(AppError::Custom(format!("找不到 sheet 的 r:id: {}", sheet)))
    } else {
        Err(AppError::SheetNotFound(sheet.into()))
    }
}

/// 从 rels XML 字符串中提取 rid 对应的 Target。
fn find_rel_target_str(rels: &[u8], rid: &str) -> Result<String> {
    let s = String::from_utf8_lossy(rels);
    let marker = format!("Id=\"{}\"", rid);
    if let Some(id_pos) = s.find(&marker) {
        let prefix = &s[..id_pos];
        let tag_start = prefix.rfind("<Relationship").ok_or_else(|| {
            AppError::Custom(format!("找不到 Relationship 标签: {}", rid))
        })?;
        let tag = &s[tag_start..];
        if let Some(t_start) = tag.find("Target=\"") {
            let rest = &tag[t_start + 8..];
            if let Some(t_end) = rest.find('"') {
                let target = rest[..t_end].to_string();
                // 归一化
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
        Err(AppError::Custom(format!("找不到 rid {} 的 Target", rid)))
    } else {
        Err(AppError::Custom(format!("找不到 rid: {}", rid)))
    }
}

/// 确定下一个 sheet 编号（从现有 zip 条目中找最大值 +1）。
fn next_sheet_number(wb_xml: &str) -> u32 {
    let mut max_n = 0u32;
    // 查找所有 sheetId="N" 模式
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

/// 确定下一个 rId（从 workbook.xml 中找 rIdN 的最大值 +1）。
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

/// 确定下一个 sheetId。
fn next_sheet_id(wb_xml: &str) -> u32 {
    next_sheet_number(wb_xml)
}

/// 字符串方式：在 workbook.xml 中追加 <sheet>。
fn patch_add_sheet_str(wb: &[u8], sheet: &str, rid: &str, sheet_id: u32) -> Result<Vec<u8>> {
    let s = String::from_utf8(wb.to_vec())
        .map_err(|e| AppError::Custom(format!("XML 不是合法 UTF-8: {}", e)))?;
    let mut result = s;

    // 在 </sheets> 前插入
    if let Some(pos) = result.find("</sheets>") {
        let new_sheet = format!("\n    <sheet name=\"{}\" sheetId=\"{}\" r:id=\"{}\"/>", sheet, sheet_id, rid);
        result.insert_str(pos, &new_sheet);
        Ok(result.into_bytes())
    } else {
        Err(AppError::Custom("workbook.xml 中找不到 </sheets>".to_string()))
    }
}

/// 字符串方式：从 workbook.xml 中移除 <sheet>。
fn patch_remove_sheet_str(wb: &[u8], sheet: &str) -> Result<Vec<u8>> {
    let s = String::from_utf8(wb.to_vec())
        .map_err(|e| AppError::Custom(format!("XML 不是合法 UTF-8: {}", e)))?;
    let mut result = s;

    let marker = format!("name=\"{}\"", sheet);
    if let Some(name_pos) = result.find(&marker) {
        let prefix = &result[..name_pos];
        let tag_start = prefix.rfind("<sheet").ok_or_else(|| {
            AppError::Custom(format!("找不到 sheet 标签: {}", sheet))
        })?;
        // 找到标签结束：> 或 />
        let rest = &result[tag_start..];
        let tag_end = rest.find('>').ok_or_else(|| {
            AppError::Custom("找不到 sheet 标签结束".to_string())
        })? + tag_start + 1;
        result.replace_range(tag_start..tag_end, "");
        Ok(result.into_bytes())
    } else {
        Err(AppError::SheetNotFound(sheet.into()))
    }
}

/// 字符串方式：在 workbook.xml 中重命名 sheet。
fn patch_rename_sheet_str(wb: &[u8], old_name: &str, new_name: &str) -> Result<Vec<u8>> {
    let s = String::from_utf8(wb.to_vec())
        .map_err(|e| AppError::Custom(format!("XML 不是合法 UTF-8: {}", e)))?;
    let mut result = s;

    let old_marker = format!("name=\"{}\"", old_name);
    let new_marker = format!("name=\"{}\"", new_name);
    result = result.replacen(&old_marker, &new_marker, 1);

    Ok(result.into_bytes())
}

/// 字符串方式：在 [Content_Types].xml 中追加 Override。
fn patch_add_content_type_str(ct: &[u8], part_name: &str) -> Result<Vec<u8>> {
    let s = String::from_utf8(ct.to_vec())
        .map_err(|e| AppError::Custom(format!("XML 不是合法 UTF-8: {}", e)))?;
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
        Err(AppError::Custom("[Content_Types].xml 中找不到 </Types>".to_string()))
    }
}

/// 字符串方式：从 [Content_Types].xml 中移除 Override。
fn patch_remove_content_type_str(ct: &[u8], part: &str) -> Result<Vec<u8>> {
    let s = String::from_utf8(ct.to_vec())
        .map_err(|e| AppError::Custom(format!("XML 不是合法 UTF-8: {}", e)))?;
    let mut result = s;

    // part 可能是 "xl/worksheets/sheet2.xml"，需要匹配 "/xl/worksheets/sheet2.xml"
    let rel_part = if part.starts_with("xl/") {
        format!("/{}", part)
    } else {
        part.to_string()
    };

    let marker = format!("PartName=\"{}\"", rel_part);
    if let Some(pn_pos) = result.find(&marker) {
        let prefix = &result[..pn_pos];
        let tag_start = prefix.rfind("<Override").ok_or_else(|| {
            AppError::Custom(format!("找不到 Override 标签: {}", part))
        })?;
        let rest = &result[tag_start..];
        let tag_end = rest.find("/>").ok_or_else(|| {
            AppError::Custom("找不到 Override 标签结束".to_string())
        })? + tag_start + 2;
        result.replace_range(tag_start..tag_end, "");
        Ok(result.into_bytes())
    } else {
        // 找不到也 OK，继续
        Ok(ct.to_vec())
    }
}

/// 字符串方式：在 workbook.xml.rels 中追加 Relationship。
fn patch_add_sheet_rel_str(rels: &[u8], rid: &str, target: &str) -> Result<Vec<u8>> {
    let s = String::from_utf8(rels.to_vec())
        .map_err(|e| AppError::Custom(format!("XML 不是合法 UTF-8: {}", e)))?;
    let mut result = s;

    // target 可以是 "xl/worksheets/sheet3.xml"，但 rels 中通常用相对路径 "worksheets/sheet3.xml"
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
        Err(AppError::Custom("rels 中找不到 </Relationships>".to_string()))
    }
}

/// 字符串方式：从 rels 中移除 Relationship。
fn patch_remove_rel_str(rels: &[u8], rid: &str) -> Result<Vec<u8>> {
    let s = String::from_utf8(rels.to_vec())
        .map_err(|e| AppError::Custom(format!("XML 不是合法 UTF-8: {}", e)))?;
    let mut result = s;

    let marker = format!("Id=\"{}\"", rid);
    if let Some(id_pos) = result.find(&marker) {
        let prefix = &result[..id_pos];
        let tag_start = prefix.rfind("<Relationship").ok_or_else(|| {
            AppError::Custom(format!("找不到 Relationship 标签: {}", rid))
        })?;
        let rest = &result[tag_start..];
        let tag_end = rest.find("/>").ok_or_else(|| {
            AppError::Custom("找不到 Relationship 标签结束".to_string())
        })? + tag_start + 2;
        result.replace_range(tag_start..tag_end, "");
        Ok(result.into_bytes())
    } else {
        Err(AppError::Custom(format!("找不到 rid: {}", rid)))
    }
}

// ───────────────────────────────────────────────────────────────────────────
// 内部辅助
// ───────────────────────────────────────────────────────────────────────────

/// 把 zip 中除 `part` 外的所有条目逐字节复制，并用 `new_xml` 替换 `part` 的内容。
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
            .map_err(|e| AppError::Custom(format!("读取 zip 条目失败: {}", e)))?;
        let name = zf.name().to_string();
        let opts = zf.options();
        if name == part {
            zw.start_file(&name, opts)
                .map_err(|e| AppError::Custom(format!("写入 zip 条目失败: {}", e)))?;
            zw.write_all(new_xml).map_err(AppError::Io)?;
        } else {
            buf.clear();
            zf.read_to_end(&mut buf).map_err(AppError::Io)?;
            zw.start_file(&name, opts)
                .map_err(|e| AppError::Custom(format!("写入 zip 条目失败: {}", e)))?;
            zw.write_all(&buf).map_err(AppError::Io)?;
        }
    }
    zw.finish()
        .map_err(|e| AppError::Custom(format!("完成 zip 写入失败: {}", e)))?;
    fs::rename(&tmp, path).map_err(AppError::Io)?;
    Ok(())
}

/// 多条目版本的 repackage：支持同时修改/新增多个 part，并跳过指定条目。
///
/// - `changes`: 需要修改或新增的条目名 → 新内容
/// - `skip_parts`: 需要跳过的条目名（不写入新 zip）
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

    // 预先记录所有需要新增的条目（不在原 zip 中）
    let mut added = std::collections::HashSet::new();

    for i in 0..n {
        let mut zf = archive
            .by_index(i)
            .map_err(|e| AppError::Custom(format!("读取 zip 条目失败: {}", e)))?;
        let name = zf.name().to_string();
        let opts = zf.options();

        // 检查是否在跳过列表中
        if skip_parts.iter().any(|sp| *sp == name) {
            continue;
        }

        // 检查是否有变更
        if let Some(new_content) = changes.get(&name) {
            zw.start_file(&name, opts)
                .map_err(|e| AppError::Custom(format!("写入 zip 条目失败: {}", e)))?;
            zw.write_all(new_content).map_err(AppError::Io)?;
            added.insert(name);
        } else {
            buf.clear();
            zf.read_to_end(&mut buf).map_err(AppError::Io)?;
            zw.start_file(&name, opts)
                .map_err(|e| AppError::Custom(format!("写入 zip 条目失败: {}", e)))?;
            zw.write_all(&buf).map_err(AppError::Io)?;
        }
    }

    // 写入新增的条目（不在原 zip 中的）
    for (name, content) in changes {
        if !added.contains(name) {
            // 使用默认 options
            zw.start_file(name, default_opt)
                .map_err(|e| AppError::Custom(format!("写入新增 zip 条目失败: {}", e)))?;
            zw.write_all(content).map_err(AppError::Io)?;
        }
    }

    zw.finish()
        .map_err(|e| AppError::Custom(format!("完成 zip 写入失败: {}", e)))?;
    fs::rename(&tmp, path).map_err(AppError::Io)?;
    Ok(())
}

/// 记录写操作到审计历史（非致命）。
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

/// 判断单元格原始字节中是否包含 `<f`（公式标记）。
fn has_formula(raw: &[u8]) -> bool {
    raw.windows(3).any(|w| w == b"<f>") || raw.windows(4).any(|w| w == b"<f ")
}

/// 从单元格原始字节中移除 `<v>...</v>` 元素。
fn strip_v_element(raw: &[u8]) -> Vec<u8> {
    // 查找 <v 或 <v> 起始位置
    let mut result = Vec::with_capacity(raw.len());
    let mut i = 0;
    while i < raw.len() {
        if (raw[i..].starts_with(b"<v>") || raw[i..].starts_with(b"<v ")) && i > 0 {
            // 向前确认这不是 <f> 或其它标签的一部分
            let prev = raw[i - 1];
            if prev == b'>' || prev == b'/' || prev == b'"' || prev == b'\'' {
                // 跳过 <v...> 标签
                i += 2; // skip "<v"
                while i < raw.len() && raw[i] != b'>' {
                    i += 1;
                }
                i += 1; // skip '>'
                // 跳过内容直到 </v>
                while i + 4 < raw.len() && !raw[i..].starts_with(b"</v>") {
                    i += 1;
                }
                // 跳过 </v> 或 </v...>
                if i + 4 <= raw.len() && raw[i..].starts_with(b"</v>") {
                    i += 4;
                } else {
                    // 异常情况：不匹配，让后续字符正常通过
                    // 这会回退太多，改用更保守的策略
                    break;
                }
                continue;
            }
        }
        result.push(raw[i]);
        i += 1;
    }
    // 如果上述循环因异常 break 退出，退回到安全路径：直接返回原始字节
    if i < raw.len() {
        return raw.to_vec();
    }
    result
}

// ───────────────────────────────────────────────────────────────────────────
// sheet 名称 → zip part 解析
// ───────────────────────────────────────────────────────────────────────────

fn read_zip_entry(archive: &mut ZipArchive<File>, name: &str) -> Result<Vec<u8>> {
    let mut zf = archive
        .by_name(name)
        .map_err(|e| AppError::Custom(format!("缺失 zip 条目 {}: {}", name, e)))?;
    let mut buf = Vec::new();
    zf.read_to_end(&mut buf).map_err(AppError::Io)?;
    Ok(buf)
}

/// 通过 `xl/workbook.xml` + `xl/_rels/workbook.xml.rels` 把 sheet 名映射到
/// `xl/worksheets/sheetN.xml`。
fn resolve_sheet_part(archive: &mut ZipArchive<File>, sheet: &str) -> Result<String> {
    let wb = read_zip_entry(archive, "xl/workbook.xml")?;
    let rid = find_sheet_rid(&wb, sheet).ok_or_else(|| {
        AppError::Custom(format!("workbook 中找不到 sheet '{}'", sheet))
    })?;
    let rels = read_zip_entry(archive, "xl/_rels/workbook.xml.rels")?;
    let target = find_rel_target(&rels, &rid)
        .ok_or_else(|| AppError::Custom(format!("找不到关系 {} 的目标", rid)))?;
    // Relationship Target 有两种真实形态，必须统一规整为 zip 内条目名（无前导斜杠、以 xl/ 开头）：
    //   - 绝对路径（部分工具产出，含或不合前导斜杠）：/xl/worksheets/sheet1.xml
    //   - 相对路径（相对于 xl/_rels/）：worksheets/sheet1.xml 或 xl/worksheets/sheet1.xml
    // 注意：zip 条目名不带前导斜杠，若直接返回 "/xl/..." 会导致 by_name 找不到条目，
    // 写入提前报错、值无法落盘（表现为 cell write 后读回为 None）。
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
                // 注意：quick-xml 事件经 Deref 只暴露标签内部内容（如 `sheet name=...`），
                // 不含 `<`，故以 `sheet` 而非 `<sheet` 做前缀判定。
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
                // 同上：deref 内容为 `Relationship Id=...`，不含 `<`。
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

// ───────────────────────────────────────────────────────────────────────────
// sheetData 的「仅重写」编辑
// ───────────────────────────────────────────────────────────────────────────

/// (before, inner, after, self_closed)
/// - before：`<sheetData ...>` 起始标签（含）之前的所有内容（self-closed 时为 `<sheetData` 之前）
/// - inner：`<sheetData>` 与 `</sheetData>` 之间的单元格数据
/// - after：`</sheetData>` 之后（self-closed 时为 `/>` 之后）
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
        // <sheetData/> 自闭合：inner 为空
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
    /// 起始标签原文，例如 `<row r="1" spans="1:26">`（已规整为带 `>` 的开放标签）
    open_tag: String,
    cells: BTreeMap<u16, Cell>,
}

struct Cell {
    /// `<c ...>...</c>` 或 `<c .../>` 的逐字节原文（编辑时整体替换）
    raw: Vec<u8>,
}

/// 把 quick-xml 的 `Event` 重建为完整 XML 标记字节。
///
/// quick-xml 事件经 `Deref` 只暴露标签**内部内容**（如 `c r="A1" s="3"`），
/// 不含 `<` `>`，因此这里手工补全括号，得到 `<c r="A1" s="3">` / `</c>` / `<c .../>` 等，
/// 供 `starts_with` 判定与 `cell_buf` 字节级重建使用。
fn event_markup(event: &Event) -> Vec<u8> {
    // `Event` 实现 `Deref<Target = [u8]>`，给出标签内部内容（不含括号）。
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
        // 其余（Text / Comment / PI / Decl / DocType / Eof）按内部内容原样返回。
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
                    // 行号（1-based）→ 0-based 作为 map key
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
                    // `<f>` / `<v>` / `<is>` / `<t>` 等单元格内部元素：原样累积
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

    // 收尾：若文件在 `</sheetData>` 前没有显式 `</row>`（不规范但偶发）
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
                    // 已存在单元格：保留其样式索引 `s`，整体重建内容
                    let style = extract_attr(&cell.raw, b"s");
                    cell.raw = rebuild_cell(&rref, style.as_deref(), cd).into_bytes();
                } else {
                    // 同行新列：插入（无样式）
                    row_model.cells.insert(
                        *col,
                        Cell {
                            raw: rebuild_cell(&rref, None, cd).into_bytes(),
                        },
                    );
                }
            }
            None => {
                // 整行不存在：新建行 + 单元格
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

/// 由最终 model 计算 `<dimension>` 的包围盒引用（如 `A1:Z50`）。
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

/// 在原 `<dimension ref="..."/>` 中替换 ref；若不存在则在 `<sheetData` 前插入。
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
    // 无 dimension 元素：在 <sheetData 之前插入（符合 schema 顺序）。
    if let Some(pos) = before.windows(10).position(|w| &w[..10] == b"<sheetData") {
        let ins = format!("<dimension ref=\"{}\"/>", refstr);
        before.splice(pos..pos, ins.into_bytes());
    }
}

/// 仅重写 `<sheetData>` 内部；其它 part 字节不变。
fn patch_sheet_xml(xml: &[u8], edits: &[(u32, u16, CellData)]) -> Result<Vec<u8>> {
    let (before, inner, after, self_closed) = match sheetdata_spans(xml) {
        Some(x) => x,
        None => return Ok(xml.to_vec()),
    };
    let model = parse_sheetdata(inner);
    let model = apply_edits(model, edits);
    let new_inner = serialize_sheet(&model);

    // T5.16：把 <dimension ref="..."> 更新为最终单元格的包围盒，避免写入远端
    // 单元格（如 Z50）后 dimension 仍停留在原始范围。
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
// 单元格重建（使用内联字符串，避免改动 sharedStrings.xml）
// ───────────────────────────────────────────────────────────────────────────

/// 返回 `(类型 t, 内部 XML)`。字符串统一用 `inlineStr`，从而不动 sharedStrings。
///
/// 值以 `=` 开头时按 Excel 语义识别为**公式**（CLI 的 `cell write =SUM(...)` 不会
/// 显式设置 `formula` 字段），此时剥离前导 `=` 写入 `<f>`，且不携带缓存值。
fn cell_xml(cd: &CellData) -> (Option<&'static str>, String) {
    // 解析「公式 / 值」：formula 字段优先；否则值以 "=" 开头即视为公式。
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
// 极小工具
// ───────────────────────────────────────────────────────────────────────────

/// 从标签原文中取 `key="value"` 的值（兼容单/双引号）。
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

/// 从 `<c r="A1">` 的原文解析出 0-based 列索引。
fn extract_col(raw: &[u8]) -> u16 {
    extract_attr(raw, b"r")
        .and_then(|r| parse_cell_ref(&r).ok())
        .map(|(_, col)| col)
        .unwrap_or(0)
}

// ───────────────────────────────────────────────────────────────────────────
// Phase 3 — 通用 fallback：preserve_all_parts_transfer
// ───────────────────────────────────────────────────────────────────────────

/// 非数据 zip 部件的前缀模式。
/// 以这些模式开头的 zip 条目从源 zip 拷贝到重建后的 zip。
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

/// 判断 zip 条目是否为非数据部件（需要从源 zip 保留）。
fn is_non_data_part(name: &str) -> bool {
    NON_DATA_PREFIXES.iter().any(|p| name.starts_with(p))
}

/// 从源 zip 读取所有条目到 HashMap。
fn read_zip_map(path: &str) -> Result<HashMap<String, Vec<u8>>> {
    use std::io::Read;
    let file = File::open(path).map_err(AppError::Io)?;
    let mut archive = ZipArchive::new(file)
        .map_err(|e| AppError::Custom(format!("无法以 zip 打开: {}", e)))?;
    let mut map = HashMap::new();
    for i in 0..archive.len() {
        let mut zf = archive
            .by_index(i)
            .map_err(|e| AppError::Custom(format!("读取 zip 条目失败: {}", e)))?;
        let name = zf.name().to_string();
        let mut buf = Vec::new();
        zf.read_to_end(&mut buf).map_err(AppError::Io)?;
        map.insert(name, buf);
    }
    Ok(map)
}

/// 写入 zip 条目到文件。
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
    // 保持原始顺序：先输出源的顺序，但以排好序的 key 列表遍历
    for (name, content) in entries {
        zw.start_file(name, opt)
            .map_err(|e| AppError::Custom(format!("写入 zip 条目失败: {}", e)))?;
        zw.write_all(content).map_err(AppError::Io)?;
    }
    zw.finish()
        .map_err(|e| AppError::Custom(format!("完成 zip 写入失败: {}", e)))?;
    fs::rename(&tmp, path).map_err(AppError::Io)?;
    Ok(())
}

/// 从 workbook.xml 解析 sheet 名称到 part 路径的映射。
fn parse_sheet_name_to_part(wb_xml: &[u8], rels_xml: &[u8]) -> HashMap<String, String> {
    let wb_str = String::from_utf8_lossy(wb_xml);
    let mut result = HashMap::new();

    // 解析所有 <sheet name="..." r:id="...">
    let mut pos = 0;
    while let Some(sheet_start) = wb_str[pos..].find("<sheet ") {
        let tag = &wb_str[pos + sheet_start..];
        let tag_end = tag.find('>').unwrap_or(tag.len());
        let tag_content = &tag[..tag_end];

        // 提取 name 和 r:id
        let name = extract_attr_str(tag_content, "name");
        let rid = extract_attr_str(tag_content, "r:id");

        if let (Some(name), Some(rid)) = (name, rid) {
            // 从 rels 中找到 rid 对应的 Target
            if let Some(target) = find_rel_target_str(rels_xml, &rid).ok() {
                result.insert(name, target);
            }
        }
        pos += sheet_start + 1;
    }
    result
}

/// 从字符串中提取属性值。
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

/// 提取 worksheet XML 中除了 <sheetData> 之外的所有顶层元素。
/// 返回 (元素名, 完整 XML 片段) 列表。
fn extract_non_data_elements(xml: &[u8]) -> Vec<(String, String)> {
    let s = String::from_utf8_lossy(xml);
    let mut elements = Vec::new();

    // 找到 <worksheet 和 </worksheet> 之间的内容
    let root_start = s.find("<worksheet").unwrap_or(0);
    let root_end = s.find("</worksheet>").unwrap_or(s.len());

    let body = &s[root_start..root_end];

    // 按顶层标签分割
    let mut depth = 0;
    let mut current_tag = String::new();
    let mut current_content = String::new();
    let mut in_tag = false;

    for ch in body.chars() {
        current_content.push(ch);
        if ch == '<' {
            depth += 1;
            in_tag = true;
            current_tag.clear();
        } else if ch == '>' {
            depth -= 0; // depth already incremented at '<'
            in_tag = false;
            // 提取标签名
            if !current_tag.is_empty() && current_tag.as_bytes()[0] != b'/' {
                // 提取标签名（不含属性和命名空间）
                let tag_name = current_tag
                    .split_whitespace()
                    .next()
                    .unwrap_or("")
                    .to_string();
                if !tag_name.is_empty() && !tag_name.starts_with('?') && tag_name != "sheetData" {
                    elements.push((tag_name, current_content.clone()));
                }
            }
            if current_tag == "/sheetData" || current_tag.starts_with("/sheetData ") {
                // 结束标签，不记录
            }
        } else if in_tag {
            current_tag.push(ch);
        } else if ch == '<' {
            // 开始新标签
            current_content.clear();
            current_content.push(ch);
            current_tag.clear();
            in_tag = true;
        }
    }

    elements
}

/// 检查 rebuilt XML 中是否已包含指定元素名。
fn has_element(xml: &str, tag_name: &str) -> bool {
    // 寻找开放标签（不含 </）
    let open = format!("<{} ", tag_name);
    let open2 = format!("<{}>", tag_name);
    let open3 = format!("<{}/>", tag_name);
    xml.contains(&open) || xml.contains(&open2) || xml.contains(&open3)
}

/// 合并 worksheet 中的非数据元素：从源 XML 中提取非数据元素，
/// 插入到重建后的 XML 的 </sheetData> 之后。
fn merge_worksheet_xml(source_xml: &[u8], rebuilt_xml: &[u8]) -> Vec<u8> {
    let rebuilt_str = String::from_utf8_lossy(rebuilt_xml);
    let source_str = String::from_utf8_lossy(source_xml);

    // 1. 提取源 XML 中 <sheetData> 之后、</worksheet> 之前的内容
    let source_after_sd = if let Some(sd_end) = source_str.find("</sheetData>") {
        let after = &source_str[sd_end + 12..];
        if let Some(ws_end) = after.rfind("</worksheet>") {
            &after[..ws_end]
        } else {
            ""
        }
    } else {
        ""
    };

    // 2. 解析源 XML 中非数据元素的标签名
    let source_elements = extract_non_data_elements(source_xml);

    // 3. 在重建后的 XML 中找到 </sheetData> 之后的位置
    let sd_end_pos = rebuilt_str.find("</sheetData>");

    match sd_end_pos {
        Some(pos) => {
            let insert_pos = pos + 12; // after </sheetData>
            let after_sd = &rebuilt_str[insert_pos..];

            // 找到 </worksheet> 的位置
            let ws_end_pos = after_sd.find("</worksheet>").unwrap_or(after_sd.len());
            let existing_after = &after_sd[..ws_end_pos];

            // 收集需要插入的 XML 片段
            let mut to_insert = String::new();

            for (tag_name, xml_fragment) in &source_elements {
                // 跳过 <sheetData>, <sheetViews>, <pageMargins>, <pageSetup>
                // sheetViews/pageMargins/pageSetup 由 rust_xlsxwriter 生成，保留重建版本
                if tag_name == "sheetData" || tag_name == "sheetViews"
                    || tag_name == "pageMargins" || tag_name == "pageSetup"
                {
                    continue;
                }
                // 检查是否已存在于重建版本中
                if !has_element(existing_after, tag_name) {
                    to_insert.push_str(xml_fragment);
                    to_insert.push('\n');
                }
            }

            if to_insert.is_empty() {
                // 无需修改
                rebuilt_xml.to_vec()
            } else {
                let mut result = rebuilt_str[..insert_pos].to_string();
                result.push('\n');
                result.push_str(&to_insert);
                result.push_str(after_sd);
                result.into_bytes()
            }
        }
        None => {
            // 重建 XML 中没有 <sheetData>，无法合并
            rebuilt_xml.to_vec()
        }
    }
}

/// 全量重建时保留所有非数据 zip 部件。
///
/// 流程：
/// 1. 打开源 zip 和重建后的新 zip
/// 2. 从源 zip 拷贝非数据部件到新 zip（覆盖新 zip 的同名部件）：
///    - styles.xml、charts/*.xml、drawings/*.xml、comments*.xml、media/* 等
/// 3. 从源 zip 拷贝 worksheet XML 中的非数据元素到新 zip 的 worksheet XML：
///    - <mergeCells>、<dataValidations>、<conditionalFormatting>、<autoFilter> 等
/// 4. 更新新 zip 的 [Content_Types].xml
/// 5. 保存新 zip
///
/// # Arguments
/// * `src_path` - 原始 xlsx 文件路径（修改前）
/// * `rebuilt_path` - rust_xlsxwriter 重建后的 xlsx 文件路径（已丢失非数据部件）
///                    此文件会被原地修改
pub fn preserve_all_parts_transfer(src_path: &str, rebuilt_path: &str) -> Result<()> {
    // 1. 读取源 zip 和重建后的 zip
    let src_entries = read_zip_map(src_path)?;
    let rebuilt_entries = read_zip_map(rebuilt_path)?;

    // 构建输出条目
    let mut output = HashMap::new();

    // 2. 收集重建 zip 中的数据部件名称
    let mut data_part_names: Vec<String> = Vec::new();

    // 3. 处理重建后的条目
    for (name, content) in &rebuilt_entries {
        // 对于 worksheet XML，合并非数据元素
        if name.starts_with("xl/worksheets/sheet") && name.ends_with(".xml") {
            // 找到对应的源 worksheet XML
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

    // 4. 从源 zip 拷贝非数据部件（不在重建 zip 中的）
    let mut added_content_types: Vec<String> = Vec::new();
    for (name, content) in &src_entries {
        if is_non_data_part(name) && !output.contains_key(name) {
            output.insert(name.clone(), content.clone());
            // 记录需要添加到 Content_Types 的部件
            // 计算 PartName（格式为 /xl/...）
            let part_name = if name.starts_with("xl/") {
                format!("/{}", name)
            } else {
                format!("/xl/{}", name)
            };
            added_content_types.push(part_name);
        }
    }

    // 5. 对于 styles.xml，始终使用源版本（保留自定义样式）
    if let Some(src_styles) = src_entries.get("xl/styles.xml") {
        output.insert("xl/styles.xml".to_string(), src_styles.clone());
    }

    // 6. 更新 [Content_Types].xml：添加新增的非数据部件
    if !added_content_types.is_empty() {
        if let Some(ct_content) = output.get("[Content_Types].xml") {
            let mut ct_str = String::from_utf8_lossy(ct_content).to_string();
            let mut modified = false;
            for part_name in &added_content_types {
                // 检查是否已存在
                if !ct_str.contains(&format!("PartName=\"{}\"", part_name)) {
                    // 确定 ContentType
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

    // 7. 写入输出 zip
    write_zip_map(rebuilt_path, &output)?;

    Ok(())
}

/// 根据部件路径猜测 ContentType。
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
        // 根据扩展名判断
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
        assert!(s.contains("r=\"A1\" s=\"3\""), "A1 样式丢失: {}", s);
        assert!(
            s.contains("r=\"Z1\"") && s.contains("t=\"inlineStr\"") && s.contains("<t>x</t>"),
            "Z1 未插入: {}",
            s
        );
        assert!(s.contains("r=\"B1\"") && s.contains("<v>2</v>"), "B1 丢失: {}", s);
        assert!(s.contains("r=\"A2\"") && s.contains("<v>3</v>"), "A2 丢失: {}", s);
        let p_a = s.find("r=\"A1\"").unwrap();
        let p_b = s.find("r=\"B1\"").unwrap();
        let p_z = s.find("r=\"Z1\"").unwrap();
        assert!(p_a < p_b && p_b < p_z, "单元格顺序错误");
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
            "编辑后样式丢失: {}",
            s
        );
        assert!(!s.contains("<v>9</v>"), "旧值未清除");
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
    // 端到端：保留式写入必须逐字节保留所有「非目标」zip part，
    // 且目标 sheet 内的样式 / 合并 / 数据验证 / 冻结窗格不得被抹掉。
    // 对应 T3.03–09、T4.10 的根因（旧 modify_file_with_wb 全量重建丢失这些特性）。
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

    // 含可辨识标记 DISTINCT_STYLE_9999，用于验证 styles.xml 被原样保留。
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

    // 含可辨识标记 DISTINCT_COMMENT_A1。
    const FIXTURE_COMMENTS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<comments xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <authors><author>tester</author></authors>
  <commentList>
    <comment ref="A1" authorId="0"><text><t>DISTINCT_COMMENT_A1</t></text></comment>
  </commentList>
</comments>"#;

    // 含可辨识标记 DISTINCT_DRAWING / DISTINCT_CHART_TITLE。
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

    // 目标 sheet：含冻结窗格(pane state=frozen)、合并单元格、数据验证、以及带 s="3" 样式索引的单元格。
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

    // 与 build_fixture 相同，但 workbook.xml.rels 中 worksheet 的 Target 使用
    // **绝对路径**（含前导斜杠）：Target="/xl/worksheets/sheet1.xml"。
    // 这是部分真实工具（如本项目 verify/data/sales.xlsx）产出的形态，曾经因
    // resolve_sheet_part 原样返回带前导斜杠的路径导致 by_name 找不到条目、
    // 写入提前报错、值无法落盘（表现为 cell write 后读回为 None）。本测试锁死该修复。
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
        // 在 Z10（行索引 9、列索引 25）写入字符串 "x"，模拟 T3.03/05 等 cell write 命令。
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
        assert!(result.success, "保留式写入应成功");

        let after: HashMap<String, Vec<u8>> = read_zip_map(&path_str);
        let target = "xl/worksheets/sheet1.xml";

        // 1) 所有「非目标」part 解压内容逐字节一致。
        for (name, content) in &before {
            if name == target {
                continue;
            }
            let a = after
                .get(name)
                .unwrap_or_else(|| panic!("非目标 part 丢失: {}", name));
            assert_eq!(a, content, "非目标 part 内容被改动（丢失源文件特性）: {}", name);
        }

        // 2) 富特性标记全部存活（样式 / 注释 / 绘图 / 图表）。
        let styles = String::from_utf8_lossy(after.get("xl/styles.xml").unwrap());
        assert!(styles.contains("DISTINCT_STYLE_9999"), "styles.xml 未保留");
        let comments = String::from_utf8_lossy(after.get("xl/comments1.xml").unwrap());
        assert!(comments.contains("DISTINCT_COMMENT_A1"), "comments1.xml 未保留");
        let drawing = String::from_utf8_lossy(after.get("xl/drawings/drawing1.xml").unwrap());
        assert!(drawing.contains("DISTINCT_DRAWING"), "drawing1.xml 未保留");
        let chart = String::from_utf8_lossy(after.get("xl/charts/chart1.xml").unwrap());
        assert!(chart.contains("DISTINCT_CHART_TITLE"), "chart1.xml 未保留");

        // 3) 目标 sheet：样式化单元格、合并、数据验证、冻结窗格均保留，新单元格已写入。
        let sheet = String::from_utf8_lossy(after.get(target).unwrap());
        assert!(sheet.contains("s=\"3\"") && sheet.contains("t=\"s\"") && sheet.contains("<v>0</v>"),
            "A1 样式/共享字符串引用丢失: {}", sheet);
        assert!(sheet.contains("mergeCells") && sheet.contains("C1:E1"), "合并单元格丢失");
        assert!(sheet.contains("dataValidation") && sheet.contains("F1:F10"), "数据验证丢失");
        assert!(sheet.contains("state=\"frozen\""), "冻结窗格丢失");
        assert!(
            sheet.contains("r=\"Z10\"") && sheet.contains("t=\"inlineStr\"") && sheet.contains("<t>x</t>"),
            "Z10 新单元格未写入: {}", sheet
        );
        // 旧值 100 / 200 仍在（未被清掉）。
        assert!(sheet.contains("<v>100</v>") && sheet.contains("<v>200</v>"), "原有数值丢失");
        // 行顺序合法：r=1 在 r=2 之前，r=2 在 r=10 之前。
        let p1 = sheet.find("r=\"1\"").unwrap();
        let p2 = sheet.find("r=\"2\"").unwrap();
        let p10 = sheet.find("r=\"10\"").unwrap();
        assert!(p1 < p2 && p2 < p10, "行顺序非法");
    }

    // 绝对路径 Target（/xl/worksheets/sheet1.xml）形态：曾因前导斜杠导致写入失败，
    // 现必须成功且富特性/新单元格都正确落盘（回归锁死）。
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
            .expect("绝对路径 Target 下保留式写入应成功");
        assert!(result.success);

        let after: HashMap<String, Vec<u8>> = read_zip_map(&path_str);
        let target = "xl/worksheets/sheet1.xml";
        let sheet = String::from_utf8_lossy(after.get(target).unwrap());
        // 新单元格写入成功（证明 resolve 正确归一化了前导斜杠）。
        assert!(
            sheet.contains("r=\"Z10\"") && sheet.contains("t=\"inlineStr\"") && sheet.contains("<t>x</t>"),
            "绝对路径 Target 下 Z10 未写入: {}", sheet
        );
        // 富特性与样式化单元格均保留。
        assert!(sheet.contains("s=\"3\"") && sheet.contains("state=\"frozen\""),
            "绝对路径 Target 下样式/冻结窗格丢失: {}", sheet);
        assert!(String::from_utf8_lossy(after.get("xl/styles.xml").unwrap()).contains("DISTINCT_STYLE_9999"),
            "styles.xml 未保留");
    }

    // T5.16：写入远端单元格（Z50）后，<dimension> 必须扩展到覆盖它，而非停留在
    // 原始范围（如 A1:G11）。
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
        let start = sheet.find("<dimension").expect("应有 dimension 元素");
        let end = start + sheet[start..].find("/>").expect("dimension 应自闭合");
        let dim_tag = &sheet[start..=end];
        assert!(
            dim_tag.contains("Z50"),
            "dimension 未扩展到远端单元格 Z50: {}",
            dim_tag
        );
    }

    // T5.19：以 "=" 开头的值按 Excel 语义识别为公式，写入 <f> 而非字面文本。
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
            "前导 = 的值未识别为公式: {}",
            sheet
        );
    }
}
