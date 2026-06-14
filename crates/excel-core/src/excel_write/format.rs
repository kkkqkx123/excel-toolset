use rust_xlsxwriter::{ChartType as XlsxChartType, Color, Format, FormatBorder};

use crate::types::*;

pub fn build_format(style: &Style) -> Format {
    let mut fmt = Format::new();
    if let Some(ref name) = style.font_name {
        fmt = fmt.set_font_name(name);
    }
    if let Some(size) = style.font_size {
        fmt = fmt.set_font_size(size);
    }
    if let Some(true) = style.bold {
        fmt = fmt.set_bold();
    }
    if let Some(true) = style.italic {
        fmt = fmt.set_italic();
    }
    if let Some(ref color) = style.font_color
        && let Some(c) = parse_color(color)
    {
        fmt = fmt.set_font_color(c);
    }
    if let Some(ref bg) = style.background_color
        && let Some(c) = parse_color(bg)
    {
        fmt = fmt.set_background_color(c);
    }
    if let Some(ref border) = style.border {
        let b = match border.to_lowercase().as_str() {
            "thin" => FormatBorder::Thin,
            "medium" => FormatBorder::Medium,
            "thick" => FormatBorder::Thick,
            "double" => FormatBorder::Double,
            "dotted" => FormatBorder::Dotted,
            "dashed" => FormatBorder::Dashed,
            _ => FormatBorder::Thin,
        };
        fmt = fmt.set_border(b);
    }
    fmt
}

pub fn parse_color(color: &str) -> Option<Color> {
    let s = color.trim_start_matches('#');
    if s.len() == 6 {
        u32::from_str_radix(s, 16)
            .ok()
            .map(|v| Color::RGB(v | 0xFF000000))
    } else if s.len() == 8 {
        u32::from_str_radix(s, 16).ok().map(Color::RGB)
    } else {
        match s.to_lowercase().as_str() {
            "red" => Some(Color::Red),
            "blue" => Some(Color::Blue),
            "green" => Some(Color::Green),
            "yellow" => Some(Color::Yellow),
            "white" => Some(Color::White),
            "black" => Some(Color::Black),
            "orange" => Some(Color::Orange),
            "purple" => Some(Color::Purple),
            "pink" => Some(Color::Pink),
            "cyan" => Some(Color::Cyan),
            "brown" => Some(Color::Brown),
            "magenta" => Some(Color::Magenta),
            "gray" => Some(Color::Gray),
            "lime" => Some(Color::Lime),
            "navy" => Some(Color::Navy),
            _ => None,
        }
    }
}

pub fn map_chart_type(ct: &ChartType) -> XlsxChartType {
    match ct {
        ChartType::Column => XlsxChartType::Column,
        ChartType::Line => XlsxChartType::Line,
        ChartType::Pie => XlsxChartType::Pie,
        ChartType::Bar => XlsxChartType::Bar,
        ChartType::Area => XlsxChartType::Area,
        ChartType::Scatter => XlsxChartType::Scatter,
    }
}
