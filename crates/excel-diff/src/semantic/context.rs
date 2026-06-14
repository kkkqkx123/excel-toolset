use std::collections::HashMap;

use excel_types::SheetData;

#[derive(Debug, Clone)]
pub struct HeaderContext {
    headers: HashMap<String, Vec<String>>,
}

impl HeaderContext {
    pub fn new(headers: HashMap<String, Vec<String>>) -> Self {
        HeaderContext { headers }
    }

    pub fn get_header(&self, sheet_name: &str, col: usize) -> Option<&str> {
        self.headers
            .get(sheet_name)
            .and_then(|headers| headers.get(col))
            .map(|h| h.as_str())
            .filter(|h| !h.is_empty())
    }

    pub fn from_sheet_maps(
        old: &HashMap<String, SheetData>,
        new: &HashMap<String, SheetData>,
    ) -> Self {
        let mut headers = HashMap::new();

        for sheet_name in old.keys().chain(new.keys()) {
            if let Some(new_sheet) = new.get(sheet_name) {
                let sheet_headers = extract_headers(new_sheet);
                if !sheet_headers.is_empty() {
                    headers.insert(sheet_name.clone(), sheet_headers);
                    continue;
                }
            }

            if let Some(old_sheet) = old.get(sheet_name) {
                let sheet_headers = extract_headers(old_sheet);
                if !sheet_headers.is_empty() {
                    headers.insert(sheet_name.clone(), sheet_headers);
                }
            }
        }

        HeaderContext { headers }
    }
}

pub fn extract_headers(sheet: &SheetData) -> Vec<String> {
    sheet
        .rows
        .first()
        .map(|row| {
            row.iter()
                .filter_map(|cell| cell.value.as_ref().filter(|v| !v.is_empty()).cloned())
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use excel_types::{CellData, CellDataType};

    #[test]
    fn test_get_header_found() {
        let mut headers = HashMap::new();
        headers.insert("Sheet1".into(), vec!["Name".into(), "Value".into()]);
        let ctx = HeaderContext::new(headers);
        assert_eq!(ctx.get_header("Sheet1", 0), Some("Name"));
        assert_eq!(ctx.get_header("Sheet1", 1), Some("Value"));
    }

    #[test]
    fn test_get_header_missing_sheet() {
        let ctx = HeaderContext::new(HashMap::new());
        assert!(ctx.get_header("Nonexistent", 0).is_none());
    }

    #[test]
    fn test_get_header_out_of_range() {
        let mut headers = HashMap::new();
        headers.insert("S".into(), vec!["A".into()]);
        let ctx = HeaderContext::new(headers);
        assert!(ctx.get_header("S", 5).is_none());
    }

    #[test]
    fn test_get_header_empty_string_returns_none() {
        let mut headers = HashMap::new();
        headers.insert("S".into(), vec!["".into()]);
        let ctx = HeaderContext::new(headers);
        assert!(ctx.get_header("S", 0).is_none());
    }

    #[test]
    fn test_extract_headers_from_sheet() {
        let sheet = SheetData {
            name: "S".into(),
            rows: vec![
                vec![
                    CellData {
                        value: Some("Name".into()),
                        data_type: CellDataType::String,
                        formula: None,
                    },
                    CellData {
                        value: Some("Score".into()),
                        data_type: CellDataType::String,
                        formula: None,
                    },
                ],
                vec![CellData {
                    value: Some("Alice".into()),
                    data_type: CellDataType::String,
                    formula: None,
                }],
            ],
        };
        let headers = extract_headers(&sheet);
        assert_eq!(headers, vec!["Name", "Score"]);
    }

    #[test]
    fn test_extract_headers_empty_sheet() {
        let sheet = SheetData {
            name: "S".into(),
            rows: vec![],
        };
        assert!(extract_headers(&sheet).is_empty());
    }

    #[test]
    fn test_from_sheet_maps_uses_new_preferred() {
        fn make_sheet(name: &str, headers: Vec<&str>) -> SheetData {
            SheetData {
                name: name.into(),
                rows: vec![
                    headers
                        .iter()
                        .map(|h| CellData {
                            value: Some(h.to_string()),
                            data_type: CellDataType::String,
                            formula: None,
                        })
                        .collect(),
                ],
            }
        }

        let mut old = HashMap::new();
        old.insert("S1".into(), make_sheet("S1", vec!["OldA", "OldB"]));
        let mut new = HashMap::new();
        new.insert("S1".into(), make_sheet("S1", vec!["NewA", "NewB"]));

        let ctx = HeaderContext::from_sheet_maps(&old, &new);
        assert_eq!(ctx.get_header("S1", 0), Some("NewA"));
        assert_eq!(ctx.get_header("S1", 1), Some("NewB"));
    }

    #[test]
    fn test_from_sheet_maps_falls_back_to_old() {
        fn make_sheet(name: &str, headers: Vec<&str>) -> SheetData {
            SheetData {
                name: name.into(),
                rows: vec![
                    headers
                        .iter()
                        .map(|h| CellData {
                            value: Some(h.to_string()),
                            data_type: CellDataType::String,
                            formula: None,
                        })
                        .collect(),
                ],
            }
        }

        let mut old = HashMap::new();
        old.insert("S1".into(), make_sheet("S1", vec!["OldA"]));
        let new = HashMap::new();

        let ctx = HeaderContext::from_sheet_maps(&old, &new);
        assert_eq!(ctx.get_header("S1", 0), Some("OldA"));
    }

    #[test]
    fn test_from_sheet_maps_combined_keys() {
        fn make_sheet(name: &str, headers: Vec<&str>) -> SheetData {
            SheetData {
                name: name.into(),
                rows: vec![
                    headers
                        .iter()
                        .map(|h| CellData {
                            value: Some(h.to_string()),
                            data_type: CellDataType::String,
                            formula: None,
                        })
                        .collect(),
                ],
            }
        }

        let mut old = HashMap::new();
        old.insert("S1".into(), make_sheet("S1", vec!["A"]));
        let mut new = HashMap::new();
        new.insert("S2".into(), make_sheet("S2", vec!["B"]));

        let ctx = HeaderContext::from_sheet_maps(&old, &new);
        assert_eq!(ctx.get_header("S1", 0), Some("A"));
        assert_eq!(ctx.get_header("S2", 0), Some("B"));
    }
}
