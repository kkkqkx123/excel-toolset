use super::*;

#[test]
fn test_read_file_info() {
    let path = setup_test_file("test_read_info.xlsx");
    cleanup_test_file(&path);

    create_simple_test_file(&path, "Sheet1").unwrap();

    let info = read_file_info(&path).unwrap();
    assert_eq!(info.path, path);
    assert!(!info.hash.is_empty());
    assert!(info.size > 0);
    assert_eq!(info.sheets, vec!["Sheet1"]);

    cleanup_test_file(&path);
}

#[test]
fn test_list_sheets() {
    let path = setup_test_file("test_list_sheets.xlsx");
    cleanup_test_file(&path);

    create_simple_test_file(&path, "Data").unwrap();

    let sheets = list_sheets(&path).unwrap();
    assert_eq!(sheets, vec!["Data"]);

    cleanup_test_file(&path);
}

#[test]
fn test_read_cell() {
    let path = setup_test_file("test_read_cell.xlsx");
    cleanup_test_file(&path);

    create_simple_test_file(&path, "Sheet1").unwrap();

    let cell = read_cell(&path, "Sheet1", 1, 0).unwrap();
    assert_eq!(cell.value, Some("Alice".to_string()));
    assert_eq!(cell.data_type, excel_core::types::CellDataType::String);

    let cell = read_cell(&path, "Sheet1", 1, 1).unwrap();
    assert_eq!(cell.value, Some("25".to_string()));

    cleanup_test_file(&path);
}

#[test]
fn test_read_range() {
    let path = setup_test_file("test_read_range.xlsx");
    cleanup_test_file(&path);

    create_simple_test_file(&path, "Sheet1").unwrap();

    let range = read_range(&path, "Sheet1", "A1:C2").unwrap();
    assert_eq!(range.len(), 2);
    assert_eq!(range[0][0].value, Some("Name".to_string()));
    assert_eq!(range[1][0].value, Some("Alice".to_string()));

    cleanup_test_file(&path);
}

#[test]
fn test_read_sheet_all() {
    let path = setup_test_file("test_read_sheet.xlsx");
    cleanup_test_file(&path);

    create_simple_test_file(&path, "Data").unwrap();

    let sheet = read_sheet_all(&path, "Data").unwrap();
    assert_eq!(sheet.name, "Data");
    assert_eq!(sheet.rows.len(), 4);
    assert_eq!(sheet.rows[0][0].value, Some("Name".to_string()));

    cleanup_test_file(&path);
}

#[test]
fn test_read_sheet_not_found() {
    let path = setup_test_file("test_not_found.xlsx");
    cleanup_test_file(&path);

    create_simple_test_file(&path, "Sheet1").unwrap();

    let result = read_sheet_all(&path, "NonExistent");
    assert!(result.is_err());

    cleanup_test_file(&path);
}
