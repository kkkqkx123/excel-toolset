//! Page setup configuration types for print settings.
//!
//! Provides types for configuring worksheet print properties:
//! page orientation, paper size, margins, print area, print titles,
//! scaling, gridlines, headings, centering, and page breaks.

use serde::{Deserialize, Serialize};

/// Page setup configuration for a worksheet.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PageSetupConfig {
    /// Target sheet name.
    pub sheet: String,
    /// Paper size (A4, A3, Letter, Legal, etc.).
    pub paper_size: Option<PaperSize>,
    /// Page orientation.
    pub orientation: Option<PageOrientation>,
    /// Page margins in inches.
    pub margins: Option<PageMargins>,
    /// Print area range, e.g. "A1:G50".
    pub print_area: Option<String>,
    /// Print title rows (rows repeated at top of each page), e.g. "1:3".
    pub print_title_rows: Option<String>,
    /// Print title columns (columns repeated at left of each page), e.g. "A:B".
    pub print_title_cols: Option<String>,
    /// Fit to pages: (width_pages, height_pages).
    pub fit_to_pages: Option<FitToPages>,
    /// Print scale percentage (100 = 100%).
    pub scale: Option<u16>,
    /// Print gridlines.
    pub print_gridlines: bool,
    /// Print row and column headings.
    pub print_headings: bool,
    /// Center horizontally on page.
    pub center_horizontally: bool,
    /// Center vertically on page.
    pub center_vertically: bool,
}

impl Default for PageSetupConfig {
    fn default() -> Self {
        Self {
            sheet: String::new(),
            paper_size: None,
            orientation: None,
            margins: None,
            print_area: None,
            print_title_rows: None,
            print_title_cols: None,
            fit_to_pages: None,
            scale: None,
            print_gridlines: false,
            print_headings: false,
            center_horizontally: false,
            center_vertically: false,
        }
    }
}

/// Paper size enumeration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PaperSize {
    /// A4 (210mm x 297mm) — default paper size for most regions.
    A4,
    /// A3 (297mm x 420mm).
    A3,
    /// US Letter (8.5" x 11").
    Letter,
    /// US Legal (8.5" x 14").
    Legal,
    /// US Executive (7.25" x 10.5").
    Executive,
    /// A5 (148mm x 210mm).
    A5,
    /// B4 (250mm x 353mm).
    B4,
    /// B5 (176mm x 250mm).
    B5,
}

impl PaperSize {
    /// Convert to rust_xlsxwriter paper size index.
    pub fn to_xlsx_index(&self) -> u8 {
        match self {
            PaperSize::A4 => 9,
            PaperSize::A3 => 8,
            PaperSize::Letter => 1,
            PaperSize::Legal => 5,
            PaperSize::Executive => 7,
            PaperSize::A5 => 11,
            PaperSize::B4 => 12,
            PaperSize::B5 => 13,
        }
    }
}

/// Page orientation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PageOrientation {
    /// Portrait orientation (taller than wide).
    Portrait,
    /// Landscape orientation (wider than tall).
    Landscape,
}

/// Page margins in inches.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageMargins {
    /// Left margin in inches.
    pub left: f64,
    /// Right margin in inches.
    pub right: f64,
    /// Top margin in inches.
    pub top: f64,
    /// Bottom margin in inches.
    pub bottom: f64,
    /// Header margin in inches.
    #[serde(default)]
    pub header: f64,
    /// Footer margin in inches.
    #[serde(default)]
    pub footer: f64,
}

impl Default for PageMargins {
    fn default() -> Self {
        Self {
            left: 0.7,
            right: 0.7,
            top: 0.75,
            bottom: 0.75,
            header: 0.3,
            footer: 0.3,
        }
    }
}

/// Fit-to-pages scaling configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FitToPages {
    /// Number of pages wide to fit.
    pub width: u16,
    /// Number of pages tall to fit.
    pub height: u16,
}

/// Page break configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageBreakConfig {
    /// Target sheet name.
    pub sheet: String,
    /// Row indices for horizontal page breaks (0-indexed).
    #[serde(default)]
    pub horizontal_breaks: Vec<u32>,
    /// Column indices for vertical page breaks (0-indexed).
    #[serde(default)]
    pub vertical_breaks: Vec<u16>,
}
