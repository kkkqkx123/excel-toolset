use crate::security;
use crate::types::*;
use serde::{Deserialize, Serialize};

// ── Core rule types ──

#[derive(Debug, Clone)]
pub struct ConditionalFormatRule {
    pub rule_type: ConditionalFormatType,
    pub condition: String,
    pub format: Option<Style>,
    pub config: Option<ConditionalFormatConfig>,
}

#[derive(Debug, Clone)]
pub enum ConditionalFormatType {
    CellValue,
    Formula,
    AboveAverage,
    Top10,
    Duplicate,
    TextContains,
    DateOccurring,
    DataBar,
    ColorScale,
    IconSet,
}

// ── Extended config types for DataBar / ColorScale / IconSet ──

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ConditionalFormatConfig {
    DataBar {
        fill_color: Option<String>,
        border_color: Option<String>,
        direction: Option<String>,
        min_type: Option<String>,
        max_type: Option<String>,
    },
    ColorScale {
        #[serde(default = "default_color_scale_variant")]
        variant: String,
        min_color: Option<String>,
        mid_color: Option<String>,
        max_color: Option<String>,
    },
    IconSet {
        icon_type: String,
        show_value: Option<bool>,
        reverse_order: Option<bool>,
    },
}

fn default_color_scale_variant() -> String {
    "2_color".to_string()
}

/// Parse icon type string to rust_xlsxwriter enum.
fn parse_icon_type(s: &str) -> rust_xlsxwriter::ConditionalFormatIconType {
    match s.to_lowercase().as_str() {
        "3_arrows" | "three_arrows" => rust_xlsxwriter::ConditionalFormatIconType::ThreeArrows,
        "3_arrows_gray" | "three_arrows_gray" => {
            rust_xlsxwriter::ConditionalFormatIconType::ThreeArrowsGray
        }
        "3_flags" | "three_flags" => rust_xlsxwriter::ConditionalFormatIconType::ThreeFlags,
        "3_traffic_lights" | "three_traffic_lights" => {
            rust_xlsxwriter::ConditionalFormatIconType::ThreeTrafficLights
        }
        "3_traffic_lights_rim" | "three_traffic_lights_with_rim" => {
            rust_xlsxwriter::ConditionalFormatIconType::ThreeTrafficLightsWithRim
        }
        "3_signs" | "three_signs" => rust_xlsxwriter::ConditionalFormatIconType::ThreeSigns,
        "3_stars" | "three_stars" => rust_xlsxwriter::ConditionalFormatIconType::ThreeStars,
        "3_triangles" | "three_triangles" => {
            rust_xlsxwriter::ConditionalFormatIconType::ThreeTriangles
        }
        "3_symbols" | "three_symbols_circled" => {
            rust_xlsxwriter::ConditionalFormatIconType::ThreeSymbolsCircled
        }
        "3_symbols2" | "three_symbols" => rust_xlsxwriter::ConditionalFormatIconType::ThreeSymbols,
        "4_arrows" | "four_arrows" => rust_xlsxwriter::ConditionalFormatIconType::FourArrows,
        "4_arrows_gray" | "four_arrows_gray" => {
            rust_xlsxwriter::ConditionalFormatIconType::FourArrowsGray
        }
        "4_red_to_black" | "four_red_to_black" => {
            rust_xlsxwriter::ConditionalFormatIconType::FourRedToBlack
        }
        "4_histograms" | "four_histograms" => {
            rust_xlsxwriter::ConditionalFormatIconType::FourHistograms
        }
        "4_traffic_lights" | "four_traffic_lights" => {
            rust_xlsxwriter::ConditionalFormatIconType::FourTrafficLights
        }
        "5_arrows" | "five_arrows" => rust_xlsxwriter::ConditionalFormatIconType::FiveArrows,
        "5_arrows_gray" | "five_arrows_gray" => {
            rust_xlsxwriter::ConditionalFormatIconType::FiveArrowsGray
        }
        "5_histograms" | "five_histograms" => {
            rust_xlsxwriter::ConditionalFormatIconType::FiveHistograms
        }
        "5_quadrants" | "five_quadrants" => {
            rust_xlsxwriter::ConditionalFormatIconType::FiveQuadrants
        }
        "5_boxes" | "five_boxes" => rust_xlsxwriter::ConditionalFormatIconType::FiveBoxes,
        _ => rust_xlsxwriter::ConditionalFormatIconType::ThreeTrafficLights,
    }
}

/// Parse date occurring string to rust_xlsxwriter enum.
fn parse_date_rule(s: &str) -> rust_xlsxwriter::ConditionalFormatDateRule {
    match s.to_lowercase().as_str() {
        "yesterday" => rust_xlsxwriter::ConditionalFormatDateRule::Yesterday,
        "today" => rust_xlsxwriter::ConditionalFormatDateRule::Today,
        "tomorrow" => rust_xlsxwriter::ConditionalFormatDateRule::Tomorrow,
        "last7days" | "last_7_days" | "last7" => {
            rust_xlsxwriter::ConditionalFormatDateRule::Last7Days
        }
        "lastweek" | "last_week" => rust_xlsxwriter::ConditionalFormatDateRule::LastWeek,
        "thisweek" | "this_week" => rust_xlsxwriter::ConditionalFormatDateRule::ThisWeek,
        "nextweek" | "next_week" => rust_xlsxwriter::ConditionalFormatDateRule::NextWeek,
        "lastmonth" | "last_month" => rust_xlsxwriter::ConditionalFormatDateRule::LastMonth,
        "thismonth" | "this_month" => rust_xlsxwriter::ConditionalFormatDateRule::ThisMonth,
        "nextmonth" | "next_month" => rust_xlsxwriter::ConditionalFormatDateRule::NextMonth,
        _ => rust_xlsxwriter::ConditionalFormatDateRule::Today,
    }
}

// ── Public helpers for string-to-enum parsing (used by CLI/HTTP) ──

pub fn parse_rule_type(s: &str) -> ConditionalFormatType {
    match s.to_lowercase().as_str() {
        "cellvalue" | "cell_value" | "cell" => ConditionalFormatType::CellValue,
        "formula" => ConditionalFormatType::Formula,
        "aboveaverage" | "above_average" => ConditionalFormatType::AboveAverage,
        "top10" => ConditionalFormatType::Top10,
        "duplicate" => ConditionalFormatType::Duplicate,
        "textcontains" | "text_contains" => ConditionalFormatType::TextContains,
        "dateoccurring" | "date_occurring" => ConditionalFormatType::DateOccurring,
        "databar" | "data_bar" => ConditionalFormatType::DataBar,
        "colorscale" | "color_scale" => ConditionalFormatType::ColorScale,
        "iconset" | "icon_set" => ConditionalFormatType::IconSet,
        _ => ConditionalFormatType::CellValue,
    }
}

// ── Write logic ──

pub fn add_conditional_format(
    path: &str,
    sheet: &str,
    range: &str,
    rule: &ConditionalFormatRule,
    params: &SecurityParams,
) -> Result<WriteResult> {
    if params.dry_run {
        return Ok(WriteResult::dry_run_success());
    }

    security::create_backup_if_needed(params)?;

    let (r1, c1, r2, c2) = crate::utils::cell_ref::parse_range(range)?;

    crate::excel_write::modify_file_with_wb(path, params, |_, wb| {
        let worksheet = wb
            .worksheet_from_name(sheet)
            .map_err(|_e| AppError::SheetNotFound(sheet.to_string()))?;

        match rule.rule_type {
            ConditionalFormatType::CellValue => {
                let fmt = build_cf_format(&rule.format);
                let cond_format = rust_xlsxwriter::ConditionalFormatCell::new()
                    .set_rule(rust_xlsxwriter::ConditionalFormatCellRule::GreaterThan(
                        rule.condition.as_str(),
                    ))
                    .set_format(&fmt);
                worksheet
                    .add_conditional_format(r1, c1, r2, c2, &cond_format)
                    .map_err(|e| AppError::Write(e.to_string()))?;
            }
            ConditionalFormatType::Formula => {
                let fmt = build_cf_format(&rule.format);
                let cond_format = rust_xlsxwriter::ConditionalFormatFormula::new()
                    .set_rule(rule.condition.as_str())
                    .set_format(&fmt);
                worksheet
                    .add_conditional_format(r1, c1, r2, c2, &cond_format)
                    .map_err(|e| AppError::Write(e.to_string()))?;
            }
            ConditionalFormatType::Duplicate => {
                let fmt = build_cf_format(&rule.format);
                let cond_format =
                    rust_xlsxwriter::ConditionalFormatDuplicate::new().set_format(&fmt);
                worksheet
                    .add_conditional_format(r1, c1, r2, c2, &cond_format)
                    .map_err(|e| AppError::Write(e.to_string()))?;
            }
            ConditionalFormatType::DataBar => {
                let mut data_bar = rust_xlsxwriter::ConditionalFormatDataBar::new();

                if let Some(ConditionalFormatConfig::DataBar {
                    fill_color,
                    border_color,
                    direction,
                    ..
                }) = &rule.config
                {
                    if let Some(color) = fill_color {
                        data_bar = data_bar.set_fill_color(color.as_str());
                    }
                    if let Some(color) = border_color {
                        data_bar = data_bar.set_border_color(color.as_str());
                    }
                    if let Some(dir) = direction {
                        let d = match dir.to_lowercase().as_str() {
                            "left_to_right" | "left-to-right" => {
                                rust_xlsxwriter::ConditionalFormatDataBarDirection::LeftToRight
                            }
                            "right_to_left" | "right-to-left" => {
                                rust_xlsxwriter::ConditionalFormatDataBarDirection::RightToLeft
                            }
                            _ => rust_xlsxwriter::ConditionalFormatDataBarDirection::Context,
                        };
                        data_bar = data_bar.set_direction(d);
                    }
                }

                worksheet
                    .add_conditional_format(r1, c1, r2, c2, &data_bar)
                    .map_err(|e| AppError::Write(e.to_string()))?;
            }
            ConditionalFormatType::ColorScale => {
                if let Some(ConditionalFormatConfig::ColorScale {
                    variant,
                    min_color,
                    mid_color,
                    max_color,
                }) = &rule.config
                {
                    if variant == "3_color" {
                        let mut cs = rust_xlsxwriter::ConditionalFormat3ColorScale::new();
                        if let Some(c) = min_color {
                            cs = cs.set_minimum_color(c.as_str());
                        }
                        if let Some(c) = mid_color {
                            cs = cs.set_midpoint_color(c.as_str());
                        }
                        if let Some(c) = max_color {
                            cs = cs.set_maximum_color(c.as_str());
                        }
                        worksheet
                            .add_conditional_format(r1, c1, r2, c2, &cs)
                            .map_err(|e| AppError::Write(e.to_string()))?;
                    } else {
                        let mut cs = rust_xlsxwriter::ConditionalFormat2ColorScale::new();
                        if let Some(c) = min_color {
                            cs = cs.set_minimum_color(c.as_str());
                        }
                        if let Some(c) = max_color {
                            cs = cs.set_maximum_color(c.as_str());
                        }
                        worksheet
                            .add_conditional_format(r1, c1, r2, c2, &cs)
                            .map_err(|e| AppError::Write(e.to_string()))?;
                    }
                } else {
                    // Default: 2-color scale with red-to-green
                    let cs = rust_xlsxwriter::ConditionalFormat2ColorScale::new()
                        .set_minimum_color("FF6B6B")
                        .set_maximum_color("63BE7B");
                    worksheet
                        .add_conditional_format(r1, c1, r2, c2, &cs)
                        .map_err(|e| AppError::Write(e.to_string()))?;
                }
            }
            ConditionalFormatType::IconSet => {
                let mut icon_set = rust_xlsxwriter::ConditionalFormatIconSet::new();

                let icon_type = if let Some(ConditionalFormatConfig::IconSet {
                    icon_type, ..
                }) = &rule.config
                {
                    parse_icon_type(icon_type)
                } else {
                    rust_xlsxwriter::ConditionalFormatIconType::ThreeTrafficLights
                };

                icon_set = icon_set.set_icon_type(icon_type);

                if let Some(ConditionalFormatConfig::IconSet {
                    show_value,
                    reverse_order,
                    ..
                }) = &rule.config
                {
                    if let Some(true) = show_value {
                        icon_set = icon_set.show_icons_only(false);
                    } else if let Some(false) = show_value {
                        icon_set = icon_set.show_icons_only(true);
                    }
                    if let Some(true) = reverse_order {
                        icon_set = icon_set.reverse_icons(true);
                    }
                }

                worksheet
                    .add_conditional_format(r1, c1, r2, c2, &icon_set)
                    .map_err(|e| AppError::Write(e.to_string()))?;
            }
            ConditionalFormatType::Top10 => {
                let fmt = build_cf_format(&rule.format);
                let cond_format = rust_xlsxwriter::ConditionalFormatTop::new()
                    .set_rule(rust_xlsxwriter::ConditionalFormatTopRule::Top(
                        rule.condition.parse::<u16>().unwrap_or(10),
                    ))
                    .set_format(&fmt);
                worksheet
                    .add_conditional_format(r1, c1, r2, c2, &cond_format)
                    .map_err(|e| AppError::Write(e.to_string()))?;
            }
            ConditionalFormatType::AboveAverage => {
                let fmt = build_cf_format(&rule.format);
                let cond_format = rust_xlsxwriter::ConditionalFormatAverage::new()
                    .set_rule(rust_xlsxwriter::ConditionalFormatAverageRule::AboveAverage)
                    .set_format(&fmt);
                worksheet
                    .add_conditional_format(r1, c1, r2, c2, &cond_format)
                    .map_err(|e| AppError::Write(e.to_string()))?;
            }
            ConditionalFormatType::TextContains => {
                let fmt = build_cf_format(&rule.format);
                let cond_format = rust_xlsxwriter::ConditionalFormatText::new()
                    .set_rule(rust_xlsxwriter::ConditionalFormatTextRule::Contains(
                        rule.condition.clone(),
                    ))
                    .set_format(&fmt);
                worksheet
                    .add_conditional_format(r1, c1, r2, c2, &cond_format)
                    .map_err(|e| AppError::Write(e.to_string()))?;
            }
            ConditionalFormatType::DateOccurring => {
                let fmt = build_cf_format(&rule.format);
                let date_rule = parse_date_rule(&rule.condition);
                let cond_format = rust_xlsxwriter::ConditionalFormatDate::new()
                    .set_rule(date_rule)
                    .set_format(&fmt);
                worksheet
                    .add_conditional_format(r1, c1, r2, c2, &cond_format)
                    .map_err(|e| AppError::Write(e.to_string()))?;
            }
        }

        Ok(())
    })
}

/// Build a Format from optional Style for conditional formatting use.
fn build_cf_format(style: &Option<Style>) -> rust_xlsxwriter::Format {
    let mut fmt = rust_xlsxwriter::Format::default();
    if let Some(s) = style {
        if let Some(font_name) = &s.font_name {
            fmt = fmt.set_font_name(font_name);
        }
        if let Some(font_size) = s.font_size {
            fmt = fmt.set_font_size(font_size);
        }
        if let Some(true) = s.bold {
            fmt = fmt.set_bold();
        }
        if let Some(true) = s.italic {
            fmt = fmt.set_italic();
        }
        if let Some(font_color) = &s.font_color {
            fmt = fmt.set_font_color(font_color.as_str());
        }
        if let Some(bg_color) = &s.background_color {
            fmt = fmt.set_background_color(bg_color.as_str());
        }
        if let Some(nf) = &s.number_format {
            fmt = fmt.set_num_format(crate::utils::helpers::resolve_number_format(nf));
        }
    }
    fmt
}

pub fn remove_conditional_format(
    path: &str,
    _sheet: &str,
    range: &str,
    params: &SecurityParams,
) -> Result<WriteResult> {
    if params.dry_run {
        return Ok(WriteResult::dry_run_success());
    }

    security::create_backup_if_needed(params)?;

    let (_r1, _c1, _r2, _c2) = crate::utils::cell_ref::parse_range(range)?;

    crate::excel_write::modify_file_with_wb(path, params, |_, _wb| {
        // Conditional formats are cleared by simply not re-adding them
        // since we rewrite the workbook without the existing conditional formats.
        Ok(())
    })
}
