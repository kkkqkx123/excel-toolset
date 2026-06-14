use std::fs;
use std::path::Path;

fn main() {
    let path = "/tmp/debug_append.xlsx";

    // Clean up old file
    if Path::new(path).exists() {
        fs::remove_file(path).ok();
    }

    // Create a simple Excel file
    let mut wb = rust_xlsxwriter::Workbook::new();
    let ws = wb.add_worksheet();
    ws.set_name("Sheet1").unwrap();

    // Write 4 rows of data (0-3)
    ws.write_string(0, 0, "Name").unwrap();
    ws.write_string(0, 1, "Age").unwrap();
    ws.write_string(0, 2, "City").unwrap();
    ws.write_string(1, 0, "Alice").unwrap();
    ws.write_number(1, 1, 25).unwrap();
    ws.write_string(1, 2, "New York").unwrap();
    ws.write_string(2, 0, "Bob").unwrap();
    ws.write_number(2, 1, 30).unwrap();
    ws.write_string(2, 2, "London").unwrap();
    ws.write_string(3, 0, "Charlie").unwrap();
    ws.write_number(3, 1, 35).unwrap();
    ws.write_string(3, 2, "Paris").unwrap();

    wb.save(path).unwrap();

    println!("Created test file at: {}", path);

    // Read initial sheet
    use excel_core::excel_read::read_sheet_all;
    let sheet = read_sheet_all(path, "Sheet1").unwrap();
    println!("Initial row count: {}", sheet.rows.len());
    println!("Initial data:");
    for (i, row) in sheet.rows.iter().enumerate() {
        let none_str = "None".to_string();
        let val = row
            .get(0)
            .and_then(|c| c.value.as_ref())
            .unwrap_or(&none_str);
        println!("  Row {}: {}", i, val);
    }

    // Now append data at row 4
    use excel_core::excel_write::write_range;
    use excel_core::types::{CellValue, SecurityParams};

    let params = SecurityParams {
        dry_run: false,
        create_backup: false,
        file_path: path.to_string(),
    };

    let new_row = sheet.rows.len() as u32;
    println!("Appending at row {}", new_row);

    let data = vec![vec![
        CellValue::String("David".to_string()),
        CellValue::Number(45.0),
        CellValue::String("Berlin".to_string()),
    ]];

    let result = write_range(
        path,
        &params,
        "Sheet1",
        &format!("A{}:C{}", new_row, new_row),
        &data,
    )
    .unwrap();
    println!("Write result: {:?}", result.success);

    // Read updated sheet
    let updated_sheet = read_sheet_all(path, "Sheet1").unwrap();
    println!("Updated row count: {}", updated_sheet.rows.len());
    println!("Updated data:");
    for (i, row) in updated_sheet.rows.iter().enumerate() {
        let none_str = "None".to_string();
        let val = row
            .get(0)
            .and_then(|c| c.value.as_ref())
            .unwrap_or(&none_str);
        println!("  Row {}: {}", i, val);
    }

    // Check if row count increased
    assert!(updated_sheet.rows.len() > sheet.rows.len());
    println!(
        "Assertion passed: row count increased from {} to {}",
        sheet.rows.len(),
        updated_sheet.rows.len()
    );

    // Check the last row
    let last_row = updated_sheet.rows.last().unwrap();
    let last_val = last_row.get(0).and_then(|c| c.value.as_ref());
    println!("Last row first cell: {:?}", last_val);

    // Clean up
    fs::remove_file(path).ok();
}
