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

pub fn create_large_xlsx(path: &std::path::Path) {
    let mut wb = rust_xlsxwriter::Workbook::new();
    let ws = wb.add_worksheet();
    ws.set_name("Sheet1").unwrap();
    ws.write_string(0, 0, "ID").unwrap();
    ws.write_string(0, 1, "Name").unwrap();
    ws.write_string(0, 2, "Value").unwrap();

    for i in 1..=100 {
        let row = i as u32;
        ws.write_number(row, 0, i as f64).unwrap();
        ws.write_string(row, 1, &format!("Item {}", i)).unwrap();
        ws.write_number(row, 2, (i * 10) as f64).unwrap();
    }
    wb.save(path).unwrap();
}

pub fn create_large_modified_xlsx(path: &std::path::Path) {
    let mut wb = rust_xlsxwriter::Workbook::new();
    let ws = wb.add_worksheet();
    ws.set_name("Sheet1").unwrap();
    ws.write_string(0, 0, "ID").unwrap();
    ws.write_string(0, 1, "Name").unwrap();
    ws.write_string(0, 2, "Value").unwrap();

    for i in 1..=100 {
        let row = i as u32;
        ws.write_number(row, 0, i as f64).unwrap();
        ws.write_string(row, 1, &format!("Item {}", i)).unwrap();
        // Modify values for some rows
        let value = if i % 10 == 0 {
            (i * 15) as f64
        } else {
            (i * 10) as f64
        };
        ws.write_number(row, 2, value).unwrap();
    }
    wb.save(path).unwrap();
}

pub fn create_unicode_xlsx(path: &std::path::Path, values: &[&str]) {
    let mut wb = rust_xlsxwriter::Workbook::new();
    let ws = wb.add_worksheet();
    ws.set_name("Sheet1").unwrap();
    ws.write_string(0, 0, "Language").unwrap();

    for (i, value) in values.iter().enumerate() {
        ws.write_string((i + 1) as u32, 0, *value).unwrap();
    }
    wb.save(path).unwrap();
}
