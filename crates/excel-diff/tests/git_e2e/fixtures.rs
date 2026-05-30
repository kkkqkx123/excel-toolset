pub fn create_simple_xlsx(path: &std::path::Path) {
    let mut wb = rust_xlsxwriter::Workbook::new();
    let ws = wb.add_worksheet();
    ws.set_name("Sheet1").unwrap();
    ws.write_string(0, 0, "Name").unwrap();
    ws.write_string(0, 1, "Value").unwrap();
    ws.write_string(1, 0, "Alice").unwrap();
    ws.write_number(1, 1, 100.0).unwrap();
    ws.write_string(2, 0, "Bob").unwrap();
    ws.write_number(2, 1, 200.0).unwrap();
    wb.save(path).unwrap();
}

pub fn create_modified_xlsx(path: &std::path::Path) {
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
    wb.save(path).unwrap();
}

pub fn create_multi_sheet_xlsx(path: &std::path::Path) {
    let mut wb = rust_xlsxwriter::Workbook::new();
    let ws1 = wb.add_worksheet();
    ws1.set_name("Revenue").unwrap();
    ws1.write_string(0, 0, "Q1").unwrap();
    ws1.write_number(0, 1, 1000.0).unwrap();

    let ws2 = wb.add_worksheet();
    ws2.set_name("Expenses").unwrap();
    ws2.write_string(0, 0, "Rent").unwrap();
    ws2.write_number(0, 1, 500.0).unwrap();

    wb.save(path).unwrap();
}

pub fn create_sheet_del_xlsx(path: &std::path::Path) {
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

    wb.save(path).unwrap();
}

pub fn create_empty_xlsx(path: &std::path::Path) {
    let mut wb = rust_xlsxwriter::Workbook::new();
    let ws = wb.add_worksheet();
    ws.set_name("Sheet1").unwrap();
    wb.save(path).unwrap();
}

pub fn create_formulas_xlsx(path: &std::path::Path) {
    let mut wb = rust_xlsxwriter::Workbook::new();
    let ws = wb.add_worksheet();
    ws.set_name("Sheet1").unwrap();
    ws.write_string(0, 0, "A").unwrap();
    ws.write_number(0, 1, 10.0).unwrap();
    ws.write_string(1, 0, "B").unwrap();
    ws.write_number(1, 1, 20.0).unwrap();
    ws.write_string(2, 0, "Sum").unwrap();
    ws.write_formula(2, 1, "=SUM(B1:B2)").unwrap();
    wb.save(path).unwrap();
}

pub fn create_formulas_modified_xlsx(path: &std::path::Path) {
    let mut wb = rust_xlsxwriter::Workbook::new();
    let ws = wb.add_worksheet();
    ws.set_name("Sheet1").unwrap();
    ws.write_string(0, 0, "A").unwrap();
    ws.write_number(0, 1, 10.0).unwrap();
    ws.write_string(1, 0, "B").unwrap();
    ws.write_number(1, 1, 20.0).unwrap();
    ws.write_string(2, 0, "Average").unwrap();
    ws.write_formula(2, 1, "=AVERAGE(B1:B2)").unwrap();
    wb.save(path).unwrap();
}

pub fn create_formulas_passive_change_xlsx(path: &std::path::Path) {
    let mut wb = rust_xlsxwriter::Workbook::new();
    let ws = wb.add_worksheet();
    ws.set_name("Sheet1").unwrap();
    ws.write_string(0, 0, "A").unwrap();
    ws.write_number(0, 1, 30.0).unwrap();
    ws.write_string(1, 0, "B").unwrap();
    ws.write_number(1, 1, 20.0).unwrap();
    ws.write_string(2, 0, "Sum").unwrap();
    ws.write_formula(2, 1, "=SUM(B1:B2)").unwrap();
    wb.save(path).unwrap();
}