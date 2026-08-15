use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{Cursor, Read, Write};
use std::path::Path;
use quick_xml::events::Event;
use quick_xml::Reader;
use zip::{ZipArchive, ZipWriter};
use crate::security::append_history_entry;
use crate::types::{
    AppError,
    Result, WorkbookHistoryEntry,
};

use super::*;

pub(crate) fn repackage_zip(
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


pub(crate) fn repackage_zip_multi(
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
        if skip_parts.contains(&name) {
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


pub(crate) fn append_history(path: &str, op: &str, old_hash: &str, new_hash: &str, dry_run: bool) {
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


pub(crate) fn has_formula(raw: &[u8]) -> bool {
    raw.windows(3).any(|w| w == b"<f>") || raw.windows(4).any(|w| w == b"<f ")
}


pub(crate) fn strip_v_element(raw: &[u8]) -> Vec<u8> {
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


pub(crate) fn read_zip_entry(archive: &mut ZipArchive<File>, name: &str) -> Result<Vec<u8>> {
    let mut zf = archive
        .by_name(name)
        .map_err(|e| AppError::Custom(format!("missing zip entry {}: {}", name, e)))?;
    let mut buf = Vec::new();
    zf.read_to_end(&mut buf).map_err(AppError::Io)?;
    Ok(buf)
}


pub(crate) fn resolve_sheet_part(archive: &mut ZipArchive<File>, sheet: &str) -> Result<String> {
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


pub(crate) fn find_sheet_rid(wb: &[u8], sheet: &str) -> Option<String> {
    let mut reader = Reader::from_reader(Cursor::new(wb));
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                // Note: quick-xml events expose only the inner tag content via Deref
                // (e.g. `sheet name=...`), without `<`, so match the prefix `sheet` not `<sheet`.
                let raw: &[u8] = &e;
                if raw.starts_with(b"sheet")
                    && extract_attr(raw, b"name").as_deref() == Some(sheet) {
                        return extract_attr(raw, b"r:id");
                    }
            }
            Ok(Event::Eof) => break,
            Ok(_) => {}
            Err(_) => break,
        }
    }
    None
}


pub(crate) fn find_rel_target(rels: &[u8], rid: &str) -> Option<String> {
    let mut reader = Reader::from_reader(Cursor::new(rels));
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                // Same as above: deref content is `Relationship Id=...`, without `<`.
                let raw: &[u8] = &e;
                if raw.starts_with(b"Relationship")
                    && extract_attr(raw, b"Id").as_deref() == Some(rid) {
                        return extract_attr(raw, b"Target");
                    }
            }
            Ok(Event::Eof) => break,
            Ok(_) => {}
            Err(_) => break,
        }
    }
    None
}


pub(crate) fn find_rel_by_type(rels: &[u8], type_suffix: &str) -> Option<String> {
    let mut reader = Reader::from_reader(Cursor::new(rels));
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                let raw: &[u8] = &e;
                if raw.starts_with(b"Relationship")
                    && let Some(ty) = extract_attr(raw, b"Type")
                        && ty.ends_with(type_suffix) {
                            return extract_attr(raw, b"Id");
                        }
            }
            Ok(Event::Eof) => break,
            Ok(_) => {}
            Err(_) => break,
        }
    }
    None
}


pub(crate) fn find_all_rel_targets_by_type(rels: &[u8], type_suffix: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut reader = Reader::from_reader(Cursor::new(rels));
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                let raw: &[u8] = &e;
                if raw.starts_with(b"Relationship")
                    && let (Some(ty), Some(tgt)) =
                        (extract_attr(raw, b"Type"), extract_attr(raw, b"Target"))
                        && ty.ends_with(type_suffix) {
                            out.push(tgt);
                        }
            }
            Ok(Event::Eof) => break,
            Ok(_) => {}
            Err(_) => break,
        }
    }
    out
}


pub(crate) fn normalize_rel_target(base_part: &str, target: &str) -> String {
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


pub(crate) fn remove_rel_by_id(rels: &[u8], rid: &str) -> String {
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


pub(crate) fn remove_drawing_elem(sheet_xml: &[u8], rid: &str) -> String {
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


pub(crate) const NON_DATA_PREFIXES: &[&str] = &[
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


pub(crate) fn is_non_data_part(name: &str) -> bool {
    NON_DATA_PREFIXES.iter().any(|p| name.starts_with(p))
}


pub(crate) fn read_zip_map(path: &str) -> Result<HashMap<String, Vec<u8>>> {
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


pub(crate) fn write_zip_map(path: &str, entries: &HashMap<String, Vec<u8>>) -> Result<()> {
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


pub(crate) fn parse_sheet_name_to_part(wb_xml: &[u8], rels_xml: &[u8]) -> HashMap<String, String> {
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
            if let Ok(target) = find_rel_target_str(rels_xml, &rid) {
                result.insert(name, target);
            }
        }
        pos += sheet_start + 1;
    }
    result
}


pub(crate) fn extract_attr_str(s: &str, key: &str) -> Option<String> {
    let marker = format!("{}=\"", key);
    if let Some(start) = s.find(&marker) {
        let val_start = start + marker.len();
        if let Some(end) = s[val_start..].find('"') {
            return Some(s[val_start..val_start + end].to_string());
        }
    }
    None
}


pub(crate) fn extract_non_data_elements(xml: &[u8]) -> Vec<(String, String)> {
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
                .split([' ', ':'])
                .next_back()
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


pub(crate) fn capture_element(body: &str, start: usize) -> (String, usize) {
    let bytes = body.as_bytes();
    let n = bytes.len();
    // Tag name (local name)
    let mut j = start + 1;
    while j < n && bytes[j] != b' ' && bytes[j] != b'>' && bytes[j] != b'/' {
        j += 1;
    }
    let tag = &body[start + 1..j];
    let local = tag
        .split([' ', ':'])
        .next_back()
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
                    .split([' ', ':'])
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


pub(crate) fn skip_element(body: &str, start: usize) -> usize {
    let (_, next) = capture_element(body, start);
    next
}


pub(crate) fn has_element(xml: &str, tag_name: &str) -> bool {
    // Look for an open tag (excluding </)
    let open = format!("<{} ", tag_name);
    let open2 = format!("<{}>", tag_name);
    let open3 = format!("<{}/>", tag_name);
    xml.contains(&open) || xml.contains(&open2) || xml.contains(&open3)
}


pub(crate) fn merge_worksheet_xml(source_xml: &[u8], rebuilt_xml: &[u8]) -> Vec<u8> {
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
    if !added_content_types.is_empty()
        && let Some(ct_content) = output.get("[Content_Types].xml") {
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

    // 7. Write the output zip
    write_zip_map(rebuilt_path, &output)?;

    Ok(())
}


pub(crate) fn guess_content_type(part_name: &str) -> &'static str {
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

