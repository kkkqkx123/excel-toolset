use crate::types::{CellData, CellDataType};
use std::fs::File;
use zip::{ZipArchive, ZipWriter};

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
    assert!(
        s.contains("r=\"B1\"") && s.contains("<v>2</v>"),
        "B1 lost: {}",
        s
    );
    assert!(
        s.contains("r=\"A2\"") && s.contains("<v>3</v>"),
        "A2 lost: {}",
        s
    );
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
        s.contains("r=\"C3\"")
            && s.contains("s=\"7\"")
            && s.contains("t=\"inlineStr\"")
            && s.contains("<t>hi</t>"),
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
    use crate::types::{CellData, CellDataType, SecurityParams};
    use std::collections::HashMap;

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
        assert_eq!(
            a, content,
            "non-target part content changed (lost source feature): {}",
            name
        );
    }

    // 2) All rich-feature markers survive (styles / comments / drawing / chart).
    let styles = String::from_utf8_lossy(after.get("xl/styles.xml").unwrap());
    assert!(
        styles.contains("DISTINCT_STYLE_9999"),
        "styles.xml not preserved"
    );
    let comments = String::from_utf8_lossy(after.get("xl/comments1.xml").unwrap());
    assert!(
        comments.contains("DISTINCT_COMMENT_A1"),
        "comments1.xml not preserved"
    );
    let drawing = String::from_utf8_lossy(after.get("xl/drawings/drawing1.xml").unwrap());
    assert!(
        drawing.contains("DISTINCT_DRAWING"),
        "drawing1.xml not preserved"
    );
    let chart = String::from_utf8_lossy(after.get("xl/charts/chart1.xml").unwrap());
    assert!(
        chart.contains("DISTINCT_CHART_TITLE"),
        "chart1.xml not preserved"
    );

    // 3) Target sheet: styled cells, merges, data validation and frozen panes are all
    // preserved, and the new cell has been written.
    let sheet = String::from_utf8_lossy(after.get(target).unwrap());
    assert!(
        sheet.contains("s=\"3\"") && sheet.contains("t=\"s\"") && sheet.contains("<v>0</v>"),
        "A1 style/shared-string reference lost: {}",
        sheet
    );
    assert!(
        sheet.contains("mergeCells") && sheet.contains("C1:E1"),
        "merged cells lost"
    );
    assert!(
        sheet.contains("dataValidation") && sheet.contains("F1:F10"),
        "data validation lost"
    );
    assert!(sheet.contains("state=\"frozen\""), "frozen panes lost");
    assert!(
        sheet.contains("r=\"Z10\"")
            && sheet.contains("t=\"inlineStr\"")
            && sheet.contains("<t>x</t>"),
        "Z10 new cell not written: {}",
        sheet
    );
    // Old values 100 / 200 are still present (not cleared).
    assert!(
        sheet.contains("<v>100</v>") && sheet.contains("<v>200</v>"),
        "original values lost"
    );
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
    use crate::types::{CellData, CellDataType, SecurityParams};
    use std::collections::HashMap;

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
        sheet.contains("r=\"Z10\"")
            && sheet.contains("t=\"inlineStr\"")
            && sheet.contains("<t>x</t>"),
        "Z10 not written with absolute-path Target: {}",
        sheet
    );
    // Rich features and styled cells are all preserved.
    assert!(
        sheet.contains("s=\"3\"") && sheet.contains("state=\"frozen\""),
        "styles/frozen panes lost with absolute-path Target: {}",
        sheet
    );
    assert!(
        String::from_utf8_lossy(after.get("xl/styles.xml").unwrap())
            .contains("DISTINCT_STYLE_9999"),
        "styles.xml not preserved"
    );
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
    let start = sheet
        .find("<dimension")
        .expect("dimension element expected");
    let end = start
        + sheet[start..]
            .find("/>")
            .expect("dimension should be self-closed");
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
