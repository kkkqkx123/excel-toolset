use rust_xlsxwriter::{
    Formula, Table, TableColumn, TableFunction as XlsxTableFunc,
    TableStyle as XlsxTableStyle,
};

use crate::excel_read;
use crate::excel_write;
use crate::security;
use crate::types::*;

/// Map our TableStylePreset to rust_xlsxwriter TableStyle.
fn map_table_style(preset: &TableStylePreset) -> XlsxTableStyle {
    match preset {
        TableStylePreset::None => XlsxTableStyle::None,
        TableStylePreset::Light1 => XlsxTableStyle::Light1,
        TableStylePreset::Light2 => XlsxTableStyle::Light2,
        TableStylePreset::Light3 => XlsxTableStyle::Light3,
        TableStylePreset::Light4 => XlsxTableStyle::Light4,
        TableStylePreset::Light5 => XlsxTableStyle::Light5,
        TableStylePreset::Light6 => XlsxTableStyle::Light6,
        TableStylePreset::Light7 => XlsxTableStyle::Light7,
        TableStylePreset::Light8 => XlsxTableStyle::Light8,
        TableStylePreset::Light9 => XlsxTableStyle::Light9,
        TableStylePreset::Light10 => XlsxTableStyle::Light10,
        TableStylePreset::Light11 => XlsxTableStyle::Light11,
        TableStylePreset::Light12 => XlsxTableStyle::Light12,
        TableStylePreset::Light13 => XlsxTableStyle::Light13,
        TableStylePreset::Light14 => XlsxTableStyle::Light14,
        TableStylePreset::Light15 => XlsxTableStyle::Light15,
        TableStylePreset::Light16 => XlsxTableStyle::Light16,
        TableStylePreset::Light17 => XlsxTableStyle::Light17,
        TableStylePreset::Light18 => XlsxTableStyle::Light18,
        TableStylePreset::Light19 => XlsxTableStyle::Light19,
        TableStylePreset::Light20 => XlsxTableStyle::Light20,
        TableStylePreset::Light21 => XlsxTableStyle::Light21,
        TableStylePreset::Medium1 => XlsxTableStyle::Medium1,
        TableStylePreset::Medium2 => XlsxTableStyle::Medium2,
        TableStylePreset::Medium3 => XlsxTableStyle::Medium3,
        TableStylePreset::Medium4 => XlsxTableStyle::Medium4,
        TableStylePreset::Medium5 => XlsxTableStyle::Medium5,
        TableStylePreset::Medium6 => XlsxTableStyle::Medium6,
        TableStylePreset::Medium7 => XlsxTableStyle::Medium7,
        TableStylePreset::Medium8 => XlsxTableStyle::Medium8,
        TableStylePreset::Medium9 => XlsxTableStyle::Medium9,
        TableStylePreset::Medium10 => XlsxTableStyle::Medium10,
        TableStylePreset::Medium11 => XlsxTableStyle::Medium11,
        TableStylePreset::Medium12 => XlsxTableStyle::Medium12,
        TableStylePreset::Medium13 => XlsxTableStyle::Medium13,
        TableStylePreset::Medium14 => XlsxTableStyle::Medium14,
        TableStylePreset::Medium15 => XlsxTableStyle::Medium15,
        TableStylePreset::Medium16 => XlsxTableStyle::Medium16,
        TableStylePreset::Medium17 => XlsxTableStyle::Medium17,
        TableStylePreset::Medium18 => XlsxTableStyle::Medium18,
        TableStylePreset::Medium19 => XlsxTableStyle::Medium19,
        TableStylePreset::Medium20 => XlsxTableStyle::Medium20,
        TableStylePreset::Medium21 => XlsxTableStyle::Medium21,
        TableStylePreset::Medium22 => XlsxTableStyle::Medium22,
        TableStylePreset::Medium23 => XlsxTableStyle::Medium23,
        TableStylePreset::Medium24 => XlsxTableStyle::Medium24,
        TableStylePreset::Medium25 => XlsxTableStyle::Medium25,
        TableStylePreset::Medium26 => XlsxTableStyle::Medium26,
        TableStylePreset::Medium27 => XlsxTableStyle::Medium27,
        TableStylePreset::Medium28 => XlsxTableStyle::Medium28,
        TableStylePreset::Dark1 => XlsxTableStyle::Dark1,
        TableStylePreset::Dark2 => XlsxTableStyle::Dark2,
        TableStylePreset::Dark3 => XlsxTableStyle::Dark3,
        TableStylePreset::Dark4 => XlsxTableStyle::Dark4,
        TableStylePreset::Dark5 => XlsxTableStyle::Dark5,
        TableStylePreset::Dark6 => XlsxTableStyle::Dark6,
        TableStylePreset::Dark7 => XlsxTableStyle::Dark7,
        TableStylePreset::Dark8 => XlsxTableStyle::Dark8,
        TableStylePreset::Dark9 => XlsxTableStyle::Dark9,
        TableStylePreset::Dark10 => XlsxTableStyle::Dark10,
        TableStylePreset::Dark11 => XlsxTableStyle::Dark11,
    }
}

/// Map our TotalFunction to rust_xlsxwriter TableFunction.
fn map_total_function(func: &TotalFunction) -> XlsxTableFunc {
    match func {
        TotalFunction::Sum => XlsxTableFunc::Sum,
        TotalFunction::Average => XlsxTableFunc::Average,
        TotalFunction::Count => XlsxTableFunc::Count,
        TotalFunction::CountNums => XlsxTableFunc::CountNumbers,
        TotalFunction::Max => XlsxTableFunc::Max,
        TotalFunction::Min => XlsxTableFunc::Min,
        TotalFunction::StdDev => XlsxTableFunc::StdDev,
        TotalFunction::Var => XlsxTableFunc::Var,
        TotalFunction::Custom(label) => XlsxTableFunc::Custom(Formula::new(label.as_str())),
    }
}

/// Normalize a range so that r1 <= r2 and c1 <= c2.
fn normalize_range(r1: u32, c1: u16, r2: u32, c2: u16) -> (u32, u16, u32, u16) {
    (
        r1.min(r2),
        c1.min(c2),
        r1.max(r2),
        c1.max(c2),
    )
}

/// Check if two ranges overlap.
fn ranges_overlap(
    a_r1: u32,
    a_c1: u16,
    a_r2: u32,
    a_c2: u16,
    b_r1: u32,
    b_c1: u16,
    b_r2: u32,
    b_c2: u16,
) -> bool {
    !(a_r2 < b_r1 || b_r2 < a_r1 || a_c2 < b_c1 || b_c2 < a_c1)
}

/// Convert 0-based row/col to Excel cell reference (e.g. "A1").
fn cell_ref_to_string(row: u32, col: u16) -> String {
    let mut col_str = String::new();
    let mut c = col;
    loop {
        let rem = (c % 26) as u8;
        col_str.insert(0, (b'A' + rem) as char);
        if c < 26 {
            break;
        }
        c = c / 26 - 1;
    }
    format!("{}{}", col_str, row + 1)
}

/// Convert 0-based row/col range to Excel range string (e.g. "A1:D10").
fn range_to_string(r1: u32, c1: u16, r2: u32, c2: u16) -> String {
    format!(
        "{}:{}",
        cell_ref_to_string(r1, c1),
        cell_ref_to_string(r2, c2)
    )
}

/// Create an Excel table (ListObject) on a worksheet.
/// Handles range normalization, name uniqueness validation, and overlap detection.
pub fn create_table(
    path: &str,
    config: &TableConfig,
    params: &SecurityParams,
) -> Result<WriteResult> {
    if params.dry_run {
        return Ok(WriteResult::dry_run_success());
    }

    security::create_backup_if_needed(params)?;

    // Parse and normalize the range
    let (raw_r1, raw_c1, raw_r2, raw_c2) = crate::utils::cell_ref::parse_range(&config.range)?;
    let (r1, c1, r2, c2) = normalize_range(raw_r1, raw_c1, raw_r2, raw_c2);
    let normalized_range = range_to_string(r1, c1, r2, c2);

    // Determine target sheet
    let target_sheet = if let Some(ref sheet_name) = config.sheet {
        sheet_name.clone()
    } else if let Some(bang_pos) = config.range.find('!') {
        config.range[..bang_pos].trim_matches('\'').to_string()
    } else {
        // Read first sheet name
        let info = excel_read::read_file_info(path)?;
        info.sheets
            .first()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "Sheet1".to_string())
    };

    // Validate table name uniqueness across the workbook
    {
        let existing_tables = list_tables(path)?;
        for t in &existing_tables {
            if t.name.eq_ignore_ascii_case(&config.name) {
                return Err(AppError::InvalidInput(format!(
                    "Table name '{}' already exists in the workbook",
                    config.name
                )));
            }
        }
    }

    // Check for overlapping tables on the same sheet
    {
        let existing_tables = list_tables(path)?;
        for t in &existing_tables {
            if t.sheet.eq_ignore_ascii_case(&target_sheet) {
                if let Ok((er1, ec1, er2, ec2)) =
                    crate::utils::cell_ref::parse_range(&t.range)
                {
                    if ranges_overlap(r1, c1, r2, c2, er1, ec1, er2, ec2) {
                        return Err(AppError::InvalidInput(format!(
                            "Table range '{}' overlaps with existing table '{}' at '{}' on sheet '{}'",
                            normalized_range, t.name, t.range, target_sheet
                        )));
                    }
                }
            }
        }
    }

    excel_write::modify_file_with_wb(path, params, move |_, wb| {
        let mut table = Table::new();
        let display_name = config
            .display_name
            .clone()
            .unwrap_or_else(|| config.name.clone());
        table = table.set_name(&display_name);
        table = table.set_style(map_table_style(&config.style));

        if config.has_header {
            table = table.set_header_row(true);
        }
        if config.has_total {
            table = table.set_total_row(true);
        }

        // Set style toggles
        table = table.set_autofilter(true);
        table = table.set_first_column(config.show_first_column);
        table = table.set_last_column(config.show_last_column);
        table = table.set_banded_rows(config.show_row_stripes);
        table = table.set_banded_columns(config.show_column_stripes);

        if config.auto_expand {
            // rust_xlsxwriter auto-expands by default for tables with total row
        }

        // Set column names if provided
        if let Some(ref column_names) = config.column_names {
            let mut columns: Vec<TableColumn> = column_names
                .iter()
                .map(|name| {
                    let mut col = TableColumn::new();
                    col = col.set_header(name);
                    col
                })
                .collect();

            // Apply total row functions to corresponding columns
            if let Some(ref total_funcs) = config.total_row_functions {
                for tf in total_funcs {
                    if (tf.column as usize) < columns.len() {
                        columns[tf.column as usize] = columns[tf.column as usize]
                            .clone()
                            .set_total_function(map_total_function(&tf.function));
                        if let TotalFunction::Custom(ref label) = tf.function {
                            columns[tf.column as usize] =
                                columns[tf.column as usize]
                                    .clone()
                                    .set_total_label(label);
                        }
                    }
                }
            }
            table = table.set_columns(&columns);
        } else if let Some(ref total_funcs) = config.total_row_functions {
            // Set total row functions without column names
            let max_col = total_funcs
                .iter()
                .map(|tf| tf.column)
                .max()
                .unwrap_or(0);
            let mut columns: Vec<TableColumn> = (0..=max_col)
                .map(|_| TableColumn::new())
                .collect();
            for tf in total_funcs {
                if (tf.column as usize) < columns.len() {
                    columns[tf.column as usize] = columns[tf.column as usize]
                        .clone()
                        .set_total_function(map_total_function(&tf.function));
                    if let TotalFunction::Custom(ref label) = tf.function {
                        columns[tf.column as usize] = columns[tf.column as usize]
                            .clone()
                            .set_total_label(label);
                    }
                }
            }
            table = table.set_columns(&columns);
        }

        let worksheet = wb
            .worksheet_from_name(&target_sheet)
            .map_err(|_e| AppError::SheetNotFound(target_sheet.clone()))?;

        worksheet
            .add_table(r1, c1, r2, c2, &table)
            .map_err(AppError::Xlsx)?;

        Ok(())
    })
}

/// Remove a table from the workbook by name.
/// The table is removed by rebuilding the workbook without re-adding it.
pub fn remove_table(
    path: &str,
    table_name: &str,
    params: &SecurityParams,
) -> Result<WriteResult> {
    if params.dry_run {
        return Ok(WriteResult::dry_run_success());
    }

    security::create_backup_if_needed(params)?;

    // Verify table exists
    let existing = list_tables(path)?;
    if !existing.iter().any(|t| t.name.eq_ignore_ascii_case(table_name)) {
        return Err(AppError::InvalidInput(format!(
            "Table '{}' not found in workbook",
            table_name
        )));
    }

    // Rebuilding the workbook without re-adding the table effectively removes it.
    // The calamine read + rust_xlsxwriter rebuild will exclude the table.
    excel_write::modify_file_with_wb(path, params, |_, _| Ok(()))
}

/// List all tables in the workbook.
pub fn list_tables(path: &str) -> Result<Vec<TableInfo>> {
    // Since rust_xlsxwriter doesn't support reading tables from existing files,
    // we use calamine to read table metadata from the xlsx archive.
    use std::io::Read;
    use std::io::BufReader;

    let file = std::fs::File::open(path)
        .map_err(|e| AppError::Custom(format!("Failed to open file: {}", e)))?;
    let mut archive =
        zip::ZipArchive::new(BufReader::new(file))
            .map_err(|e| AppError::Custom(format!("Failed to read zip: {}", e)))?;

    let mut tables: Vec<TableInfo> = Vec::new();

    // Each worksheet may reference table parts via tableParts in the sheet XML
    // We scan for table XML files and extract their metadata
    for i in 0..archive.len() {
        let mut entry = match archive.by_index(i) {
            Ok(e) => e,
            Err(_) => continue,
        };
        let name = entry.name().to_string();

        // Look for table XML files: xl/tables/tableN.xml
        if name.starts_with("xl/tables/table") && name.ends_with(".xml") {
            let mut xml_str = String::new();
            if entry.read_to_string(&mut xml_str).is_ok() {
                if let Some(info) = parse_table_xml(&xml_str) {
                    tables.push(info);
                }
            }
        }
    }

    // Also look for table-related relationships to map table IDs to sheet names
    // For now, we extract what we can from the table XML directly
    // The sheet association is inferred from table part naming convention

    // Read workbook relationships and sheet rels to map table to sheet
    for i in 0..archive.len() {
        let mut entry = match archive.by_index(i) {
            Ok(e) => e,
            Err(_) => continue,
        };
        let name = entry.name().to_string();

        // Read sheet rels that reference table parts
        if name.starts_with("xl/worksheets/_rels/") && name.ends_with(".rels") {
            let mut rels_str = String::new();
            if entry.read_to_string(&mut rels_str).is_ok() {
                // Extract sheet name from rels path: _rels/sheet1.xml.rels -> sheet1
                let sheet_num = name
                    .strip_prefix("xl/worksheets/_rels/")
                    .and_then(|s| s.strip_suffix(".xml.rels"))
                    .unwrap_or("");

                // Find table references in this sheet's rels
                for line in rels_str.lines() {
                    if line.contains("table") && line.contains("Target=") {
                        if let Some(target) = extract_xml_attr(line, "Target") {
                            // Match table target to the tables we found
                            let table_file = target
                                .strip_prefix("../tables/")
                                .unwrap_or(&target);
                            for table in &mut tables {
                                // Set the sheet name based on relationship
                                if table.sheet.is_empty() {
                                    // We need to map sheet numbers to names
                                    // This requires reading workbook.xml for sheet names
                                    // For simplicity, leave sheet empty and fill later
                                }
                            }
                            let _ = (sheet_num, table_file);
                        }
                    }
                }
            }
        }
    }

    // Read workbook.xml to get sheet names mapping
    for i in 0..archive.len() {
        let mut entry = match archive.by_index(i) {
            Ok(e) => e,
            Err(_) => continue,
        };
        let name = entry.name().to_string();

        if name == "xl/workbook.xml" {
            let mut wb_xml = String::new();
            if entry.read_to_string(&mut wb_xml).is_ok() {
                let sheet_names: Vec<String> = wb_xml
                    .lines()
                    .filter(|l| l.contains("<sheet "))
                    .filter_map(|l| extract_xml_attr(l, "name"))
                    .collect();

                // Assign sheet names to tables based on worksheet rels
                // This is a simplified mapping: tables in order of discovery
                // A complete implementation would parse the full rels chain
                for (idx, table) in tables.iter_mut().enumerate() {
                    if table.sheet.is_empty() && idx < sheet_names.len() {
                        table.sheet = sheet_names[idx].clone();
                    }
                }
            }
        }
    }

    Ok(tables)
}

/// Parse a table XML string and extract TableInfo.
fn parse_table_xml(xml: &str) -> Option<TableInfo> {
    let name = extract_xml_attr_on_tag(xml, "<table ", "name")?;
    let display_name = extract_xml_attr_on_tag(xml, "<table ", "displayName");
    let range = extract_xml_attr_on_tag(xml, "<table ", "ref")?;

    let header_row_count = extract_xml_attr_on_tag(xml, "<table ", "headerRowCount")
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(1);
    let totals_row_count = extract_xml_attr_on_tag(xml, "<table ", "totalsRowCount")
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(0);

    // Extract style info
    let style_str = xml
        .lines()
        .find(|l| l.contains("<tableStyleInfo "))
        .map(|l| l.to_string())
        .unwrap_or_default();

    let style_name = extract_xml_attr_on_tag(&style_str, "<tableStyleInfo ", "name");
    let show_first = extract_xml_attr_on_tag(&style_str, "<tableStyleInfo ", "showFirstColumn")
        .map(|v| v == "1" || v == "true")
        .unwrap_or(false);
    let show_last = extract_xml_attr_on_tag(&style_str, "<tableStyleInfo ", "showLastColumn")
        .map(|v| v == "1" || v == "true")
        .unwrap_or(false);
    let show_rows = extract_xml_attr_on_tag(&style_str, "<tableStyleInfo ", "showRowStripes")
        .map(|v| v == "1" || v == "true")
        .unwrap_or(true);
    let show_cols = extract_xml_attr_on_tag(&style_str, "<tableStyleInfo ", "showColumnStripes")
        .map(|v| v == "1" || v == "true")
        .unwrap_or(false);

    // Map style name back to our preset
    let style = match style_name.as_deref() {
        Some("TableStyleNone") | None => TableStylePreset::None,
        Some(s) if s.starts_with("TableStyleLight") => {
            match &s["TableStyleLight".len()..] {
                "1" => TableStylePreset::Light1,
                "2" => TableStylePreset::Light2,
                "3" => TableStylePreset::Light3,
                "4" => TableStylePreset::Light4,
                "5" => TableStylePreset::Light5,
                "6" => TableStylePreset::Light6,
                "7" => TableStylePreset::Light7,
                "8" => TableStylePreset::Light8,
                "9" => TableStylePreset::Light9,
                "10" => TableStylePreset::Light10,
                "11" => TableStylePreset::Light11,
                "12" => TableStylePreset::Light12,
                "13" => TableStylePreset::Light13,
                "14" => TableStylePreset::Light14,
                "15" => TableStylePreset::Light15,
                "16" => TableStylePreset::Light16,
                "17" => TableStylePreset::Light17,
                "18" => TableStylePreset::Light18,
                "19" => TableStylePreset::Light19,
                "20" => TableStylePreset::Light20,
                "21" => TableStylePreset::Light21,
                _ => TableStylePreset::Medium2,
            }
        }
        Some(s) if s.starts_with("TableStyleMedium") => {
            match &s["TableStyleMedium".len()..] {
                "1" => TableStylePreset::Medium1,
                "2" => TableStylePreset::Medium2,
                "3" => TableStylePreset::Medium3,
                "4" => TableStylePreset::Medium4,
                "5" => TableStylePreset::Medium5,
                "6" => TableStylePreset::Medium6,
                "7" => TableStylePreset::Medium7,
                "8" => TableStylePreset::Medium8,
                "9" => TableStylePreset::Medium9,
                "10" => TableStylePreset::Medium10,
                "11" => TableStylePreset::Medium11,
                "12" => TableStylePreset::Medium12,
                "13" => TableStylePreset::Medium13,
                "14" => TableStylePreset::Medium14,
                "15" => TableStylePreset::Medium15,
                "16" => TableStylePreset::Medium16,
                "17" => TableStylePreset::Medium17,
                "18" => TableStylePreset::Medium18,
                "19" => TableStylePreset::Medium19,
                "20" => TableStylePreset::Medium20,
                "21" => TableStylePreset::Medium21,
                "22" => TableStylePreset::Medium22,
                "23" => TableStylePreset::Medium23,
                "24" => TableStylePreset::Medium24,
                "25" => TableStylePreset::Medium25,
                "26" => TableStylePreset::Medium26,
                "27" => TableStylePreset::Medium27,
                "28" => TableStylePreset::Medium28,
                _ => TableStylePreset::Medium2,
            }
        }
        Some(s) if s.starts_with("TableStyleDark") => {
            match &s["TableStyleDark".len()..] {
                "1" => TableStylePreset::Dark1,
                "2" => TableStylePreset::Dark2,
                "3" => TableStylePreset::Dark3,
                "4" => TableStylePreset::Dark4,
                "5" => TableStylePreset::Dark5,
                "6" => TableStylePreset::Dark6,
                "7" => TableStylePreset::Dark7,
                "8" => TableStylePreset::Dark8,
                "9" => TableStylePreset::Dark9,
                "10" => TableStylePreset::Dark10,
                "11" => TableStylePreset::Dark11,
                _ => TableStylePreset::Medium2,
            }
        }
        _ => TableStylePreset::Medium2,
    };

    Some(TableInfo {
        name,
        display_name,
        sheet: String::new(), // Will be resolved later
        range,
        has_header: header_row_count > 0,
        has_total: totals_row_count > 0,
        style,
        show_first_column: show_first,
        show_last_column: show_last,
        show_row_stripes: show_rows,
        show_column_stripes: show_cols,
    })
}

/// Helper: extract an XML attribute value from a line.
fn extract_xml_attr(line: &str, attr_name: &str) -> Option<String> {
    let pattern = format!("{}=\"", attr_name);
    if let Some(start) = line.find(&pattern) {
        let value_start = start + pattern.len();
        if let Some(end) = line[value_start..].find('"') {
            return Some(line[value_start..value_start + end].to_string());
        }
    }
    None
}

/// Helper: extract an XML attribute from a specific tag in multi-line XML.
fn extract_xml_attr_on_tag(xml: &str, tag: &str, attr_name: &str) -> Option<String> {
    for line in xml.lines() {
        if line.contains(tag) {
            if let Some(val) = extract_xml_attr(line, attr_name) {
                return Some(val);
            }
        }
    }
    None
}

/// Get information about a specific table by name.
pub fn get_table(path: &str, table_name: &str) -> Result<TableInfo> {
    let tables = list_tables(path)?;
    tables
        .into_iter()
        .find(|t| t.name.eq_ignore_ascii_case(table_name))
        .ok_or_else(|| {
            AppError::InvalidInput(format!("Table '{}' not found in workbook", table_name))
        })
}
