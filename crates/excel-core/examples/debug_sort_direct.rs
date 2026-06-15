use excel_core::types::{CellData, CellDataType, SheetData, SortColumn};
use std::collections::HashMap;

fn sort_data(
    data: &mut HashMap<String, SheetData>,
    sheet: &str,
    columns: &[SortColumn],
) -> Result<(), String> {
    let sd = data
        .get_mut(sheet)
        .ok_or_else(|| format!("Sheet '{}' not found", sheet))?;
    if sd.rows.len() > 1 {
        let header = sd.rows[0].clone();
        let mut body: Vec<Vec<CellData>> = sd.rows.drain(1..).collect();

        body.sort_by(|a, b| {
            for sc in columns {
                let ca = a
                    .get(sc.column as usize)
                    .and_then(|c| c.value.as_deref())
                    .unwrap_or("");
                let cb = b
                    .get(sc.column as usize)
                    .and_then(|c| c.value.as_deref())
                    .unwrap_or("");

                if let (Ok(na), Ok(nb)) = (ca.parse::<f64>(), cb.parse::<f64>()) {
                    let cmp = na.partial_cmp(&nb).unwrap_or(std::cmp::Ordering::Equal);
                    if cmp != std::cmp::Ordering::Equal {
                        return if sc.descending { cmp.reverse() } else { cmp };
                    }
                } else {
                    let cmp = ca.to_lowercase().cmp(&cb.to_lowercase());
                    if cmp != std::cmp::Ordering::Equal {
                        return if sc.descending { cmp.reverse() } else { cmp };
                    }
                }
            }
            std::cmp::Ordering::Equal
        });

        sd.rows = vec![header];
        sd.rows.extend(body);
    }
    Ok(())
}

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
        let none_str = "None".to_string();
        let name = row
            .get(0)
            .and_then(|c| c.value.as_ref())
            .unwrap_or(&none_str);
        let age = row
            .get(1)
            .and_then(|c| c.value.as_ref())
            .unwrap_or(&none_str);
        println!("  Row {}: {} - {}", i, name, age);
    }

    // Sort by column 1 (Age) in descending order
    let sort_columns = vec![SortColumn {
        column: 1,
        descending: true,
    }];

    data.insert("Sheet1".to_string(), sheet);

    let result = sort_data(&mut data, "Sheet1", &sort_columns);
    println!("Sort result: {:?}", result);

    let sorted_sheet = data.get("Sheet1").unwrap();
    println!("After sort:");
    for (i, row) in sorted_sheet.rows.iter().enumerate() {
        let none_str = "None".to_string();
        let name = row
            .get(0)
            .and_then(|c| c.value.as_ref())
            .unwrap_or(&none_str);
        let age = row
            .get(1)
            .and_then(|c| c.value.as_ref())
            .unwrap_or(&none_str);
        println!("  Row {}: {} - {}", i, name, age);
    }

    // Check first data row
    // Check first data row
    if sorted_sheet.rows.len() > 1 {
        let first_age = sorted_sheet.rows[1][1].value.as_ref();
        println!("First age after header: {:?}", first_age);
    }
}
