use std::fs;
use std::path::Path;

use excel_core::excel_read::{
    list_sheets, read_cell, read_file_info, read_formula, read_range, read_sheet_all,
};
use excel_core::excel_write::{
    add_sheet, append_rows, clear_range, create_file, delete_rows, delete_sheet, insert_rows,
    merge_cells, rename_sheet, set_formula, write_cell, write_range,
};
use excel_core::features::comments::{add_comment, delete_comment, get_comment, update_comment};
use excel_core::features::search::{
    MatchType, SearchQuery, SearchType, search_sheet, search_workbook,
};
use excel_core::operations::{dedup_sheet, filter_rows, sort_sheet};
use excel_core::security::{compute_file_hash, create_backup, rollback};
use excel_core::types::{
    BackupInfo, CellValue, FilterCondition, FilterOp, SecurityParams, SortColumn,
};

fn test_root() -> &'static tempfile::TempDir {
    use std::sync::OnceLock;
    static ROOT: OnceLock<tempfile::TempDir> = OnceLock::new();
    ROOT.get_or_init(|| tempfile::tempdir().expect("Failed to create temp dir for tests"))
}

fn setup_test_file(name: &str) -> String {
    let file_path = test_root().path().join(name);
    file_path.to_string_lossy().to_string()
}

fn cleanup_test_file(_path: &str) {
    // TempDir handles cleanup on process exit via the static reference.
}

fn create_simple_test_file(path: &str, sheet_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let parent = Path::new(path).parent().ok_or("Invalid path")?;
    fs::create_dir_all(parent)?;

    let mut wb = rust_xlsxwriter::Workbook::new();
    let ws = wb.add_worksheet();
    ws.set_name(sheet_name)?;
    ws.write_string(0, 0, "Name")?;
    ws.write_string(0, 1, "Age")?;
    ws.write_string(0, 2, "City")?;
    ws.write_string(1, 0, "Alice")?;
    ws.write_number(1, 1, 25)?;
    ws.write_string(1, 2, "New York")?;
    ws.write_string(2, 0, "Bob")?;
    ws.write_number(2, 1, 30)?;
    ws.write_string(2, 2, "London")?;
    ws.write_string(3, 0, "Charlie")?;
    ws.write_number(3, 1, 35)?;
    ws.write_string(3, 2, "Paris")?;
    wb.save(path)?;
    Ok(())
}

mod file_read_tests;

mod file_write_tests;

mod data_operations_tests;

mod security_tests;

mod error_handling_tests;

mod business_scenarios;
