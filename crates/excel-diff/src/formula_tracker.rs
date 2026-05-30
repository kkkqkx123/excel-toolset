use std::collections::{HashMap, HashSet};

use excel_core::cell_ref;
use excel_core::types::SheetData;

pub struct FormulaTracker {
    pub(crate) dependencies: HashMap<String, HashSet<String>>,
}

impl FormulaTracker {
    pub fn build_from_sheet(sheet: &SheetData) -> Self {
        let mut dependencies = HashMap::new();
        for (ri, row) in sheet.rows.iter().enumerate() {
            for (ci, cell) in row.iter().enumerate() {
                if let Some(formula) = &cell.formula {
                    let cell_ref = cell_ref::format_cell_ref(ri as u32, ci as u16);
                    let refs = extract_cell_refs(formula);
                    if !refs.is_empty() {
                        dependencies.insert(cell_ref, refs);
                    }
                }
            }
        }
        FormulaTracker { dependencies }
    }

    pub fn is_passive_change(
        &self,
        _cell_ref: &str,
        old_formula: Option<&str>,
        new_formula: Option<&str>,
    ) -> bool {
        match (old_formula, new_formula) {
            (Some(old_f), Some(new_f)) => old_f == new_f,
            _ => false,
        }
    }

    pub fn get_dependency_chain(&self, cell: &str) -> Option<String> {
        let visited = &mut HashSet::new();
        let chain = &mut Vec::new();
        self.build_chain(cell, visited, chain);
        if chain.len() <= 1 {
            return None;
        }
        Some(chain.join(" → "))
    }

    fn build_chain(&self, cell: &str, visited: &mut HashSet<String>, chain: &mut Vec<String>) {
        if !visited.insert(cell.to_string()) {
            return;
        }
        chain.push(cell.to_string());
        if let Some(deps) = self.dependencies.get(cell) {
            for dep in deps {
                self.build_chain(dep, visited, chain);
            }
        }
    }
}

fn extract_cell_refs(formula: &str) -> HashSet<String> {
    let body = formula.strip_prefix('=').unwrap_or(formula).trim();
    if body.is_empty() {
        return HashSet::new();
    }

    let mut refs = HashSet::new();
    let cleaned = strip_all_sheet_prefixes(body);

    // Find all potential cell references and ranges by scanning for patterns
    let chars: Vec<char> = cleaned.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        // Try to match a range (e.g., A1:B5) first
        if let Some((end, range_cells)) = try_match_range(&chars, i) {
            refs.extend(range_cells);
            i = end;
            continue;
        }

        // Try to match a single cell (e.g., A1)
        if let Some((end, cell)) = try_match_cell(&chars, i) {
            refs.insert(cell);
            i = end;
            continue;
        }

        i += 1;
    }

    refs
}

/// Strip `Sheet!` or `'Sheet'!` prefix from the formula body.
fn strip_all_sheet_prefixes(body: &str) -> String {
    let mut result = body.to_string();
    while let Some(pos) = result.find('!') {
        // Check if there's a quoted sheet name before '!'
        let before = &result[..pos];
        if before.ends_with('\'')
            && let Some(quote_start) = before.find('\'')
        {
            let before_quote = &before[..quote_start];
            if quote_start == 0
                || before_quote.ends_with(|c: char| !c.is_alphanumeric() && c != '\'')
            {
                result = format!("{}{}", &before[..quote_start], &result[pos + 1..]);
                continue;
            }
        }
        // Unquoted sheet name: find the start of the sheet name before '!'
        if pos >= 1 {
            let name_start = before
                .char_indices()
                .rev()
                .find(|(_, c)| !c.is_alphanumeric())
                .map(|(i, _)| i + 1)
                .unwrap_or(0);
            result = format!("{}{}", &result[..name_start], &result[pos + 1..]);
        }
    }
    result
}

fn try_match_range(chars: &[char], start: usize) -> Option<(usize, Vec<String>)> {
    let c1 = try_match_cell(chars, start)?;
    let (end1, cell1) = c1;

    // Must be followed by ':'
    if end1 >= chars.len() || chars[end1] != ':' {
        return None;
    }

    let c2 = try_match_cell(chars, end1 + 1)?;
    let (end2, cell2) = c2;

    let (r1, c1_idx) = cell_ref::parse_cell_ref(&cell1).ok()?;
    let (r2, c2_idx) = cell_ref::parse_cell_ref(&cell2).ok()?;

    let mut cells = Vec::new();
    for r in r1.min(r2)..=r1.max(r2) {
        for c in c1_idx.min(c2_idx)..=c1_idx.max(c2_idx) {
            cells.push(cell_ref::format_cell_ref(r, c));
        }
    }

    Some((end2, cells))
}

fn try_match_cell(chars: &[char], start: usize) -> Option<(usize, String)> {
    if start >= chars.len() {
        return None;
    }

    let mut pos = start;

    // Skip leading '$' for absolute references
    if pos < chars.len() && chars[pos] == '$' {
        pos += 1;
    }

    let mut col_part = String::new();
    let mut row_part = String::new();

    // Read column letters (A-Z, a-z)
    while pos < chars.len() && chars[pos].is_ascii_alphabetic() {
        col_part.push(chars[pos]);
        pos += 1;
    }

    if col_part.is_empty() {
        return None;
    }

    // If followed by '(', it's a function name, not a cell reference
    if pos < chars.len() && chars[pos] == '(' {
        return None;
    }

    // Skip '$' between column and row
    if pos < chars.len() && chars[pos] == '$' {
        pos += 1;
    }

    // Read row digits
    while pos < chars.len() && chars[pos].is_ascii_digit() {
        row_part.push(chars[pos]);
        pos += 1;
    }

    if row_part.is_empty() {
        return None;
    }

    Some((pos, format!("{}{}", col_part.to_uppercase(), row_part)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use excel_core::types::{CellData, CellDataType};

    #[test]
    fn test_extract_single_cell() {
        let refs = extract_cell_refs("=A1");
        assert_eq!(refs.len(), 1);
        assert!(refs.contains("A1"));
    }

    #[test]
    fn test_extract_sum_range() {
        let refs = extract_cell_refs("=SUM(A1:A3)");
        assert_eq!(refs.len(), 3);
        assert!(refs.contains("A1"));
        assert!(refs.contains("A2"));
        assert!(refs.contains("A3"));
    }

    #[test]
    fn test_extract_simple_arithmetic() {
        let refs = extract_cell_refs("=A1+B2");
        assert_eq!(refs.len(), 2);
        assert!(refs.contains("A1"));
        assert!(refs.contains("B2"));
    }

    #[test]
    fn test_extract_mixed() {
        let refs = extract_cell_refs("=SUM(A1:A3)*B5");
        assert_eq!(refs.len(), 4);
        assert!(refs.contains("A1"));
        assert!(refs.contains("A2"));
        assert!(refs.contains("A3"));
        assert!(refs.contains("B5"));
    }

    #[test]
    fn test_formula_no_refs() {
        let refs = extract_cell_refs("=\"hello\"");
        assert!(refs.is_empty());
    }

    #[test]
    fn test_absolute_ref() {
        let refs = extract_cell_refs("=$A$1+$B$2");
        assert_eq!(refs.len(), 2);
        assert!(refs.contains("A1"));
        assert!(refs.contains("B2"));
    }

    #[test]
    fn test_passive_identical_formula() {
        let tracker = FormulaTracker {
            dependencies: HashMap::new(),
        };
        assert!(tracker.is_passive_change("A5", Some("=SUM(A1:A3)"), Some("=SUM(A1:A3)")));
        assert!(!tracker.is_passive_change("A5", Some("=SUM(A1:A3)"), Some("=AVERAGE(A1:A3)")));
        assert!(!tracker.is_passive_change("A5", Some("=SUM(A1:A3)"), None));
        assert!(!tracker.is_passive_change("A5", None, Some("=SUM(A1:A3)")));
    }

    #[test]
    fn test_dependency_chain() {
        let mut deps = HashMap::new();
        deps.insert("C5".into(), ["A2".into(), "A3".into(), "A4".into()].into());
        deps.insert("D5".into(), ["C5".into()].into());
        let tracker = FormulaTracker { dependencies: deps };

        let chain = tracker.get_dependency_chain("D5");
        assert!(chain.is_some());
        let chain_str = chain.unwrap();
        assert!(chain_str.contains("D5"));
        assert!(chain_str.contains("C5"));
        assert!(chain_str.contains("A2"));
    }

    #[test]
    fn test_empty_formula_returns_no_refs() {
        let refs = extract_cell_refs("");
        assert!(refs.is_empty());
    }

    #[test]
    fn test_formula_with_only_equals() {
        let refs = extract_cell_refs("=");
        assert!(refs.is_empty());
    }

    #[test]
    fn test_extract_range_backwards() {
        let refs = extract_cell_refs("=SUM(C3:A1)");
        assert_eq!(refs.len(), 9);
        assert!(refs.contains("A1"));
        assert!(refs.contains("C3"));
    }

    #[test]
    fn test_extract_sheet_prefixed_refs() {
        let refs = extract_cell_refs("=Sheet2!A1+Sheet2!B2");
        assert_eq!(refs.len(), 2);
        assert!(refs.contains("A1"));
        assert!(refs.contains("B2"));
    }

    #[test]
    fn test_extract_quoted_sheet_prefixed_refs() {
        let refs = extract_cell_refs("='Sheet 2'!A1");
        assert_eq!(refs.len(), 1);
        assert!(refs.contains("A1"));
    }

    #[test]
    fn test_extract_range_with_sheet_prefix() {
        let refs = extract_cell_refs("=SUM(Sheet2!A1:B2)");
        assert_eq!(refs.len(), 4);
        assert!(refs.contains("A1"));
        assert!(refs.contains("B2"));
    }

    #[test]
    fn test_build_from_sheet_with_formulas() {
        let sheet = SheetData {
            name: "S".into(),
            rows: vec![
                vec![
                    CellData {
                        value: Some("10".into()),
                        data_type: CellDataType::Float,
                        formula: Some("=A2+1".into()),
                    },
                    CellData {
                        value: Some("20".into()),
                        data_type: CellDataType::Float,
                        formula: Some("=B2+1".into()),
                    },
                ],
                vec![
                    CellData {
                        value: Some("1".into()),
                        data_type: CellDataType::Float,
                        formula: None,
                    },
                    CellData {
                        value: Some("2".into()),
                        data_type: CellDataType::Float,
                        formula: None,
                    },
                ],
            ],
        };
        let tracker = FormulaTracker::build_from_sheet(&sheet);
        assert!(tracker.dependencies.contains_key("A1"));
        assert!(tracker.dependencies.contains_key("B1"));
        assert_eq!(tracker.dependencies.len(), 2);
    }

    #[test]
    fn test_build_from_sheet_no_formulas() {
        let sheet = SheetData {
            name: "S".into(),
            rows: vec![vec![CellData {
                value: Some("1".into()),
                data_type: CellDataType::Float,
                formula: None,
            }]],
        };
        let tracker = FormulaTracker::build_from_sheet(&sheet);
        assert!(tracker.dependencies.is_empty());
    }

    #[test]
    fn test_no_dependency_chain_for_no_deps() {
        let tracker = FormulaTracker {
            dependencies: HashMap::new(),
        };
        assert!(tracker.get_dependency_chain("A1").is_none());
    }

    #[test]
    fn test_dependency_chain_self_reference_does_not_loop() {
        let mut deps = HashMap::new();
        deps.insert("A1".into(), ["A1".into()].into());
        let tracker = FormulaTracker { dependencies: deps };

        let chain = tracker.get_dependency_chain("A1");
        assert!(chain.is_none());
    }

    #[test]
    fn test_strip_all_sheet_prefixes_no_prefix() {
        let result = strip_all_sheet_prefixes("A1+B2");
        assert_eq!(result, "A1+B2");
    }

    #[test]
    fn test_strip_all_sheet_prefixes_simple() {
        let result = strip_all_sheet_prefixes("Sheet1!A1");
        assert_eq!(result, "A1");
    }

    #[test]
    fn test_strip_all_sheet_prefixes_quoted() {
        let result = strip_all_sheet_prefixes("'My Sheet'!A1");
        assert_eq!(result, "A1");
    }

    #[test]
    fn test_try_match_cell_function_call_false_positive() {
        let chars: Vec<char> = "SUM(".chars().collect();
        let result = try_match_cell(&chars, 0);
        assert!(result.is_none());
    }

    #[test]
    fn test_try_match_cell_out_of_bounds() {
        let chars = vec![];
        let result = try_match_cell(&chars, 0);
        assert!(result.is_none());
    }
}
