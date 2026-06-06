use calamine::Reader;
use regex::Regex;
use serde::Serialize;

use crate::excel_read;
use crate::types::*;

#[derive(Debug, Clone, Serialize)]
pub enum SearchType {
    Value,
    Formula,
    Both,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum MatchType {
    Exact,
    Contains,
    Regex,
}

#[derive(Debug, Clone, Serialize)]
pub struct SearchQuery {
    pub pattern: String,
    pub search_type: SearchType,
    pub match_type: MatchType,
    pub case_sensitive: bool,
    pub sheets: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SearchMatch {
    pub sheet: String,
    pub cell: String,
    pub value: Option<String>,
    pub formula: Option<String>,
    pub context: Option<Vec<Vec<String>>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SearchResults {
    pub query: String,
    pub matches: Vec<SearchMatch>,
    pub total_matches: usize,
}

pub fn search_workbook(path: &str, query: &SearchQuery) -> Result<SearchResults> {
    let all_sheets = excel_read::list_sheets(path)?;
    let target_sheets = if let Some(ref sheets) = query.sheets {
        sheets.clone()
    } else {
        all_sheets
    };

    let mut all_matches = Vec::new();

    for sheet in target_sheets {
        let sheet_matches = search_sheet(path, &sheet, query)?;
        all_matches.extend(sheet_matches.matches);
    }

    Ok(SearchResults {
        query: query.pattern.clone(),
        matches: all_matches.clone(),
        total_matches: all_matches.len(),
    })
}

pub fn search_sheet(path: &str, sheet: &str, query: &SearchQuery) -> Result<SearchResults> {
    let mut matches = Vec::new();

    let mut workbook = calamine::open_workbook::<calamine::Xlsx<_>, _>(path)
        .map_err(|e| AppError::Read(e.to_string()))?;

    let range = workbook
        .worksheet_range(sheet)
        .map_err(|_e| AppError::SheetNotFound(sheet.to_string()))?;

    let ws_formulas = workbook.worksheet_formula(sheet).ok();

    let regex_compiled = if query.match_type == MatchType::Regex {
        Some(compile_regex(&query.pattern, query.case_sensitive)?)
    } else {
        None
    };

    for row in 0..range.height() {
        for col in 0..range.width() {
            let cell_value = range.get_value((row as u32, col as u32));
            let cell_formula = ws_formulas
                .as_ref()
                .and_then(|f| f.get_value((row as u32, col as u32)).map(|s| s.to_string()));

            let should_search_value =
                matches!(&query.search_type, SearchType::Value | SearchType::Both);
            let should_search_formula =
                matches!(&query.search_type, SearchType::Formula | SearchType::Both);

            let mut value_match = None;
            let mut formula_match = None;

            if should_search_value && let Some(value) = cell_value {
                let value_str = value_to_string(value);
                if check_match(&value_str, query, regex_compiled.as_ref()) {
                    value_match = Some(value_str);
                }
            }

            if should_search_formula
                && let Some(ref formula) = cell_formula
                && check_match(formula, query, regex_compiled.as_ref())
            {
                formula_match = Some(formula.clone());
            }

            if value_match.is_some() || formula_match.is_some() {
                let context = get_context(&range, row, col);

                let cell_addr = crate::utils::cell_ref::format_cell_ref(row as u32, col as u16);

                matches.push(SearchMatch {
                    sheet: sheet.to_string(),
                    cell: cell_addr,
                    value: value_match,
                    formula: formula_match,
                    context,
                });
            }
        }
    }

    let total_matches = matches.len();
    Ok(SearchResults {
        query: query.pattern.clone(),
        matches,
        total_matches,
    })
}

fn compile_regex(pattern: &str, case_sensitive: bool) -> Result<regex::Regex> {
    let flags = if case_sensitive { "" } else { "(?i)" };
    let full_pattern = format!("{}{}", flags, pattern);
    Regex::new(&full_pattern).map_err(|e| AppError::InvalidInput(format!("Invalid regex: {}", e)))
}

fn check_match(text: &str, query: &SearchQuery, regex_compiled: Option<&Regex>) -> bool {
    let compare_text = if query.case_sensitive {
        text
    } else {
        &text.to_lowercase()
    };

    let compare_pattern = if query.case_sensitive {
        query.pattern.as_str()
    } else {
        &query.pattern.to_lowercase()
    };

    match query.match_type {
        MatchType::Exact => compare_text == compare_pattern,
        MatchType::Contains => compare_text.contains(compare_pattern),
        MatchType::Regex => {
            if let Some(regex) = regex_compiled {
                regex.is_match(text)
            } else {
                false
            }
        }
    }
}

fn value_to_string(value: &calamine::Data) -> String {
    match value {
        calamine::Data::Empty => String::new(),
        calamine::Data::String(s) => s.clone(),
        calamine::Data::Float(f) => f.to_string(),
        calamine::Data::Int(i) => i.to_string(),
        calamine::Data::Bool(b) => b.to_string(),
        calamine::Data::DateTime(dt) => dt.to_string(),
        calamine::Data::Error(e) => format!("{}", e),
        _ => String::new(),
    }
}

fn get_context(
    range: &calamine::Range<calamine::Data>,
    center_row: usize,
    center_col: usize,
) -> Option<Vec<Vec<String>>> {
    let context_size = 2;
    let start_row = center_row.saturating_sub(context_size);
    let end_row = (center_row + context_size + 1).min(range.height());
    let start_col = center_col.saturating_sub(context_size);
    let end_col = (center_col + context_size + 1).min(range.width());

    let mut context = Vec::new();

    for row in start_row..end_row {
        let mut row_data = Vec::new();
        for col in start_col..end_col {
            let cell_value = range.get_value((row as u32, col as u32));
            let cell_str = cell_value.map(value_to_string).unwrap_or_default();
            row_data.push(cell_str);
        }
        context.push(row_data);
    }

    Some(context)
}

impl Default for SearchQuery {
    fn default() -> Self {
        SearchQuery {
            pattern: String::new(),
            search_type: SearchType::Both,
            match_type: MatchType::Contains,
            case_sensitive: false,
            sheets: None,
        }
    }
}
