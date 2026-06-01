use std::collections::{HashMap, HashSet};

use excel_core::cell_ref;
use excel_types::SheetData;

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
