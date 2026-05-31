use rust_xlsxwriter::ChartType as XlsxChartType;

use crate::types::*;

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
