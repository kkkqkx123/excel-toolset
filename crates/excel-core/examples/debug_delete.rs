use std::fs;
use std::path::Path;

fn main() {
    let path = "/tmp/debug_delete.xlsx";

    // Clean up old file
    if Path::new(path).exists() {
        fs::remove_file(path).ok();
    }

    // Create a simple Excel file with one sheet
    let mut wb = rust_xlsxwriter::Workbook::new();
    let ws = wb.add_worksheet();
    ws.set_name("Sheet1").unwrap();
    ws.write_string(0, 0, "Test").unwrap();
    wb.save(path).unwrap();

    println!("Created test file at: {}", path);

    // Read initial sheets
    use calamine::Reader;
    let mut workbook: calamine::Xlsx<_> = calamine::open_workbook(path).unwrap();
    println!("Initial sheets: {:?}", workbook.sheet_names());

    // Now delete the sheet
    use excel_core::excel_write::delete_sheet;
    use excel_core::types::SecurityParams;

    let params = SecurityParams {
        dry_run: false,
        create_backup: false,
        file_path: path.to_string(),
    };

    let result = delete_sheet(path, &params, "Sheet1");
    println!("Delete result: {:?}", result);

    // Read sheets after deletion
    let mut workbook2: calamine::Xlsx<_> = calamine::open_workbook(path).unwrap();
    println!("Sheets after deletion: {:?}", workbook2.sheet_names());

    // Try to list sheets using the API
    use excel_core::excel_read::list_sheets;
    let sheets = list_sheets(path);
    println!("list_sheets result: {:?}", sheets);

    // Clean up
    fs::remove_file(path).ok();
}
