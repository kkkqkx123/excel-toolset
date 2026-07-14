use std::collections::{HashMap, HashSet};

use excel_types::SheetData;

#[derive(Debug, Clone, Default)]
pub struct FormulaTracker {
    pub dependencies: HashMap<String, HashSet<String>>,
}

impl FormulaTracker {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn build_from_sheet(sheet: &SheetData) -> Self {
        let mut tracker = Self::new();

        for (row_idx, row) in sheet.rows.iter().enumerate() {
            for (col_idx, cell) in row.iter().enumerate() {
                if let Some(formula) = &cell.formula {
                    let cell_ref = crate::helpers::format_cell_ref(row_idx, col_idx);
                    let refs = extract_cell_refs(formula);
                    if !refs.is_empty() {
                        tracker.dependencies.insert(cell_ref, refs);
                    }
                }
            }
        }

        tracker
    }

    pub fn is_passive_change(
        &self,
        _cell_ref: &str,
        old_formula: Option<&str>,
        new_formula: Option<&str>,
    ) -> bool {
        match (old_formula, new_formula) {
            (Some(of), Some(nf)) => of == nf,
            _ => false,
        }
    }

    /// Detect cycles in the dependency graph. Returns a list of cell references
    /// that participate in cycles (one entry per cycle-starting node).
    pub fn detect_cycles(&self) -> Vec<String> {
        let mut cycles = Vec::new();
        let mut global_visited = HashSet::new();

        for node in self.dependencies.keys() {
            if global_visited.contains(node.as_str()) {
                continue;
            }
            let mut path = Vec::new();
            let mut path_set = HashSet::new();
            if self.detect_cycles_dfs(node, &mut path, &mut path_set, &mut global_visited) {
                cycles.push(node.clone());
            }
        }

        cycles
    }

    fn detect_cycles_dfs(
        &self,
        node: &str,
        path: &mut Vec<String>,
        path_set: &mut HashSet<String>,
        global_visited: &mut HashSet<String>,
    ) -> bool {
        if path_set.contains(node) {
            return true;
        }
        if global_visited.contains(node) {
            return false;
        }

        global_visited.insert(node.to_string());
        path.push(node.to_string());
        path_set.insert(node.to_string());

        if let Some(deps) = self.dependencies.get(node) {
            for dep in deps {
                if self.detect_cycles_dfs(dep, path, path_set, global_visited) {
                    return true;
                }
            }
        }

        path_set.remove(node);
        path.pop();

        false
    }

    pub fn get_dependency_chain(&self, cell_ref: &str) -> Option<String> {
        if !self.dependencies.contains_key(cell_ref) {
            return None;
        }
        let mut chain = Vec::new();
        let mut visited = HashSet::new();

        if self.build_chain_recursive(cell_ref, &mut chain, &mut visited) {
            Some(chain.join(" -> "))
        } else {
            None
        }
    }

    fn build_chain_recursive(
        &self,
        cell_ref: &str,
        chain: &mut Vec<String>,
        visited: &mut HashSet<String>,
    ) -> bool {
        if visited.contains(cell_ref) {
            return false;
        }

        visited.insert(cell_ref.to_string());
        chain.push(cell_ref.to_string());

        if let Some(deps) = self.dependencies.get(cell_ref) {
            for dep in deps {
                if !self.build_chain_recursive(dep, chain, visited) {
                    return false;
                }
            }
        }

        true
    }
}

pub fn extract_cell_refs(formula: &str) -> HashSet<String> {
    let mut refs = HashSet::new();
    let formula = strip_all_sheet_prefixes(formula);

    if !formula.starts_with('=') {
        return refs;
    }

    let formula = &formula[1..];
    let chars: Vec<char> = formula.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        if let Some(range) = try_match_range(&chars, i) {
            for cell in expand_range(&range) {
                refs.insert(cell);
            }
            i += range.len();
        } else if let Some((cell_ref, match_len)) = try_match_cell_with_len(&chars, i) {
            refs.insert(cell_ref);
            i += match_len;
        } else {
            i += 1;
        }
    }

    refs
}

pub fn strip_all_sheet_prefixes(formula: &str) -> String {
    let mut result = String::new();
    let mut i = 0;
    let chars: Vec<char> = formula.chars().collect();

    while i < chars.len() {
        if chars[i] == '\'' {
            let end = match chars[i + 1..].iter().position(|&c| c == '\'') {
                Some(pos) => i + 1 + pos,
                None => {
                    result.push(chars[i]);
                    i += 1;
                    continue;
                }
            };
            if end + 1 < chars.len() && chars[end + 1] == '!' {
                i = end + 2;
            } else {
                result.push(chars[i]);
                i += 1;
            }
        } else {
            let mut j = i;
            while j < chars.len() && chars[j] != '!' && chars[j] != '\'' {
                j += 1;
            }

            if j < chars.len() && chars[j] == '!' {
                let mut name_start = j;
                while name_start > i && chars[name_start - 1].is_alphanumeric() {
                    name_start -= 1;
                }
                if name_start < j {
                    while i < name_start {
                        result.push(chars[i]);
                        i += 1;
                    }
                    i = j + 1;
                } else {
                    while i <= j {
                        result.push(chars[i]);
                        i += 1;
                    }
                }
            } else {
                while i < j {
                    result.push(chars[i]);
                    i += 1;
                }
            }
        }
    }

    result
}

pub fn try_match_cell(chars: &[char], pos: usize) -> Option<String> {
    try_match_cell_with_len(chars, pos).map(|(s, _)| s)
}

fn try_match_cell_with_len(chars: &[char], pos: usize) -> Option<(String, usize)> {
    if pos >= chars.len() {
        return None;
    }

    let start = pos;
    let mut i = pos;

    let _has_dollar_prefix = if i < chars.len() && chars[i] == '$' {
        i += 1;
        true
    } else {
        false
    };

    while i < chars.len() && chars[i].is_ascii_alphabetic() {
        i += 1;
    }

    let _has_dollar_middle = if i < chars.len() && chars[i] == '$' {
        i += 1;
        true
    } else {
        false
    };

    while i < chars.len() && chars[i].is_ascii_digit() {
        i += 1;
    }

    let _has_dollar_suffix = if i < chars.len() && chars[i] == '$' {
        i += 1;
        true
    } else {
        false
    };

    let match_len = i - start;

    if i > start {
        let has_letters = chars[start..i].iter().any(|c| c.is_ascii_alphabetic());
        let has_digits = chars[start..i].iter().any(|c| c.is_ascii_digit());

        if has_letters && has_digits {
            let result: String = chars[start..i].iter().collect();
            Some((result.replace('$', ""), match_len))
        } else {
            None
        }
    } else {
        None
    }
}

fn try_match_range(chars: &[char], pos: usize) -> Option<String> {
    if pos >= chars.len() {
        return None;
    }

    let (start_cell, start_len) = try_match_cell_with_len(chars, pos)?;

    let mut i = pos + start_len;

    if i >= chars.len() || chars[i] != ':' {
        return None;
    }

    i += 1;

    let (end_cell, _end_len) = try_match_cell_with_len(chars, i)?;

    Some(format!("{}:{}", start_cell, end_cell))
}

fn expand_range(range: &str) -> Vec<String> {
    let parts: Vec<&str> = range.split(':').collect();
    if parts.len() != 2 {
        return Vec::new();
    }

    let start = parts[0];
    let end = parts[1];

    let (start_row, start_col) = match parse_cell_ref(start) {
        Some(coords) => coords,
        None => return Vec::new(),
    };

    let (end_row, end_col) = match parse_cell_ref(end) {
        Some(coords) => coords,
        None => return Vec::new(),
    };

    let mut cells = Vec::new();

    for row in start_row.min(end_row)..=start_row.max(end_row) {
        for col in start_col.min(end_col)..=start_col.max(end_col) {
            let col_name = crate::helpers::index_to_col(col);
            let row_num = row + 1;
            cells.push(format!("{}{}", col_name, row_num));
        }
    }

    cells
}

fn parse_cell_ref(ref_str: &str) -> Option<(usize, usize)> {
    let chars: Vec<char> = ref_str.chars().collect();
    let mut col_end = 0;

    while col_end < chars.len() && chars[col_end].is_ascii_alphabetic() {
        col_end += 1;
    }

    if col_end == 0 || col_end >= chars.len() {
        return None;
    }

    let col_str: String = chars[..col_end].iter().collect();
    let col = col_str_to_index(&col_str)?;

    let row_str: String = chars[col_end..].iter().collect();
    let row: usize = row_str.parse().ok()?;

    Some((row - 1, col))
}

fn col_str_to_index(col_str: &str) -> Option<usize> {
    let mut index = 0;

    for c in col_str.chars() {
        if !c.is_ascii_alphabetic() {
            return None;
        }
        index = index * 26 + (c.to_ascii_uppercase() as usize - 'A' as usize + 1);
    }

    Some(index - 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use excel_types::{CellData, CellDataType};

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
    fn test_dependency_chain_multi_cell_cycle_returns_none() {
        let mut deps = HashMap::new();
        deps.insert("A1".into(), ["B1".into()].into());
        deps.insert("B1".into(), ["A1".into()].into());
        let tracker = FormulaTracker { dependencies: deps };

        let chain = tracker.get_dependency_chain("A1");
        assert!(
            chain.is_none(),
            "multi-cell cycle should return None: {:?}",
            chain
        );
    }

    #[test]
    fn test_dependency_chain_deep_chain_succeeds() {
        let mut deps = HashMap::new();
        deps.insert("C5".into(), ["C4".into()].into());
        deps.insert("C4".into(), ["C3".into()].into());
        deps.insert("C3".into(), ["C2".into()].into());
        let tracker = FormulaTracker { dependencies: deps };

        let chain = tracker.get_dependency_chain("C5");
        assert!(chain.is_some());
        let chain_str = chain.unwrap();
        assert!(chain_str.contains("C5"));
        assert!(chain_str.contains("C4"));
        assert!(chain_str.contains("C3"));
        assert!(chain_str.contains("C2"));
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
