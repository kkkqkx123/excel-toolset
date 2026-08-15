use excel_core::features::search;
use excel_core::types::*;
use crate::cli::args::*;

pub(crate) fn run_search(args: &SearchArgs) -> Result<serde_json::Value> {
    match &args.command {
        SearchSub::Workbook {
            path,
            pattern,
            match_type,
            search_type,
            case_sensitive,
            sheets,
        } => {
            let query =
                build_search_query(pattern, match_type, search_type, *case_sensitive, sheets)?;
            let results = search::search_workbook(path, &query)?;
            Ok(serde_json::to_value(results).map_err(|e| AppError::Serialize(e.to_string()))?)
        }
        SearchSub::Sheet {
            path,
            sheet,
            pattern,
            match_type,
            search_type,
            case_sensitive,
        } => {
            let query =
                build_search_query(pattern, match_type, search_type, *case_sensitive, &None)?;
            let results = search::search_sheet(path, sheet, &query)?;
            Ok(serde_json::to_value(results).map_err(|e| AppError::Serialize(e.to_string()))?)
        }
    }
}


fn build_search_query(
    pattern: &str,
    match_type: &str,
    search_type: &str,
    case_sensitive: bool,
    sheets: &Option<Vec<String>>,
) -> Result<search::SearchQuery> {
    let st = match search_type {
        "value" => search::SearchType::Value,
        "formula" => search::SearchType::Formula,
        _ => search::SearchType::Both,
    };
    let mt = match match_type {
        "exact" => search::MatchType::Exact,
        "regex" => search::MatchType::Regex,
        _ => search::MatchType::Contains,
    };
    Ok(search::SearchQuery {
        pattern: pattern.to_string(),
        search_type: st,
        match_type: mt,
        case_sensitive,
        sheets: sheets.clone(),
    })
}

// ── Conditional Format ──

