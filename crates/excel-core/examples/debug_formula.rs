use std::fs;
use std::path::Path;

fn main() {
    let path = "/tmp/debug_formula.xlsx";

    // Clean up old file
    if Path::new(path).exists() {
        fs::remove_file(path).ok();
    }

    // Create a simple Excel file
    let mut wb = rust_xlsxwriter::Workbook::new();
    let ws = wb.add_worksheet();
    ws.set_name("Sheet1").unwrap();

    // Write some data
    ws.write_number(1, 0, 10.0).unwrap();
    ws.write_number(2, 0, 20.0).unwrap();
    ws.write_number(3, 0, 30.0).unwrap();

    // Test writing formula with = prefix
    ws.write_formula(4, 0, "=SUM(A2:A4)").unwrap();
    println!("Wrote formula: =SUM(A2:A4) at A5");

    wb.save(path).unwrap();

    println!("Created test file at: {}", path);

    // Read back the formula
    use calamine::Reader;
    let mut workbook: calamine::Xlsx<_> = calamine::open_workbook(path).unwrap();
    let formulas = workbook.worksheet_formula("Sheet1").unwrap();

    println!("Formula at A5: {:?}", formulas.get_value((4, 0)));

    // Also test using read_formula
    use excel_core::excel_read::read_formula;
    let formula = read_formula(path, "Sheet1", "A5");
    println!("read_formula result: {:?}", formula);

    // Test without = prefix
    let path2 = "/tmp/debug_formula2.xlsx";
    if Path::new(path2).exists() {
        fs::remove_file(path2).ok();
    }

    let mut wb2 = rust_xlsxwriter::Workbook::new();
    let ws2 = wb2.add_worksheet();
    ws2.set_name("Sheet1").unwrap();

    ws2.write_number(1, 0, 10.0).unwrap();
    ws2.write_number(2, 0, 20.0).unwrap();
    ws2.write_number(3, 0, 30.0).unwrap();

    ws2.write_formula(4, 0, "SUM(A2:A4)").unwrap();
    println!("Wrote formula: SUM(A2:A4) at A5 (without =)");

    wb2.save(path2).unwrap();

    let mut workbook2: calamine::Xlsx<_> = calamine::open_workbook(path2).unwrap();
    let formulas2 = workbook2.worksheet_formula("Sheet1").unwrap();

    println!(
        "Formula at A5 (without =): {:?}",
        formulas2.get_value((4, 0))
    );

    // Clean up
    fs::remove_file(path).ok();
    fs::remove_file(path2).ok();
}
