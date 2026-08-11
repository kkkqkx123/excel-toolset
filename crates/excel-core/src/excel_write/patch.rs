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

use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{Cursor, Read, Write};
use std::path::Path;

use quick_xml::events::Event;
use quick_xml::escape::escape;
use quick_xml::Reader;
use zip::{ZipArchive, ZipWriter};

use crate::security::{append_history_entry, compute_file_hash, create_backup};
use crate::types::{
    AppError, CellData, CellDataType, Result, SecurityParams, WorkbookHistoryEntry, WriteResult,
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
    let tmp = Path::new(path).with_extension("patch_tmp");
    {
        let tf = File::create(&tmp).map_err(AppError::Io)?;
        let mut zw = ZipWriter::new(tf);
        let mut buf = Vec::new();
        let n = archive.len();
        for i in 0..n {
            let mut zf = archive
                .by_index(i)
                .map_err(|e| AppError::Custom(format!("读取 zip 条目失败: {}", e)))?;
            let name = zf.name().to_string();
            // 复用源条目的压缩方式与权限，保证产出文件结构一致。
            let opts = zf.options();
            if name == part {
                zw.start_file(&name, opts)
                    .map_err(|e| AppError::Custom(format!("写入 zip 条目失败: {}", e)))?;
                zw.write_all(&new_xml).map_err(AppError::Io)?;
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
    }

    fs::rename(&tmp, path).map_err(AppError::Io)?;

    let new_hash = compute_file_hash(path).map_err(AppError::Io)?;

    // T2.30：记录写操作到审计历史（非致命，失败不影响主流程）。
    if !params.dry_run {
        let entry = WorkbookHistoryEntry {
            timestamp: chrono::Utc::now(),
            operation_type: "write_cells".to_string(),
            target_path: path.to_string(),
            old_hash: old_hash.clone(),
            new_hash: new_hash.clone(),
            result: "success".to_string(),
        };
        let _ = append_history_entry(path, &entry);
    }

    Ok(WriteResult {
        success: true,
        message: String::new(),
        backup_info,
        old_hash,
        new_hash,
        diff: None,
    })
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
