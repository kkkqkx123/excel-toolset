use excel_types::CellValue;

use super::*;

// --- Helpers ---

fn num(n: f64) -> CellValue {
    CellValue::Number(n)
}
fn txt(s: &str) -> CellValue {
    CellValue::String(s.into())
}
fn bool_val(b: bool) -> CellValue {
    CellValue::Bool(b)
}
fn err(e: &str) -> CellValue {
    CellValue::Error(e.into())
}

// Build range-marker prefix for test data: [sentinel, rows, ...cells]
fn range_marker(cols: usize, rows: usize) -> Vec<CellValue> {
    vec![
        CellValue::Number(-(cols as f64 + 1_000_000.0)),
        CellValue::Number(rows as f64),
    ]
}

// --- Range marker tests ---

#[test]
fn test_consume_range_marker_valid() {
    let mut args = range_marker(2, 3);
    // Add 6 cell values (2 cols * 3 rows)
    for i in 0..6 {
        args.push(num(i as f64));
    }
    // Append extra params after the range
    args.push(num(2.0)); // col_index for VLOOKUP

    assert!(is_range_marker(&args[0]));
    let result = consume_range_marker(&args, 0);
    assert!(result.is_some());
    let (cols, rows, end) = result.unwrap();
    assert_eq!(cols, 2);
    assert_eq!(rows, 3);
    assert_eq!(end, 8); // 2 (marker+rows) + 6 cells
    assert_eq!(args[end], num(2.0)); // extra param preserved after end
}

#[test]
fn test_consume_range_marker_not_found() {
    let args = vec![num(42.0), txt("hello")];
    assert!(consume_range_marker(&args, 0).is_none());
}

#[test]
fn test_col_letters() {
    assert_eq!(col_index_to_letters(0), "A");
    assert_eq!(col_index_to_letters(25), "Z");
    assert_eq!(col_index_to_letters(26), "AA");
    assert_eq!(col_index_to_letters(27), "AB");
    assert_eq!(col_index_to_letters(701), "ZZ");
}

#[test]
fn test_parse_a1_ref() {
    assert_eq!(parse_a1_ref("A1"), Some((String::new(), 0, 0)));
    assert_eq!(parse_a1_ref("B2"), Some((String::new(), 1, 1)));
    assert_eq!(parse_a1_ref("ZZ10"), Some((String::new(), 9, 701)));
    assert_eq!(parse_a1_ref("Sheet1!A1"), Some(("Sheet1".into(), 0, 0)));
    assert_eq!(
        parse_a1_ref("'My Sheet'!B3"),
        Some(("My Sheet".into(), 2, 1))
    );
    assert_eq!(parse_a1_ref(""), None);
    assert_eq!(parse_a1_ref("1A"), None);
}

// --- VLOOKUP tests ---

#[test]
fn test_vlookup_exact_match_with_range() {
    // Simulate VLOOKUP(42, A1:B3, 2, FALSE)
    // Range: 3 rows, 2 cols: [(10,"a"), (20,"b"), (42,"target")]
    let mut args = vec![num(42.0)]; // lookup_value
    args.extend(range_marker(2, 3));
    args.extend(vec![
        num(10.0),
        txt("a"), // row 1
        num(20.0),
        txt("b"), // row 2
        num(42.0),
        txt("target"), // row 3
    ]);
    args.push(num(2.0)); // col_index
    args.push(bool_val(false)); // exact
    let dummy_provider = InMemoryDataProvider::new_shared();
    let result = lookup_vlookup(&args, &*dummy_provider);
    assert_eq!(result, txt("target"));
}

#[test]
fn test_vlookup_not_found() {
    let mut args = vec![num(99.0)];
    args.extend(range_marker(1, 2));
    args.extend(vec![num(10.0), num(20.0)]);
    args.push(num(1.0));
    args.push(bool_val(false));
    let dummy_provider = InMemoryDataProvider::new_shared();
    let result = lookup_vlookup(&args, &*dummy_provider);
    assert_eq!(result, err("#N/A"));
}

#[test]
fn test_vlookup_approximate_match() {
    // Sorted ascending: 10, 20, 30. Lookup 25 -> returns 20's row value.
    let mut args = vec![num(25.0)];
    args.extend(range_marker(2, 3));
    args.extend(vec![
        num(10.0),
        txt("ten"),
        num(20.0),
        txt("twenty"),
        num(30.0),
        txt("thirty"),
    ]);
    args.push(num(2.0)); // col_index
    // No range_lookup = approximate (default TRUE)
    let dummy_provider = InMemoryDataProvider::new_shared();
    let result = lookup_vlookup(&args, &*dummy_provider);
    assert_eq!(result, txt("twenty"));
}

#[test]
fn test_vlookup_col_out_of_range() {
    let mut args = vec![num(1.0)];
    args.extend(range_marker(1, 1));
    args.extend(vec![num(10.0)]);
    args.push(num(5.0)); // col_index 5 exceeds 1 column
    args.push(bool_val(false));
    let dummy_provider = InMemoryDataProvider::new_shared();
    let result = lookup_vlookup(&args, &*dummy_provider);
    assert_eq!(result, err("#REF!"));
}

#[test]
fn test_vlookup_insufficient_args() {
    let dummy_provider = InMemoryDataProvider::new_shared();
    assert_eq!(
        lookup_vlookup(&[num(1.0)], &*dummy_provider),
        err("#VALUE!")
    );
}

// --- HLOOKUP tests ---

#[test]
fn test_hlookup_exact_match_with_range() {
    // Table: header row [10, 20, 42], data row [a, b, target]
    // HLOOKUP(42, table, 2, FALSE)
    let mut args = vec![num(42.0)];
    args.extend(range_marker(3, 2));
    args.extend(vec![
        num(10.0),
        num(20.0),
        num(42.0), // row 1 (header)
        txt("a"),
        txt("b"),
        txt("target"), // row 2 (data)
    ]);
    args.push(num(2.0)); // row_index
    args.push(bool_val(false)); // exact
    let dummy_provider = InMemoryDataProvider::new_shared();
    let result = lookup_hlookup(&args, &*dummy_provider);
    assert_eq!(result, txt("target"));
}

#[test]
fn test_hlookup_not_found() {
    let mut args = vec![num(99.0)];
    args.extend(range_marker(2, 1));
    args.extend(vec![num(10.0), num(20.0)]);
    args.push(num(1.0));
    args.push(bool_val(false));
    let dummy_provider = InMemoryDataProvider::new_shared();
    let result = lookup_hlookup(&args, &*dummy_provider);
    assert_eq!(result, err("#N/A"));
}

// --- MATCH tests ---

#[test]
fn test_match_exact() {
    let mut args = vec![txt("b")];
    args.extend(range_marker(1, 3));
    args.extend(vec![txt("a"), txt("b"), txt("c")]);
    args.push(num(0.0)); // match_type exact
    let dummy_provider = InMemoryDataProvider::new_shared();
    let result = lookup_match(&args, &*dummy_provider);
    assert_eq!(result, num(2.0)); // 1-based position
}

#[test]
fn test_match_exact_not_found() {
    let mut args = vec![txt("x")];
    args.extend(range_marker(1, 3));
    args.extend(vec![txt("a"), txt("b"), txt("c")]);
    args.push(num(0.0));
    let dummy_provider = InMemoryDataProvider::new_shared();
    let result = lookup_match(&args, &*dummy_provider);
    assert_eq!(result, err("#N/A"));
}

#[test]
fn test_match_less_than() {
    // match_type=1: largest value <= lookup. Sorted ascending: 10, 20, 30. Lookup 25 -> 20 at pos 2.
    let mut args = vec![num(25.0)];
    args.extend(range_marker(1, 3));
    args.extend(vec![num(10.0), num(20.0), num(30.0)]);
    args.push(num(1.0)); // default match_type = less than
    let dummy_provider = InMemoryDataProvider::new_shared();
    let result = lookup_match(&args, &*dummy_provider);
    assert_eq!(result, num(2.0));
}

// --- INDEX tests ---

#[test]
fn test_index_range_2d() {
    // INDEX(A1:B3, 2, 2)
    let mut args = Vec::new();
    args.extend(range_marker(2, 3));
    args.extend(vec![
        num(1.0),
        txt("x"), // row 1
        num(2.0),
        txt("y"), // row 2
        num(3.0),
        txt("z"), // row 3
    ]);
    args.push(num(2.0)); // row_num
    args.push(num(2.0)); // col_num
    let dummy_provider = InMemoryDataProvider::new_shared();
    let result = lookup_index(&args, &*dummy_provider);
    assert_eq!(result, txt("y"));
}

#[test]
fn test_index_inline_1d() {
    // INDEX({10}, 1) -> 10 (inline array flattened to single value)
    let dummy_provider = InMemoryDataProvider::new_shared();
    let result = lookup_index(
        &[num(10.0), num(1.0)], // args[0]=array_value, args[1]=row_num
        &*dummy_provider,
    );
    assert_eq!(result, num(10.0));

    // INDEX({10}, 2) -> #REF! (row 2 is out of bounds)
    let result2 = lookup_index(&[num(10.0), num(2.0)], &*dummy_provider);
    assert_eq!(result2, err("#REF!"));
}

#[test]
fn test_index_row_out_of_range() {
    let mut args = Vec::new();
    args.extend(range_marker(1, 1));
    args.extend(vec![num(42.0)]);
    args.push(num(5.0)); // row_num 5 exceeds 1 row
    args.push(num(1.0)); // col_num
    let dummy_provider = InMemoryDataProvider::new_shared();
    let result = lookup_index(&args, &*dummy_provider);
    assert_eq!(result, err("#REF!"));
}

// --- XLOOKUP tests ---

#[test]
fn test_xlookup_exact_match() {
    // XLOOKUP("b", lookup_array, return_array)
    let mut args = vec![txt("b")];
    args.extend(range_marker(1, 3));
    args.extend(vec![txt("a"), txt("b"), txt("c")]); // lookup_array
    args.extend(range_marker(1, 3));
    args.extend(vec![num(1.0), num(2.0), num(3.0)]); // return_array
    let dummy_provider = InMemoryDataProvider::new_shared();
    let result = lookup_xlookup(&args, &*dummy_provider);
    assert_eq!(result, num(2.0));
}

#[test]
fn test_xlookup_not_found_with_default() {
    let mut args = vec![txt("z")];
    args.extend(range_marker(1, 2));
    args.extend(vec![txt("a"), txt("b")]); // lookup_array
    args.extend(range_marker(1, 2));
    args.extend(vec![num(10.0), num(20.0)]); // return_array
    args.push(txt("not found")); // if_not_found
    let dummy_provider = InMemoryDataProvider::new_shared();
    let result = lookup_xlookup(&args, &*dummy_provider);
    assert_eq!(result, txt("not found"));
}

// --- CHOOSE tests ---

#[test]
fn test_choose() {
    let result = lookup_choose(&[num(2.0), txt("a"), txt("b"), txt("c")]);
    assert_eq!(result, txt("b"));
}

#[test]
fn test_choose_out_of_range() {
    let result = lookup_choose(&[num(5.0), txt("a"), txt("b")]);
    assert_eq!(result, err("#VALUE!"));
}

// --- ADDRESS tests ---

#[test]
fn test_address_absolute() {
    let result = lookup_address(&[num(1.0), num(1.0), num(1.0)]);
    assert_eq!(result, txt("$A$1"));
}

#[test]
fn test_address_relative() {
    // row=2, col=3, abs=4 (relative) -> C2
    let result = lookup_address(&[num(2.0), num(3.0), num(4.0)]);
    assert_eq!(result, txt("C2"));
}

#[test]
fn test_address_with_sheet() {
    let result = lookup_address(&[num(1.0), num(1.0), num(1.0), bool_val(true), txt("Data")]);
    assert_eq!(result, txt("Data!$A$1"));
}

// --- ROWS / COLUMNS tests ---

#[test]
fn test_rows_with_range() {
    let mut args = Vec::new();
    args.extend(range_marker(3, 5));
    args.extend(vec![CellValue::Empty; 15]); // 3*5
    assert_eq!(lookup_rows(&args), num(5.0));
}

#[test]
fn test_columns_with_range() {
    let mut args = Vec::new();
    args.extend(range_marker(3, 5));
    args.extend(vec![CellValue::Empty; 15]);
    assert_eq!(lookup_columns(&args), num(3.0));
}

#[test]
fn test_rows_flat() {
    // Flat args: treat as 1 row
    assert_eq!(lookup_rows(&[num(1.0), num(2.0)]), num(1.0));
}

#[test]
fn test_columns_flat() {
    // Flat args: treat as N columns
    assert_eq!(lookup_columns(&[num(1.0), num(2.0), num(3.0)]), num(3.0));
}

// --- OFFSET / INDIRECT stubs ---

#[test]
fn test_offset_no_context() {
    let args = vec![num(1.0), num(2.0), num(3.0)];
    let dummy_provider = InMemoryDataProvider::new_shared();
    assert_eq!(lookup_offset(&args, &*dummy_provider), err("#REF!"));
}

#[test]
fn test_indirect_invalid_ref() {
    let dummy_provider = InMemoryDataProvider::new_shared();
    assert_eq!(
        lookup_indirect(&[txt("not_a_ref")], &*dummy_provider),
        err("#REF!")
    );
}

#[test]
fn test_indirect_no_sheet() {
    // A1 without sheet: no sheet context available
    let dummy_provider = InMemoryDataProvider::new_shared();
    assert_eq!(
        lookup_indirect(&[txt("A1")], &*dummy_provider),
        err("#REF!")
    );
}

// --- Value comparison tests ---

#[test]
fn test_lookup_values_equal_strings_case_insensitive() {
    assert!(lookup_values_equal(&txt("Hello"), &txt("HELLO")));
    assert!(lookup_values_equal(&txt("hello"), &txt("hello")));
}

#[test]
fn test_lookup_values_equal_numbers() {
    assert!(lookup_values_equal(&num(42.0), &num(42.0)));
    assert!(!lookup_values_equal(&num(42.0), &num(43.0)));
}

#[test]
fn test_lookup_values_equal_cross_type() {
    // Number vs string: never equal in lookup
    assert!(!lookup_values_equal(&num(42.0), &txt("42")));
}

// --- A trivial shared provider factory for tests ---

/// Extend InMemoryDataProvider for tests that need a shared ref.
use crate::engine::InMemoryDataProvider;

/// Extension trait to get a shared DataProvider for tests.
trait InMemoryExt {
    fn new_shared() -> std::sync::Arc<InMemoryDataProvider>;
}

impl InMemoryExt for InMemoryDataProvider {
    fn new_shared() -> std::sync::Arc<InMemoryDataProvider> {
        std::sync::Arc::new(InMemoryDataProvider::new())
    }
}
