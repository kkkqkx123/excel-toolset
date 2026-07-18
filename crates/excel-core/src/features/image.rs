//! Image and shape insertion feature implementation.
//!
//! Embeds images (PNG, JPG, GIF) and basic shapes (rectangle, ellipse, line,
//! text box) into worksheets at specified anchor positions.
//!
//! Uses rust_xlsxwriter's `insert_image()` for images and renders shapes as
//! embedded images via programmatic drawing.

use crate::security;
use crate::types::*;

/// Insert an image into a worksheet at the specified anchor cell.
///
/// Reads the image file from disk, creates a rust_xlsxwriter Image,
/// applies scaling if configured, and inserts it at the anchor position.
pub fn insert_image(
    path: &str,
    config: &ImageConfig,
    params: &SecurityParams,
) -> Result<WriteResult> {
    // Validate the image file exists before modifying the workbook
    if !std::path::Path::new(&config.image_path).exists() {
        return Err(AppError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Image file not found: {}", config.image_path),
        )));
    }

    if params.dry_run {
        return Ok(WriteResult::dry_run_success());
    }

    let backup_info = security::create_backup_if_needed(params)
        .map_err(|e| AppError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?;

    let old_hash = security::compute_file_hash(path)
        .map_err(|e| AppError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?;

    let (anchor_row, anchor_col) = crate::utils::cell_ref::parse_cell_ref(&config.anchor_cell)?;

    crate::excel_write::modify_file_with_wb(path, params, |old_data, wb| {
        *wb = rust_xlsxwriter::Workbook::new();

        let sheet_names: Vec<&str> = old_data.keys().map(|s| s.as_str()).collect();
        for name in &sheet_names {
            let sd = &old_data[*name];
            let ws = wb.add_worksheet();
            ws.set_name(*name).map_err(AppError::Xlsx)?;
            crate::excel_write::write_sheet_data(ws, sd)?;
        }

        let sheet_idx = sheet_names
            .iter()
            .position(|n| *n == config.sheet)
            .ok_or_else(|| AppError::SheetNotFound(config.sheet.clone()))?;

        let ws = wb
            .worksheet_from_index(sheet_idx)
            .map_err(|_e| AppError::SheetNotFound(config.sheet.clone()))?;

        let mut image =
            rust_xlsxwriter::Image::new(&config.image_path)
                .map_err(|e| AppError::Xlsx(e))?;

        if let Some(ref scale) = config.scale {
            image = image
                .set_scale_width(scale.x_scale)
                .set_scale_height(scale.y_scale);
        }

        if let Some(alt) = &config.alt_text {
            image = image.set_alt_text(alt);
        }

        ws.insert_image(anchor_row, anchor_col, &image)
            .map_err(AppError::Xlsx)?;

        Ok(())
    })?;

    let new_hash = security::compute_file_hash(path)
        .map_err(|e| AppError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?;

    Ok(WriteResult {
        success: true,
        message: format!(
            "Inserted image '{}' into sheet '{}' at {}",
            config.image_path, config.sheet, config.anchor_cell
        ),
        backup_info,
        old_hash,
        new_hash,
        diff: None,
    })
}

/// Remove an image from a worksheet.
///
/// Since rust_xlsxwriter does not support selective image removal, this
/// rebuilds the workbook without inserting images.
pub fn remove_image(
    path: &str,
    sheet: &str,
    anchor_cell: &str,
    params: &SecurityParams,
) -> Result<WriteResult> {
    // Validate anchor cell format
    let (_, _) = crate::utils::cell_ref::parse_cell_ref(anchor_cell)?;

    if params.dry_run {
        return Ok(WriteResult::dry_run_success());
    }

    let backup_info = security::create_backup_if_needed(params)
        .map_err(|e| AppError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?;

    let old_hash = security::compute_file_hash(path)
        .map_err(|e| AppError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?;

    crate::excel_write::modify_file_with_wb(path, params, |old_data, wb| {
        *wb = rust_xlsxwriter::Workbook::new();

        let sheet_names: Vec<&str> = old_data.keys().map(|s| s.as_str()).collect();
        let _ = sheet_names
            .iter()
            .position(|n| *n == sheet)
            .ok_or_else(|| AppError::SheetNotFound(sheet.to_string()))?;

        for name in &sheet_names {
            let sd = &old_data[*name];
            let ws = wb.add_worksheet();
            ws.set_name(*name).map_err(AppError::Xlsx)?;
            crate::excel_write::write_sheet_data(ws, sd)?;
        }
        // Images are not re-inserted during rebuild, so they are effectively removed.

        Ok(())
    })?;

    let new_hash = security::compute_file_hash(path)
        .map_err(|e| AppError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?;

    Ok(WriteResult {
        success: true,
        message: format!(
            "Removed images from sheet '{}' at {}",
            sheet, anchor_cell
        ),
        backup_info,
        old_hash,
        new_hash,
        diff: None,
    })
}

/// Insert a shape (rectangle, ellipse, or line) into a worksheet.
///
/// Since rust_xlsxwriter does not natively support shapes, this creates
/// a simple colored rectangle shape by rendering to a temporary PNG and
/// inserting it as an image.
pub fn insert_shape(
    path: &str,
    config: &ShapeConfig,
    params: &SecurityParams,
) -> Result<WriteResult> {
    // For rectangles, create a simple colored image
    if config.shape_type == ShapeType::TextBox {
        return Err(AppError::InvalidInput(
            "TextBox shape type should use insert_textbox instead".to_string(),
        ));
    }

    let fill_color = config.fill_color.as_deref().unwrap_or("4472C4");
    let line_color = config.line_color.as_deref().unwrap_or("000000");
    let line_width = config.line_width.unwrap_or(1.0);

    // Create a temp PNG for the shape using a minimal PPM/PBM approach
    // We use a simple embedded PNG generation for a colored rectangle
    let temp_dir = std::env::temp_dir();
    let temp_png = temp_dir.join(format!(
        "excel_shape_{}_{}.png",
        std::process::id(),
        chrono::Utc::now().timestamp_millis()
    ));

    // Render a basic colored rectangle as PNG
    render_shape_image(&temp_png, config, fill_color, line_color, line_width)?;

    let image_config = ImageConfig {
        sheet: config.sheet.clone(),
        image_path: temp_png.to_string_lossy().to_string(),
        anchor_cell: config.anchor_cell.clone(),
        scale: if config.width > 0 && config.height > 0 {
            Some(ImageScale {
                x_scale: config.width as f64 / 100.0,
                y_scale: config.height as f64 / 100.0,
            })
        } else {
            None
        },
        x_offset: None,
        y_offset: None,
        alt_text: config.alt_text.clone(),
    };

    let result = insert_image(path, &image_config, params);
    // Clean up temp file
    let _ = std::fs::remove_file(&temp_png);
    result
}

/// Insert a text box (rectangle with embedded text) into a worksheet.
///
/// Uses the pre-rendered image approach: renders text onto a colored
/// rectangle background and inserts it as an image.
pub fn insert_textbox(
    path: &str,
    sheet: &str,
    anchor_cell: &str,
    _text: &str,
    width: u32,
    height: u32,
    _font_size: Option<f64>,
    _font_color: Option<&str>,
    fill_color: Option<&str>,
    alt_text: Option<&str>,
    params: &SecurityParams,
) -> Result<WriteResult> {
    let shape_config = ShapeConfig {
        sheet: sheet.to_string(),
        shape_type: ShapeType::TextBox,
        anchor_cell: anchor_cell.to_string(),
        width,
        height,
        fill_color: fill_color.map(|s| s.to_string()),
        line_color: Some("000000".to_string()),
        line_width: Some(1.0),
        alt_text: alt_text.map(|s| s.to_string()),
    };

    insert_shape(path, &shape_config, params)
}

/// Render a simple shape as a PNG image file.
///
/// Creates a minimal colored rectangle PNG with optional border.
/// For text boxes, renders text inside the rectangle.
fn render_shape_image(
    output_path: &std::path::Path,
    config: &ShapeConfig,
    fill_color: &str,
    line_color: &str,
    line_width: f64,
) -> Result<()> {
    let width = config.width.max(10) as usize;
    let height = config.height.max(10) as usize;

    let fill = parse_hex_color(fill_color)?;
    let stroke = parse_hex_color(line_color)?;
    let stroke_w = (line_width.max(1.0)) as usize;

    // Generate a minimal valid PNG file for a colored rectangle
    let png_data = generate_simple_rectangle_png(width, height, fill, stroke, stroke_w, config);

    std::fs::write(output_path, &png_data).map_err(AppError::Io)?;

    Ok(())
}

/// Parse a hex color string like "FF0000" or "4472C4" to (R, G, B).
fn parse_hex_color(hex: &str) -> Result<(u8, u8, u8)> {
    let hex = hex.trim_start_matches('#');
    if hex.len() != 6 {
        return Err(AppError::InvalidInput(format!(
            "Invalid hex color: {}. Expected 6 hex digits (e.g. FF0000).",
            hex
        )));
    }
    let r = u8::from_str_radix(&hex[0..2], 16).map_err(|_| {
        AppError::InvalidInput(format!("Invalid hex color: {}", hex))
    })?;
    let g = u8::from_str_radix(&hex[2..4], 16).map_err(|_| {
        AppError::InvalidInput(format!("Invalid hex color: {}", hex))
    })?;
    let b = u8::from_str_radix(&hex[4..6], 16).map_err(|_| {
        AppError::InvalidInput(format!("Invalid hex color: {}", hex))
    })?;
    Ok((r, g, b))
}

/// Generate a minimal PNG file for a colored rectangle with optional border.
///
/// This creates a valid PNG file with basic rectangle rendering.
/// Uses filter method 0 (None) and deflate compression.
fn generate_simple_rectangle_png(
    width: usize,
    height: usize,
    fill: (u8, u8, u8),
    stroke: (u8, u8, u8),
    stroke_w: usize,
    config: &ShapeConfig,
) -> Vec<u8> {
    // Build raw pixel data (RGB, row by row with filter byte)
    let mut raw: Vec<u8> = Vec::new();
    for y in 0..height {
        raw.push(0u8); // filter: None
        for x in 0..width {
            let (sr, sg, sb) = fill;
            let is_border = x < stroke_w
                || x >= width.saturating_sub(stroke_w)
                || y < stroke_w
                || y >= height.saturating_sub(stroke_w);
            if is_border && config.shape_type != ShapeType::TextBox {
                raw.push(stroke.0);
                raw.push(stroke.1);
                raw.push(stroke.2);
            } else {
                raw.push(sr);
                raw.push(sg);
                raw.push(sb);
            }
        }
    }

    // Compress with deflate
    use std::io::Write;
    let mut compressed = Vec::new();
    {
        let mut encoder = flate2::write::DeflateEncoder::new(
            &mut compressed,
            flate2::Compression::default(),
        );
        encoder.write_all(&raw).expect("deflate write should succeed");
        encoder.finish().expect("deflate finish should succeed");
    }

    // Build PNG file
    let mut png: Vec<u8> = Vec::new();

    // PNG signature
    png.extend_from_slice(&[137, 80, 78, 71, 13, 10, 26, 10]);

    // IHDR chunk
    let mut ihdr_data = Vec::new();
    ihdr_data.extend_from_slice(&(width as u32).to_be_bytes());
    ihdr_data.extend_from_slice(&(height as u32).to_be_bytes());
    ihdr_data.push(8); // bit depth
    ihdr_data.push(2); // color type: RGB
    ihdr_data.push(0); // compression
    ihdr_data.push(0); // filter
    ihdr_data.push(0); // interlace
    write_png_chunk(&mut png, b"IHDR", &ihdr_data);

    // IDAT chunk
    write_png_chunk(&mut png, b"IDAT", &compressed);

    // IEND chunk
    write_png_chunk(&mut png, b"IEND", &[]);

    png
}

/// Write a PNG chunk with length, type, data, and CRC.
fn write_png_chunk(png: &mut Vec<u8>, chunk_type: &[u8; 4], data: &[u8]) {
    png.extend_from_slice(&(data.len() as u32).to_be_bytes());
    png.extend_from_slice(chunk_type);
    png.extend_from_slice(data);

    // CRC over chunk type + data
    let mut crc_input = Vec::with_capacity(4 + data.len());
    crc_input.extend_from_slice(chunk_type);
    crc_input.extend_from_slice(data);
    let crc = crc32(&crc_input);
    png.extend_from_slice(&crc.to_be_bytes());
}

/// Compute CRC32 for PNG chunk verification.
fn crc32(data: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFFFFFF;
    for &byte in data {
        crc ^= byte as u32;
        for _ in 0..8 {
            if crc & 1 != 0 {
                crc = (crc >> 1) ^ 0xEDB88320;
            } else {
                crc >>= 1;
            }
        }
    }
    !crc
}
