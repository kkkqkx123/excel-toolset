use crate::types::{
    AppError, DataValidationConfig, DataValidationType,
    Result, SheetProtectionConfig,
};
use crate::utils::cell_ref::index_to_col;


pub(crate) fn patch_merge_cells_str(xml: &[u8], new_range: &str) -> Result<Vec<u8>> {
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


pub(crate) fn count_merge_cells(xml: &str) -> usize {
    xml.matches("<mergeCell ").count()
}


pub(crate) fn patch_freeze_panes_str(xml: &[u8], rows: u32, cols: u16) -> Result<Vec<u8>> {
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


pub(crate) fn patch_auto_filter_str(xml: &[u8], new_range: Option<&str>) -> Result<Vec<u8>> {
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
    if let Some(range) = new_range
        && let Some(pos) = result.find("</worksheet>") {
            result.insert_str(pos, &format!("  <autoFilter ref=\"{}\"/>\n", range));
        }

    Ok(result.into_bytes())
}


pub(crate) fn build_data_validation_xml_str(config: &DataValidationConfig) -> String {
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


pub(crate) fn patch_data_validation_str(xml: &[u8], new_dv: &str) -> Result<Vec<u8>> {
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


pub(crate) fn build_sheet_protection_xml_str(config: &SheetProtectionConfig) -> String {
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


pub(crate) fn bool_to_01(b: bool) -> &'static str {
    if b { "1" } else { "0" }
}


pub(crate) fn patch_sheet_protection_str(xml: &[u8], new_sp: Option<&str>) -> Result<Vec<u8>> {
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
    if let Some(sp) = new_sp
        && let Some(pos) = result.find("</worksheet>") {
            result.insert_str(pos, &format!("  {}\n", sp));
        }

    Ok(result.into_bytes())
}


pub(crate) fn patch_sheet_visibility_str(wb_xml: &[u8], sheet_name: &str, state: Option<&str>) -> Result<Vec<u8>> {
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

