use calamine::Reader;
use std::fs;
use std::path::Path;

fn main() {
    let path = "/tmp/debug_clear.xlsx";

    // Clean up old file
    if Path::new(path).exists() {
        fs::remove_file(path).ok();
    }

    // Create a simple Excel file
    let mut wb = rust_xlsxwriter::Workbook::new();
    let ws = wb.add_worksheet();
    ws.set_name("Sheet1").unwrap();

    // Write some initial data
    ws.write_string(0, 0, "Name").unwrap();
    ws.write_string(0, 1, "Age").unwrap();
    ws.write_string(0, 2, "City").unwrap();
    ws.write_string(1, 0, "Alice").unwrap();
    ws.write_number(1, 1, 25).unwrap();
    ws.write_string(1, 2, "New York").unwrap();

    wb.save(path).unwrap();

    println!("Created test file at: {}", path);

    // Read the file to verify initial data
    let mut workbook: calamine::Xlsx<_> = calamine::open_workbook(path).unwrap();
    let range = workbook.worksheet_range("Sheet1").unwrap();

    println!("Initial data at A1:C1:");
    println!("  A1 (0,0): {:?}", range.get_value((0, 0)));
    println!("  B1 (0,1): {:?}", range.get_value((0, 1)));
    println!("  C1 (0,2): {:?}", range.get_value((0, 2)));

    // Now clear range A1:C1
    use excel_core::excel_write::clear_range;
    use excel_core::types::SecurityParams;

    let params = SecurityParams {
        dry_run: false,
        create_backup: false,
        file_path: path.to_string(),
    };

    let result = clear_range(path, &params, "Sheet1", "A1:C1").unwrap();
    println!("Clear result: {:?}", result.success);

    // Read again to verify clearing
    let mut workbook2: calamine::Xlsx<_> = calamine::open_workbook(path).unwrap();
    let range2 = workbook2.worksheet_range("Sheet1").unwrap();

    println!("After clearing A1:C1:");
    println!("  A1 (0,0): {:?}", range2.get_value((0, 0)));
    println!("  B1 (0,1): {:?}", range2.get_value((0, 1)));
    println!("  C1 (0,2): {:?}", range2.get_value((0, 2)));

    // Also test using read_cell
    use excel_core::excel_read::read_cell;
    let cell0 = read_cell(path, "Sheet1", 0, 0);
    println!("read_cell(0,0) result: {:?}", cell0);

    let cell1 = read_cell(path, "Sheet1", 0, 1);
    println!("read_cell(0,1) result: {:?}", cell1);

    let cell2 = read_cell(path, "Sheet1", 0, 2);
    println!("read_cell(0,2) result: {:?}", cell2);

    // Clean up
    fs::remove_file(path).ok();
}
