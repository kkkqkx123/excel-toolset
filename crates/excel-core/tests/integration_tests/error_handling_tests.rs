use super::*;

#[test]
fn test_nonexistent_file() {
    let path = setup_test_file("nonexistent.xlsx");
    cleanup_test_file(&path);

    let result = read_file_info(&path);
    assert!(result.is_err());

    let result = list_sheets(&path);
    assert!(result.is_err());
}

#[test]
fn test_invalid_sheet_name() {
    let path = setup_test_file("test_invalid_sheet.xlsx");
    cleanup_test_file(&path);

    create_simple_test_file(&path, "Sheet1").unwrap();

    let result = read_cell(&path, "NonExistent", 0, 0);
    assert!(result.is_err());

    let result = read_range(&path, "NonExistent", "A1:C1");
    assert!(result.is_err());
}

#[test]
fn test_invalid_cell_reference() {
    let path = setup_test_file("test_invalid_cell.xlsx");
    cleanup_test_file(&path);

    create_simple_test_file(&path, "Sheet1").unwrap();

    // Reading a cell beyond the data range now returns an empty cell
    let result = read_cell(&path, "Sheet1", 100, 0);
    assert!(result.is_ok());
    let cell = result.unwrap();
    assert_eq!(cell.value, None);
    assert_eq!(cell.data_type, excel_core::types::CellDataType::Empty);

    // Reading a range with invalid reference returns empty data
    let result = read_range(&path, "Sheet1", "Z100:AA101");
    assert!(result.is_ok());
    let range = result.unwrap();
    // The range should be empty or contain only empty cells
    assert!(
        range.is_empty()
            || range
                .iter()
                .all(|row| row.iter().all(|cell| cell.value.is_none()))
    );

    cleanup_test_file(&path);
}

#[test]
fn test_write_nonexistent_sheet() {
    let path = setup_test_file("test_write_nonexistent.xlsx");
    cleanup_test_file(&path);

    create_simple_test_file(&path, "Sheet1").unwrap();

    let params = SecurityParams {
        dry_run: false,
        create_backup: false,
        file_path: path.clone(),
    };

    let result = write_cell(
        &path,
        &params,
        "NonExistent",
        0,
        0,
        &CellValue::String("Test".to_string()),
    );
    assert!(result.is_err());
}

#[test]
fn test_delete_nonexistent_sheet() {
    let path = setup_test_file("test_delete_nonexistent.xlsx");
    cleanup_test_file(&path);

    create_simple_test_file(&path, "Sheet1").unwrap();

    let params = SecurityParams {
        dry_run: false,
        create_backup: false,
        file_path: path.clone(),
    };

    let result = delete_sheet(&path, &params, "NonExistent");
    assert!(result.is_err());
}

#[test]
fn test_rename_to_existing_sheet() {
    let path = setup_test_file("test_rename_existing.xlsx");
    cleanup_test_file(&path);

    let mut wb = rust_xlsxwriter::Workbook::new();
    let ws1 = wb.add_worksheet();
    ws1.set_name("Sheet1").unwrap();
    let ws2 = wb.add_worksheet();
    ws2.set_name("Sheet2").unwrap();
    wb.save(&path).unwrap();

    let params = SecurityParams {
        dry_run: false,
        create_backup: false,
        file_path: path.clone(),
    };

    let result = rename_sheet(&path, &params, "Sheet1", "Sheet2");
    assert!(result.is_err());

    cleanup_test_file(&path);
}
