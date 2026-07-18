//! Slicer feature implementation.
//!
//! Creates interactive visual filter controls for pivot table fields.
//! Since rust_xlsxwriter does not natively support slicers, this
//! implementation performs XML post-processing on the xlsx ZIP archive
//! to inject slicer XML parts conforming to the OOXML specification.
//!
//! The post-processing injects:
//! - `xl/slicers/slicer1.xml` -- Slicer definition (drawing object)
//! - `xl/slicerCaches/slicerCache1.xml` -- Slicer cache (filter item data)
//! - `xl/drawings/drawing1.xml` -- Drawing anchors for the slicer on the sheet
//! - Updates to relationships and content types

use std::collections::HashSet;
use std::io::{Cursor, Read, Write};

use zip::result::ZipError;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use crate::excel_read;
use crate::security;
use crate::types::*;

// ── OOXML namespace constants ──

const NS_A: &str = "http://schemas.openxmlformats.org/drawingml/2006/main";
const NS_R: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
const NS_MC: &str = "http://schemas.openxmlformats.org/markup-compatibility/2006";
const NS_XDR: &str = "http://schemas.openxmlformats.org/drawingml/2006/spreadsheetDrawing";
const NS_X14: &str = "http://schemas.microsoft.com/office/spreadsheetml/2009/9/main";
const NS_X14AC: &str = "http://schemas.microsoft.com/office/spreadsheetml/2009/9/ac";

const SLICER_URI: &str = "{2F2913AC-816F-4CA1-A5E6-96EA2F5C09D6}";
const SLICER_CACHE_URI: &str = "{AE3F784C-AC03-4FA6-A0BB-8CC533AA7419}";

const CONTENT_TYPE_SLICER: &str = "application/vnd.ms-excel.slicer+xml";
const CONTENT_TYPE_SLICER_CACHE: &str = "application/vnd.ms-excel.slicerCache+xml";
const CONTENT_TYPE_DRAWING: &str = "application/vnd.openxmlformats-officedocument.drawing+xml";

// ── Public API ──

/// Create a slicer for a pivot table field.
///
/// Reads the pivot's source data, extracts unique values for the given field,
/// then post-processes the xlsx ZIP archive to inject slicer XML parts.
pub fn create_slicer(
    path: &str,
    config: &SlicerConfig,
    params: &SecurityParams,
) -> Result<WriteResult> {
    if params.dry_run {
        return Ok(WriteResult::dry_run_success());
    }

    let backup_info = security::create_backup_if_needed(params)
        .map_err(|e| AppError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?;

    let old_hash = security::compute_file_hash(path)
        .map_err(|e| AppError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?;

    // Collect unique values from the pivot source data for the slicer field
    let unique_values = collect_slicer_values(path, config)?;

    if unique_values.is_empty() {
        return Err(AppError::InvalidInput(
            "No unique values found for slicer field".to_string(),
        ));
    }

    // Determine target sheet index
    let (target_sheet, _target_range) = parse_source_range(&config.source_range)?;

    // Inject slicer XML parts into the xlsx ZIP
    inject_slicer_xml(path, config, &unique_values, &target_sheet)?;

    let new_hash = security::compute_file_hash(path)
        .map_err(|e| AppError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?;

    Ok(WriteResult {
        success: true,
        message: format!(
            "Created slicer '{}' on sheet '{}' with {} items",
            config.name,
            config.target_sheet,
            unique_values.len()
        ),
        backup_info,
        old_hash,
        new_hash,
        diff: None,
    })
}

// ── Data collection ──

/// Read the pivot source range and collect unique string values for the slicer field.
fn collect_slicer_values(path: &str, config: &SlicerConfig) -> Result<Vec<String>> {
    let (source_sheet, source_range) = parse_source_range(&config.source_range)?;

    let source_data = excel_read::read_range(path, &source_sheet, &source_range)?;

    if source_data.is_empty() || source_data.len() < 2 {
        return Err(AppError::InvalidInput(
            "Source data must contain at least a header row and one data row".to_string(),
        ));
    }

    let field_col = config.field_column as usize;

    // Skip header row, collect unique values
    let mut seen = HashSet::new();
    let mut values: Vec<String> = Vec::new();

    for row in source_data.iter().skip(1) {
        if let Some(cell) = row.get(field_col) {
            if let Some(ref val) = cell.value {
                let trimmed = val.trim().to_string();
                if !trimmed.is_empty() && seen.insert(trimmed.clone()) {
                    values.push(trimmed);
                }
            }
        }
    }

    // Sort for consistent ordering
    values.sort();
    Ok(values)
}

// ── ZIP post-processing ──

/// Read the xlsx ZIP, inject slicer XML parts, and write back.
fn inject_slicer_xml(
    path: &str,
    config: &SlicerConfig,
    values: &[String],
    target_sheet: &str,
) -> Result<()> {
    // Read the existing xlsx file
    let file_bytes = std::fs::read(path).map_err(|e| {
        AppError::Io(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to read xlsx for slicer injection: {e}"),
        ))
    })?;

    let cursor = Cursor::new(file_bytes);
    let mut archive = ZipArchive::new(cursor).map_err(|e| {
        AppError::Io(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to open xlsx as ZIP: {e}"),
        ))
    })?;

    // Find the sheet index for the target sheet
    let sheet_index = find_sheet_index(&mut archive, target_sheet)?;

    // Read all existing entries
    let mut entries: Vec<(String, Vec<u8>)> = Vec::new();
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(zip_error_to_app)?;
        let name = entry.name().to_string();
        let mut data = Vec::new();
        entry.read_to_end(&mut data).map_err(io_error_to_app)?;
        entries.push((name, data));
    }

    // Generate new XML parts
    let slicer_cache_xml = build_slicer_cache_xml(config, values);
    let slicer_xml = build_slicer_xml(config, values.len() as u32);
    let drawing_xml = build_drawing_xml(config);

    // Build modified entries
    let mut new_entries: Vec<(String, Vec<u8>)> = Vec::new();

    for (name, data) in &entries {
        match name.as_str() {
            // Update [Content_Types].xml to register new part types
            "[Content_Types].xml" => {
                let updated = update_content_types(data, sheet_index);
                new_entries.push((name.clone(), updated));
            }
            // Update workbook relationships to reference slicer cache
            "xl/workbook.xml.rels" => {
                let updated = update_workbook_rels(data);
                new_entries.push((name.clone(), updated));
            }
            // Update worksheet relationships to reference the drawing
            s if s.starts_with("xl/worksheets/_rels/")
                && s.contains(&format!("sheet{sheet_index}.xml.rels")) =>
            {
                let updated = update_sheet_rels(data);
                new_entries.push((name.clone(), updated));
            }
            // Keep all other entries as-is
            _ => {
                new_entries.push((name.clone(), data.clone()));
            }
        }
    }

    // Add new XML parts
    new_entries.push((
        "xl/slicerCaches/slicerCache1.xml".to_string(),
        slicer_cache_xml,
    ));
    new_entries.push(("xl/slicers/slicer1.xml".to_string(), slicer_xml));
    new_entries.push(("xl/drawings/drawing1.xml".to_string(), drawing_xml));
    new_entries.push((
        "xl/drawings/_rels/drawing1.xml.rels".to_string(),
        build_drawing_rels().into_bytes(),
    ));

    // Rebuild the ZIP with injected parts
    let output = rebuild_zip(&new_entries)?;
    std::fs::write(path, output).map_err(|e| {
        AppError::Io(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to write modified xlsx: {e}"),
        ))
    })?;

    Ok(())
}

// ── Sheet index detection ──

fn find_sheet_index<R: Read + std::io::Seek>(
    archive: &mut ZipArchive<R>,
    target_sheet: &str,
) -> Result<usize> {
    // Read workbook.xml to map sheet names to indices
    let mut wb_entry = archive
        .by_name("xl/workbook.xml")
        .map_err(|_| AppError::InvalidInput("xl/workbook.xml not found in xlsx".to_string()))?;
    let mut wb_xml = String::new();
    wb_entry
        .read_to_string(&mut wb_xml)
        .map_err(io_error_to_app)?;

    // Find the sheet with the matching name
    // Sheets are 1-indexed in the sheetId, 0-based for rId references
    let mut sheet_idx = 0usize; // 0-based index for rId (rId1 -> index 0)
    for line in wb_xml.lines() {
        // Look for: <sheet name="SheetName" sheetId="N" r:id="rIdM"/>
        if line.contains(&format!("name=\"{target_sheet}\"")) {
            // Extract the rId to determine index
            if let Some(r_id_pos) = line.find("r:id=\"rId") {
                let rest = &line[r_id_pos + 8..]; // skip 'r:id="rId'
                if let Some(end) = rest.find('"') {
                    if let Ok(num) = rest[..end].parse::<usize>() {
                        sheet_idx = num - 1; // rId1 -> index 0
                    }
                }
            }
            break;
        }
    }

    Ok(sheet_idx + 1) // Return 1-based sheet index
}

fn rebuild_zip(entries: &[(String, Vec<u8>)]) -> Result<Vec<u8>> {
    let buf = Cursor::new(Vec::new());
    let mut writer = ZipWriter::new(buf);
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);

    for (name, data) in entries {
        writer.start_file(name, options).map_err(zip_error_to_app)?;
        writer.write_all(data).map_err(io_error_to_app)?;
    }

    let finished = writer.finish().map_err(zip_error_to_app)?;
    Ok(finished.into_inner())
}

// ── XML builders ──

fn build_slicer_cache_xml(config: &SlicerConfig, values: &[String]) -> Vec<u8> {
    let mut items_xml = String::new();
    for val in values {
        let escaped = xml_escape(val);
        items_xml.push_str(&format!("   <x14:slicerCacheItem s=\"{escaped}\"/>\n"));
    }

    let style = config.style.as_deref().unwrap_or("SlicerStyleLight1");
    let column_count = config.columns;

    let xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<extLst xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
        xmlns:mc="{NS_MC}"
        xmlns:x14="{NS_X14}"
        mc:Ignorable="x14">
  <ext uri="{SLICER_CACHE_URI}">
    <x14:slicerCacheDefinition name="{name}" sourceName="{name}">
      <x14:pivotTables>
        <x14:pivotTable tabId="0" name="PivotTable::{source_sheet}::{pt_name}"/>
      </x14:pivotTables>
      <x14:data>
        <x14:dataField/>
      </x14:data>
      <x14:slicerCacheItems>
        <x14:slicerCacheItem n="(All)" nd="true"/>
{items}
      </x14:slicerCacheItems>
      <x14:extLst>
        <x14:ext uri="{NS_X14AC}">
          <x14ac:tabId val="0" xmlns:x14ac="{NS_X14AC}"/>
          <x14ac:columnCount val="{column_count}" xmlns:x14ac="{NS_X14AC}"/>
          <x14ac:style xmlns:x14ac="{NS_X14AC}" xr9:uid="{{00000000-0008-0000-0000-000000000000}}" name="{style}" xmlns:xr9="http://schemas.microsoft.com/office/spreadsheetml/2016/revision9"/>
        </x14:ext>
      </x14:extLst>
    </x14:slicerCacheDefinition>
  </ext>
</extLst>"#,
        name = xml_escape(&config.name),
        source_sheet = "",
        pt_name = xml_escape(&config.pivot_table_name),
        items = items_xml,
        style = xml_escape(style),
        column_count = column_count,
    );

    xml.into_bytes()
}

fn build_slicer_xml(config: &SlicerConfig, item_count: u32) -> Vec<u8> {
    let style = config.style.as_deref().unwrap_or("SlicerStyleLight1");

    let xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<extLst xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
        xmlns:mc="{NS_MC}"
        xmlns:x14="{NS_X14}"
        mc:Ignorable="x14">
  <ext uri="{SLICER_URI}">
    <x14:slicer name="{name}" cache="SlicerCache1" caption="{name}"
                rowCount="{item_count}" showCaption="1"
                columnCount="{columns}" lockedPosition="1">
      <x14:extLst>
        <x14:ext uri="{NS_X14AC}">
          <x14ac:style xmlns:x14ac="{NS_X14AC}" xr9:uid="{{00000000-0008-0000-0000-000000000000}}" name="{style}" xmlns:xr9="http://schemas.microsoft.com/office/spreadsheetml/2016/revision9"/>
        </x14:ext>
      </x14:extLst>
    </x14:slicer>
  </ext>
</extLst>"#,
        name = xml_escape(&config.name),
        item_count = item_count + 1, // +1 for "(All)" entry
        columns = config.columns,
        style = xml_escape(style),
    );

    xml.into_bytes()
}

fn build_drawing_xml(config: &SlicerConfig) -> Vec<u8> {
    let col_from = config.position.col / 64; // approximate column from pixels
    let row_from = config.position.row / 20; // approximate row from pixels
    let col_to = col_from + 6; // slicer width ~= 6 columns
    let row_to = row_from + 14; // slicer height ~= 14 rows

    let (emu_col, emu_row, emu_width, emu_height) = config.position.to_emu();

    let xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<xdr:wsDr xmlns:xdr="{NS_XDR}"
          xmlns:a="{NS_A}"
          xmlns:r="{NS_R}"
          xmlns:mc="{NS_MC}"
          mc:Ignorable="x14">
  <xdr:twoCellAnchor editAs="twoCell">
    <xdr:from>
      <xdr:col>{col_from}</xdr:col>
      <xdr:colOff>{emu_col}</xdr:colOff>
      <xdr:row>{row_from}</xdr:row>
      <xdr:rowOff>{emu_row}</xdr:rowOff>
    </xdr:from>
    <xdr:to>
      <xdr:col>{col_to}</xdr:col>
      <xdr:colOff>0</xdr:colOff>
      <xdr:row>{row_to}</xdr:row>
      <xdr:rowOff>0</xdr:rowOff>
    </xdr:to>
    <xdr:graphicFrame macro="" fPublished="0">
      <xdr:nvGraphicFramePr>
        <xdr:cNvPr id="0" name="Slicer_{name}"/>
        <xdr:cNvGraphicFramePr/>
      </xdr:nvGraphicFramePr>
      <xdr:xfrm>
        <a:off x="0" y="0"/>
        <a:ext cx="{emu_width}" cy="{emu_height}"/>
      </xdr:xfrm>
      <a:graphic>
        <a:graphicData uri="http://schemas.microsoft.com/office/spreadsheetml/2010/slicer">
          <xdr:spTgt link="rId1"/>
        </a:graphicData>
      </a:graphic>
    </xdr:graphicFrame>
    <xdr:clientData/>
  </xdr:twoCellAnchor>
</xdr:wsDr>"#,
        name = xml_escape(&config.name),
        col_from = col_from,
        emu_col = emu_col % 9525, // colOff is within one column
        row_from = row_from,
        emu_row = emu_row % 9525,
        col_to = col_to,
        row_to = row_to,
        emu_width = emu_width,
        emu_height = emu_height,
    );

    xml.into_bytes()
}

fn build_drawing_rels() -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="{NS_R}">
  <Relationship Id="rId1"
    Type="http://schemas.microsoft.com/office/2010/relationships/slicer"
    Target="../slicers/slicer1.xml"/>
</Relationships>"#
    )
}

// ── XML update helpers ──

fn update_content_types(original: &[u8], _sheet_index: usize) -> Vec<u8> {
    let xml_str = String::from_utf8_lossy(original);

    // Only add if not already present
    if xml_str.contains(CONTENT_TYPE_SLICER) {
        return original.to_vec();
    }

    let new_parts = format!(
        r#"<Override PartName="/xl/slicerCaches/slicerCache1.xml" ContentType="{CONTENT_TYPE_SLICER_CACHE}"/>
  <Override PartName="/xl/slicers/slicer1.xml" ContentType="{CONTENT_TYPE_SLICER}"/>
  <Override PartName="/xl/drawings/drawing1.xml" ContentType="{CONTENT_TYPE_DRAWING}"/>"#
    );

    // Insert before </Types>
    let updated = xml_str.replace("</Types>", &format!("{new_parts}\n</Types>"));
    updated.into_bytes()
}

fn update_workbook_rels(original: &[u8]) -> Vec<u8> {
    let xml_str = String::from_utf8_lossy(original);

    if xml_str.contains("slicerCache1") {
        return original.to_vec();
    }

    // Find the highest existing rId and increment
    let mut max_rid = 0;
    for line in xml_str.lines() {
        if let Some(pos) = line.find("Id=\"rId") {
            let rest = &line[pos + 7..];
            if let Some(end) = rest.find('"') {
                if let Ok(n) = rest[..end].parse::<usize>() {
                    max_rid = max_rid.max(n);
                }
            }
        }
    }

    let new_r_id = max_rid + 1;
    let new_rel = format!(
        r#"<Relationship Id="rId{new_r_id}"
    Type="http://schemas.microsoft.com/office/2007/relationships/slicerCache"
    Target="slicerCaches/slicerCache1.xml"/>"#
    );

    let updated = xml_str.replace("</Relationships>", &format!("{new_rel}\n</Relationships>"));
    updated.into_bytes()
}

fn update_sheet_rels(original: &[u8]) -> Vec<u8> {
    let xml_str = String::from_utf8_lossy(original);

    if xml_str.contains("drawing1.xml") {
        return original.to_vec();
    }

    let new_rel = r#"<Relationship Id="rIdDraw"
    Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/drawing"
    Target="../drawings/drawing1.xml"/>"#;

    let updated = xml_str.replace("</Relationships>", &format!("{new_rel}\n</Relationships>"));
    updated.into_bytes()
}

// ── Utilities ──

fn parse_source_range(range_spec: &str) -> Result<(String, String)> {
    if let Some(excl) = range_spec.find('!') {
        let sheet = range_spec[..excl].to_string();
        // Remove single quotes around sheet name if present
        let sheet = sheet.trim_matches('\'');
        let range = range_spec[excl + 1..].to_string();
        Ok((sheet.to_string(), range))
    } else {
        Err(AppError::InvalidInput(format!(
            "Invalid source range '{range_spec}': expected 'Sheet!Range' format"
        )))
    }
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn zip_error_to_app(e: ZipError) -> AppError {
    AppError::Io(std::io::Error::new(
        std::io::ErrorKind::Other,
        e.to_string(),
    ))
}

fn io_error_to_app(e: std::io::Error) -> AppError {
    AppError::Io(e)
}
