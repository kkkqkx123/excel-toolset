use calamine::Reader;
use std::fs;
use std::path::Path;

fn main() {
    let path = "/tmp/debug_test.xlsx";

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

    // Read the file using calamine
    let mut workbook: calamine::Xlsx<_> = calamine::open_workbook(path).unwrap();
    let range = workbook.worksheet_range("Sheet1").unwrap();

    println!("Reading cell at row 1, col 0:");
    println!("  Value: {:?}", range.get_value((1, 0)));

    println!("Reading cell at row 4, col 0:");
    println!("  Value: {:?}", range.get_value((4, 0)));

    // Now write data at A5 (row 4, col 0)
    use excel_core::excel_write::write_range;
    use excel_core::types::{CellValue, SecurityParams};

    let params = SecurityParams {
        dry_run: false,
        create_backup: false,
        file_path: path.to_string(),
    };

    let data = vec![
        vec![
            CellValue::String("X".to_string()),
            CellValue::String("Y".to_string()),
        ],
        vec![
            CellValue::String("Z".to_string()),
            CellValue::String("W".to_string()),
        ],
    ];

    let result = write_range(path, &params, "Sheet1", "A5:B6", &data).unwrap();
    println!("Write result: {:?}", result.success);

    // Read again
    let mut workbook2: calamine::Xlsx<_> = calamine::open_workbook(path).unwrap();
    let range2 = workbook2.worksheet_range("Sheet1").unwrap();

    println!("After writing:");
    println!("  Cell at row 4, col 0: {:?}", range2.get_value((4, 0)));
    println!("  Cell at row 4, col 1: {:?}", range2.get_value((4, 1)));
    println!("  Cell at row 5, col 0: {:?}", range2.get_value((5, 0)));
    println!("  Cell at row 5, col 1: {:?}", range2.get_value((5, 1)));

    // Clean up
    fs::remove_file(path).ok();
}
