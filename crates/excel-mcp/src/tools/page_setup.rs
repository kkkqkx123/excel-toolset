// Page setup category tools: configure, set/clear page breaks.

use serde_json::Value;
use std::collections::HashMap;

use super::helpers::*;
use crate::server::{ToolDef, ToolHandler};
use excel_core::types::*;

pub fn tools() -> Vec<ToolDef> {
    vec![
        ToolDef {
            name: "excel_page_setup_configure",
            description: "Configure page setup settings for a worksheet: orientation, paper size, margins, print area, print titles, scaling, gridlines, headings, and centering.",
            input_schema: object_schema(
                vec![
                    ("path", string_prop("Path to the .xlsx file", true)),
                    ("sheet", string_prop("Target sheet name", true)),
                    ("orientation", string_prop("Page orientation: portrait or landscape", false)),
                    ("paper_size", enum_prop("Paper size", &["a4", "a3", "letter", "legal", "executive", "a5", "b4", "b5"])),
                    ("print_area", string_prop("Print area range, e.g. A1:G50", false)),
                    ("print_title_rows", string_prop("Rows repeated at top of each page, e.g. 1:3", false)),
                    ("print_title_cols", string_prop("Columns repeated at left of each page, e.g. A:B", false)),
                    ("fit_to_pages_width", int_prop("Fit to pages width")),
                    ("fit_to_pages_height", int_prop("Fit to pages height")),
                    ("scale", int_prop("Print scale percentage (100 = 100%)")),
                    ("print_gridlines", bool_prop("Print gridlines", Some(false))),
                    ("print_headings", bool_prop("Print row/column headings", Some(false))),
                    ("center_horizontally", bool_prop("Center horizontally on page", Some(false))),
                    ("center_vertically", bool_prop("Center vertically on page", Some(false))),
                    ("margins_left", int_prop("Left margin (multiplied by 100 for precision, e.g. 70 = 0.7 inches)")),
                    ("margins_right", int_prop("Right margin (multiplied by 100)")),
                    ("margins_top", int_prop("Top margin (multiplied by 100)")),
                    ("margins_bottom", int_prop("Bottom margin (multiplied by 100)")),
                    ("margins_header", int_prop("Header margin (multiplied by 100)")),
                    ("margins_footer", int_prop("Footer margin (multiplied by 100)")),
                ],
                vec!["path", "sheet"],
            ),
        },
        ToolDef {
            name: "excel_page_setup_page_breaks_set",
            description: "Set horizontal and vertical page breaks on a worksheet.",
            input_schema: object_schema(
                vec![
                    ("path", string_prop("Path to the .xlsx file", true)),
                    ("sheet", string_prop("Target sheet name", true)),
                    ("horizontal_breaks", string_array_prop("Row indices for horizontal page breaks (0-indexed)")),
                    ("vertical_breaks", string_array_prop("Column indices for vertical page breaks (0-indexed)")),
                ],
                vec!["path", "sheet"],
            ),
        },
        ToolDef {
            name: "excel_page_setup_page_breaks_clear",
            description: "Clear all page breaks from a worksheet.",
            input_schema: object_schema(
                vec![
                    ("path", string_prop("Path to the .xlsx file", true)),
                    ("sheet", string_prop("Target sheet name", true)),
                ],
                vec!["path", "sheet"],
            ),
        },
    ]
}

pub fn register(handlers: &mut HashMap<String, ToolHandler>) {
    handlers.insert("excel_page_setup_configure".into(), handle_page_setup_configure);
    handlers.insert("excel_page_setup_page_breaks_set".into(), handle_page_setup_page_breaks_set);
    handlers.insert("excel_page_setup_page_breaks_clear".into(), handle_page_setup_page_breaks_clear);
}

fn handle_page_setup_configure(args: Value) -> String {
    let path = get_string(&args, "path").unwrap_or_default();
    let sheet = get_string(&args, "sheet").unwrap_or_default();
    let params = security_params(&path, false);

    let orientation = get_string(&args, "orientation")
        .and_then(|s| serde_json::from_str::<PageOrientation>(&format!("\"{}\"", s)).ok());

    let paper_size = get_string(&args, "paper_size")
        .and_then(|s| serde_json::from_str::<PaperSize>(&format!("\"{}\"", s)).ok());

    let margins_left = get_u32(&args, "margins_left");
    let margins_right = get_u32(&args, "margins_right");
    let margins_top = get_u32(&args, "margins_top");
    let margins_bottom = get_u32(&args, "margins_bottom");
    let margins_header = get_u32(&args, "margins_header");
    let margins_footer = get_u32(&args, "margins_footer");

    let margins = if margins_left.is_some()
        || margins_right.is_some()
        || margins_top.is_some()
        || margins_bottom.is_some()
    {
        Some(PageMargins {
            left: margins_left.unwrap_or(70) as f64 / 100.0,
            right: margins_right.unwrap_or(70) as f64 / 100.0,
            top: margins_top.unwrap_or(75) as f64 / 100.0,
            bottom: margins_bottom.unwrap_or(75) as f64 / 100.0,
            header: margins_header.unwrap_or(30) as f64 / 100.0,
            footer: margins_footer.unwrap_or(30) as f64 / 100.0,
        })
    } else {
        None
    };

    let fit_to_pages = match (
        get_u32(&args, "fit_to_pages_width"),
        get_u32(&args, "fit_to_pages_height"),
    ) {
        (Some(w), Some(h)) => Some(FitToPages {
            width: w as u16,
            height: h as u16,
        }),
        _ => None,
    };

    let config = PageSetupConfig {
        sheet,
        orientation,
        paper_size,
        margins,
        print_area: get_string(&args, "print_area"),
        print_title_rows: get_string(&args, "print_title_rows"),
        print_title_cols: get_string(&args, "print_title_cols"),
        fit_to_pages,
        scale: get_u32(&args, "scale").map(|s| s as u16),
        print_gridlines: get_bool(&args, "print_gridlines").unwrap_or(false),
        print_headings: get_bool(&args, "print_headings").unwrap_or(false),
        center_horizontally: get_bool(&args, "center_horizontally").unwrap_or(false),
        center_vertically: get_bool(&args, "center_vertically").unwrap_or(false),
    };

    match excel_core::excel_write::configure_page_setup(&path, &params, &config) {
        Ok(result) => to_result_string(&result),
        Err(e) => format!("Error: {e}"),
    }
}

fn handle_page_setup_page_breaks_set(args: Value) -> String {
    let path = get_string(&args, "path").unwrap_or_default();
    let sheet = get_string(&args, "sheet").unwrap_or_default();
    let params = security_params(&path, false);

    let horizontal_breaks: Vec<u32> = get_string_array(&args, "horizontal_breaks")
        .unwrap_or_default()
        .iter()
        .filter_map(|s| s.parse::<u32>().ok())
        .collect();

    let vertical_breaks: Vec<u16> = get_string_array(&args, "vertical_breaks")
        .unwrap_or_default()
        .iter()
        .filter_map(|s| s.parse::<u16>().ok())
        .collect();

    let config = PageBreakConfig {
        sheet,
        horizontal_breaks,
        vertical_breaks,
    };

    match excel_core::excel_write::set_page_breaks(&path, &params, &config) {
        Ok(result) => to_result_string(&result),
        Err(e) => format!("Error: {e}"),
    }
}

fn handle_page_setup_page_breaks_clear(args: Value) -> String {
    let path = get_string(&args, "path").unwrap_or_default();
    let sheet = get_string(&args, "sheet").unwrap_or_default();
    let params = security_params(&path, false);

    match excel_core::excel_write::clear_page_breaks(&path, &params, &sheet) {
        Ok(result) => to_result_string(&result),
        Err(e) => format!("Error: {e}"),
    }
}
