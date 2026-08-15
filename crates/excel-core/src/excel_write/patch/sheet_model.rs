use std::collections::BTreeMap;
use std::io::Cursor;
use quick_xml::events::Event;
use quick_xml::escape::escape;
use quick_xml::Reader;
use crate::types::{
    CellData, CellDataType,
    Result,
};
use crate::utils::cell_ref::{index_to_col, parse_cell_ref};


pub(crate) fn sheetdata_spans(xml: &[u8]) -> Option<(&[u8], &[u8], &[u8], bool)> {
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
pub(crate) struct SheetModel {
    pub(crate) rows: BTreeMap<u32, Row>,
}


pub(crate) struct Row {
    /// The raw start tag, e.g. `<row r="1" spans="1:26">` (normalized to an open tag ending with `>`)`
    pub(crate) open_tag: String,
    pub(crate) cells: BTreeMap<u16, Cell>,
}


pub(crate) struct Cell {
    /// Byte-for-byte original of `<c ...>...</c>` or `<c .../>` (replaced as a whole when editing)
    pub(crate) raw: Vec<u8>,
}


pub(crate) fn event_markup(event: &Event) -> Vec<u8> {
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


pub(crate) fn parse_sheetdata(inner: &[u8]) -> SheetModel {
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


pub(crate) fn apply_edits(mut model: SheetModel, edits: &[(u32, u16, CellData)]) -> SheetModel {
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


pub(crate) fn serialize_sheet(model: &SheetModel) -> String {
    let mut out = String::new();
    for row in model.rows.values() {
        out.push_str(&row.open_tag);
        for cell in row.cells.values() {
            out.push_str(&String::from_utf8_lossy(&cell.raw));
        }
        out.push_str("</row>");
    }
    out
}


pub(crate) fn dimension_ref(model: &SheetModel) -> String {
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


pub(crate) fn replace_dimension(before: &mut Vec<u8>, refstr: &str) {
    if let Some(start) = before.windows(10).position(|w| &w[..10] == b"<dimension")
        && let Some(rel) = before[start..].windows(5).position(|w| &w[..5] == b"ref=\"") {
            let abs = start + rel + 5;
            if let Some(end) = before[abs..].iter().position(|&b| b == b'"') {
                let end = abs + end;
                let new = refstr.as_bytes();
                before.splice(abs..end, new.iter().cloned());
                return;
            }
        }
    // No dimension element: insert before <sheetData (follows schema order).
    if let Some(pos) = before.windows(10).position(|w| &w[..10] == b"<sheetData") {
        let ins = format!("<dimension ref=\"{}\"/>", refstr);
        before.splice(pos..pos, ins.into_bytes());
    }
}


pub(crate) fn patch_sheet_xml(xml: &[u8], edits: &[(u32, u16, CellData)]) -> Result<Vec<u8>> {
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


pub(crate) fn cell_xml(cd: &CellData) -> (Option<&'static str>, String) {
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


pub(crate) fn rebuild_cell(rref: &str, style: Option<&str>, cd: &CellData) -> String {
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


pub(crate) fn extract_attr(raw: &[u8], key: &[u8]) -> Option<String> {
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


pub(crate) fn extract_col(raw: &[u8]) -> u16 {
    extract_attr(raw, b"r")
        .and_then(|r| parse_cell_ref(&r).ok())
        .map(|(_, col)| col)
        .unwrap_or(0)
}

// ───────────────────────────────────────────────────────────────────────────
// Phase 3 — generic fallback: preserve_all_parts_transfer
// ───────────────────────────────────────────────────────────────────────────

