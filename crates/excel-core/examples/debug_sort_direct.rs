use excel_core::excel_write::core::sort;
use excel_core::types::{CellData, CellDataType, SheetData, SortColumn};

fn main() {
    // Create test data
    let mut data = std::collections::HashMap::new();

    let mut sheet = SheetData {
        name: "Sheet1".to_string(),
        rows: vec![
            vec![
                // Header
                CellData {
                    value: Some("Name".to_string()),
                    data_type: CellDataType::String,
                    formula: None,
                },
                CellData {
                    value: Some("Age".to_string()),
                    data_type: CellDataType::String,
                    formula: None,
                },
            ],
            vec![
                // Row 1
                CellData {
                    value: Some("Alice".to_string()),
                    data_type: CellDataType::String,
                    formula: None,
                },
                CellData {
                    value: Some("25".to_string()),
                    data_type: CellDataType::String,
                    formula: None,
                },
            ],
            vec![
                // Row 2
                CellData {
                    value: Some("Bob".to_string()),
                    data_type: CellDataType::String,
                    formula: None,
                },
                CellData {
                    value: Some("30".to_string()),
                    data_type: CellDataType::String,
                    formula: None,
                },
            ],
            vec![
                // Row 3
                CellData {
                    value: Some("Charlie".to_string()),
                    data_type: CellDataType::String,
                    formula: None,
                },
                CellData {
                    value: Some("35".to_string()),
                    data_type: CellDataType::String,
                    formula: None,
                },
            ],
        ],
    };

    println!("Before sort:");
    for (i, row) in sheet.rows.iter().enumerate() {
        let name = row
            .get(0)
            .and_then(|c| c.value.as_ref())
            .unwrap_or(&"None".to_string());
        let age = row
            .get(1)
            .and_then(|c| c.value.as_ref())
            .unwrap_or(&"None".to_string());
        println!("  Row {}: {} - {}", i, name, age);
    }

    // Sort by column 1 (Age) in descending order
    let sort_columns = vec![SortColumn {
        column: 1,
        descending: true,
    }];

    data.insert("Sheet1".to_string(), sheet);

    let result = sort(&mut data, "Sheet1", &sort_columns);
    println!("Sort result: {:?}", result);

    let sorted_sheet = data.get("Sheet1").unwrap();
    println!("After sort:");
    for (i, row) in sorted_sheet.rows.iter().enumerate() {
        let name = row
            .get(0)
            .and_then(|c| c.value.as_ref())
            .unwrap_or(&"None".to_string());
        let age = row
            .get(1)
            .and_then(|c| c.value.as_ref())
            .unwrap_or(&"None".to_string());
        println!("  Row {}: {} - {}", i, name, age);
    }

    // Check first data row
    if sorted_sheet.rows.len() > 1 {
        let first_age = sorted_sheet.rows[1][1].value.as_ref();
        println!("First age after header: {:?}", first_age);
    }
}
