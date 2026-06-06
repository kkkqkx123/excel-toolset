use std::fs;
use std::path::Path;

fn main() {
    let path = "/tmp/debug_sort.xlsx";

    // Clean up old file
    if Path::new(path).exists() {
        fs::remove_file(path).ok();
    }

    // Create a simple Excel file
    let mut wb = rust_xlsxwriter::Workbook::new();
    let ws = wb.add_worksheet();
    ws.set_name("Sheet1").unwrap();

    // Write header and data
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

    // Read initial data
    use excel_core::excel_read::read_sheet_all;
    let sheet = read_sheet_all(path, "Sheet1").unwrap();
    println!("Initial data:");
    let none_str = "None".to_string();
    for (i, row) in sheet.rows.iter().enumerate() {
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

    // Now sort by Age column (column 1) in descending order
    use excel_core::operations::sort_sheet;
    use excel_core::types::{SecurityParams, SortColumn};

    let params = SecurityParams {
        dry_run: false,
        create_backup: false,
        file_path: path.to_string(),
    };

    let sort_columns = vec![SortColumn {
        column: 1,
        descending: true,
    }];

    let result = sort_sheet(path, &params, "Sheet1", &sort_columns).unwrap();
    println!("Sort result: {:?}", result.success);

    // Read sorted data
    let sorted_sheet = read_sheet_all(path, "Sheet1").unwrap();
    println!("Sorted data:");
    for (i, row) in sorted_sheet.rows.iter().enumerate() {
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

    // Check the first data row (row 1, after header)
    if sorted_sheet.rows.len() > 1 {
        let first_age = sorted_sheet.rows[1][1].value.as_ref();
        println!("First age after header: {:?}", first_age);
    }

    // Clean up
    fs::remove_file(path).ok();
}
