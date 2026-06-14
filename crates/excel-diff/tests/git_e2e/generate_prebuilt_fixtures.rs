// Generate prebuilt test fixtures for git e2e tests
//
// This example program generates Excel files that are used by git driver integration tests.
// These prebuilt files are stored in tests/git_e2e/fixtures/ and are loaded directly by tests
// that simulate real git diff driver scenarios.
//
// Usage:
//   cargo run --example generate_prebuilt_fixtures
//
// Note: Most tests use dynamic fixture creation (fixtures.rs), but git driver tests need
// prebuilt files to simulate the git diff protocol accurately.

fn main() {
    let out_dir = std::env::args().nth(1).map_or_else(
        || {
            let p = std::path::PathBuf::from("tests/git_e2e/fixtures");
            std::fs::create_dir_all(&p).unwrap();
            p
        },
        std::path::PathBuf::from,
    );

    // simple.xlsx: 2 columns, header + 2 data rows
    {
        let mut wb = rust_xlsxwriter::Workbook::new();
        let ws = wb.add_worksheet();
        ws.set_name("Sheet1").unwrap();
        ws.write_string(0, 0, "Name").unwrap();
        ws.write_string(0, 1, "Value").unwrap();
        ws.write_string(1, 0, "Alice").unwrap();
        ws.write_number(1, 1, 100.0).unwrap();
        ws.write_string(2, 0, "Bob").unwrap();
        ws.write_number(2, 1, 200.0).unwrap();
        wb.save(out_dir.join("simple.xlsx")).unwrap();
        eprintln!("Generated: {:?}", out_dir.join("simple.xlsx"));
    }

    // modified.xlsx: same header, Alice's value changed, Bob -> Charlie, Dave added
    {
        let mut wb = rust_xlsxwriter::Workbook::new();
        let ws = wb.add_worksheet();
        ws.set_name("Sheet1").unwrap();
        ws.write_string(0, 0, "Name").unwrap();
        ws.write_string(0, 1, "Value").unwrap();
        ws.write_string(1, 0, "Alice").unwrap();
        ws.write_number(1, 1, 150.0).unwrap();
        ws.write_string(2, 0, "Charlie").unwrap();
        ws.write_number(2, 1, 300.0).unwrap();
        ws.write_string(3, 0, "Dave").unwrap();
        ws.write_number(3, 1, 400.0).unwrap();
        wb.save(out_dir.join("modified.xlsx")).unwrap();
        eprintln!("Generated: {:?}", out_dir.join("modified.xlsx"));
    }

    // multi_sheet.xlsx: 2 sheets
    {
        let mut wb = rust_xlsxwriter::Workbook::new();
        let ws1 = wb.add_worksheet();
        ws1.set_name("Revenue").unwrap();
        ws1.write_string(0, 0, "Q1").unwrap();
        ws1.write_number(0, 1, 1000.0).unwrap();
        let ws2 = wb.add_worksheet();
        ws2.set_name("Expenses").unwrap();
        ws2.write_string(0, 0, "Rent").unwrap();
        ws2.write_number(0, 1, 500.0).unwrap();
        wb.save(out_dir.join("multi_sheet.xlsx")).unwrap();
        eprintln!("Generated: {:?}", out_dir.join("multi_sheet.xlsx"));
    }

    // sheet_del.xlsx: like simple.xlsx but with 2 sheets (tests sheet deletion)
    {
        let mut wb = rust_xlsxwriter::Workbook::new();
        let ws1 = wb.add_worksheet();
        ws1.set_name("Sheet1").unwrap();
        ws1.write_string(0, 0, "Name").unwrap();
        ws1.write_string(0, 1, "Value").unwrap();
        ws1.write_string(1, 0, "Alice").unwrap();
        ws1.write_number(1, 1, 100.0).unwrap();
        let ws2 = wb.add_worksheet();
        ws2.set_name("Extra").unwrap();
        ws2.write_string(0, 0, "Note").unwrap();
        ws2.write_string(1, 0, "extra").unwrap();
        wb.save(out_dir.join("sheet_del.xlsx")).unwrap();
        eprintln!("Generated: {:?}", out_dir.join("sheet_del.xlsx"));
    }

    // empty.xlsx: workbook with no data
    {
        let mut wb = rust_xlsxwriter::Workbook::new();
        let ws = wb.add_worksheet();
        ws.set_name("Sheet1").unwrap();
        wb.save(out_dir.join("empty.xlsx")).unwrap();
        eprintln!("Generated: {:?}", out_dir.join("empty.xlsx"));
    }

    // formulas.xlsx: workbook with formulas
    {
        let mut wb = rust_xlsxwriter::Workbook::new();
        let ws = wb.add_worksheet();
        ws.set_name("Sheet1").unwrap();
        ws.write_string(0, 0, "A").unwrap();
        ws.write_number(0, 1, 10.0).unwrap();
        ws.write_string(1, 0, "B").unwrap();
        ws.write_number(1, 1, 20.0).unwrap();
        ws.write_string(2, 0, "Sum").unwrap();
        ws.write_formula(2, 1, "=SUM(B1:B2)").unwrap();
        wb.save(out_dir.join("formulas.xlsx")).unwrap();
        eprintln!("Generated: {:?}", out_dir.join("formulas.xlsx"));
    }
}
