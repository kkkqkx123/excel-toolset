// Image and shape category tools: insert image, remove image, insert shape.

use serde_json::Value;
use std::collections::HashMap;

use super::helpers::*;
use crate::server::{ToolDef, ToolHandler};
use excel_core::types::*;

pub fn tools() -> Vec<ToolDef> {
    vec![
        ToolDef {
            name: "excel_image_insert",
            description: "Insert an image (PNG, JPG, GIF, SVG) into a worksheet at a specified anchor cell position.",
            input_schema: object_schema(
                vec![
                    ("path", string_prop("Path to the .xlsx file", true)),
                    ("sheet", string_prop("Target sheet name", true)),
                    ("image_path", string_prop("Path to the image file on disk", true)),
                    ("anchor_cell", string_prop("Anchor cell for image placement, e.g. B2", true)),
                    ("x_scale", int_prop("Horizontal scale factor (multiplied by 100, e.g. 100 = 1.0x)")),
                    ("y_scale", int_prop("Vertical scale factor (multiplied by 100, e.g. 100 = 1.0x)")),
                    ("alt_text", string_prop("Alternative text for accessibility", false)),
                ],
                vec!["path", "sheet", "image_path", "anchor_cell"],
            ),
        },
        ToolDef {
            name: "excel_image_remove",
            description: "Remove images from a worksheet at a specified anchor cell position.",
            input_schema: object_schema(
                vec![
                    ("path", string_prop("Path to the .xlsx file", true)),
                    ("sheet", string_prop("Target sheet name", true)),
                    ("anchor_cell", string_prop("Anchor cell where the image was placed, e.g. B2", true)),
                ],
                vec!["path", "sheet", "anchor_cell"],
            ),
        },
        ToolDef {
            name: "excel_shape_insert",
            description: "Insert a shape (rectangle, rounded_rectangle, ellipse, line, text_box) into a worksheet.",
            input_schema: object_schema(
                vec![
                    ("path", string_prop("Path to the .xlsx file", true)),
                    ("sheet", string_prop("Target sheet name", true)),
                    ("shape_type", enum_prop("Shape type", &["rectangle", "rounded_rectangle", "ellipse", "line", "text_box"])),
                    ("anchor_cell", string_prop("Anchor cell for shape placement, e.g. B2", true)),
                    ("width", int_prop("Width in pixels",)),
                    ("height", int_prop("Height in pixels",)),
                    ("fill_color", string_prop("Fill color as hex string, e.g. FF0000", false)),
                    ("line_color", string_prop("Line/border color as hex string, e.g. 000000", false)),
                    ("alt_text", string_prop("Alternative text for accessibility", false)),
                ],
                vec!["path", "sheet", "shape_type", "anchor_cell", "width", "height"],
            ),
        },
    ]
}

pub fn register(handlers: &mut HashMap<String, ToolHandler>) {
    handlers.insert("excel_image_insert".into(), handle_image_insert);
    handlers.insert("excel_image_remove".into(), handle_image_remove);
    handlers.insert("excel_shape_insert".into(), handle_shape_insert);
}

fn handle_image_insert(args: Value) -> String {
    let path = get_string(&args, "path").unwrap_or_default();
    let sheet = get_string(&args, "sheet").unwrap_or_default();
    let image_path = get_string(&args, "image_path").unwrap_or_default();
    let anchor_cell = get_string(&args, "anchor_cell").unwrap_or_default();
    let params = security_params(&path, false);

    let x_scale = get_u32(&args, "x_scale");
    let y_scale = get_u32(&args, "y_scale");

    let scale = match (x_scale, y_scale) {
        (Some(x), Some(y)) => Some(ImageScale {
            x_scale: x as f64 / 100.0,
            y_scale: y as f64 / 100.0,
        }),
        _ => None,
    };

    let config = ImageConfig {
        sheet,
        image_path,
        anchor_cell,
        scale,
        x_offset: None,
        y_offset: None,
        alt_text: get_string(&args, "alt_text"),
    };

    match excel_core::excel_write::insert_image(&path, &params, &config) {
        Ok(result) => to_result_string(&result),
        Err(e) => format!("Error: {e}"),
    }
}

fn handle_image_remove(args: Value) -> String {
    let path = get_string(&args, "path").unwrap_or_default();
    let sheet = get_string(&args, "sheet").unwrap_or_default();
    let anchor_cell = get_string(&args, "anchor_cell").unwrap_or_default();
    let params = security_params(&path, false);

    match excel_core::excel_write::remove_image(&path, &params, &sheet, &anchor_cell) {
        Ok(result) => to_result_string(&result),
        Err(e) => format!("Error: {e}"),
    }
}

fn handle_shape_insert(args: Value) -> String {
    let path = get_string(&args, "path").unwrap_or_default();
    let sheet = get_string(&args, "sheet").unwrap_or_default();
    let shape_type_str = get_string(&args, "shape_type").unwrap_or_else(|| "rectangle".to_string());
    let anchor_cell = get_string(&args, "anchor_cell").unwrap_or_default();
    let width = get_u32(&args, "width").unwrap_or(100);
    let height = get_u32(&args, "height").unwrap_or(100);
    let params = security_params(&path, false);

    let shape_type: ShapeType = serde_json::from_str(&format!("\"{}\"", shape_type_str))
        .unwrap_or(ShapeType::Rectangle);

    let config = ShapeConfig {
        sheet,
        shape_type,
        anchor_cell,
        width,
        height,
        fill_color: get_string(&args, "fill_color"),
        line_color: get_string(&args, "line_color"),
        line_width: get_u32(&args, "line_width").map(|v| v as f64 * 0.1),
        alt_text: get_string(&args, "alt_text"),
    };

    match excel_core::excel_write::insert_shape(&path, &params, &config) {
        Ok(result) => to_result_string(&result),
        Err(e) => format!("Error: {e}"),
    }
}
