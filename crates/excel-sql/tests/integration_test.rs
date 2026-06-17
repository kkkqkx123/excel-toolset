//! Integration tests for excel-sql crate
//!
//! Tests that verify the integration of multiple components working together:
//! - Excel data loading
//! - SQL queries
//! - Filtering, sorting, deduplication
//! - Multi-sheet operations
//! - Session management

use excel_sql::{ExcelQueryEngine, QuerySession};
use excel_types::{CellData, CellDataType, FilterCondition, FilterOp, SheetData, SortColumn};

fn make_cell(value: Option<&str>, dt: CellDataType) -> CellData {
    CellData {
        value: value.map(|s| s.to_string()),
        data_type: dt,
        formula: None,
    }
}

fn create_sample_sheet(name: &str, rows: Vec<Vec<Option<&str>>>) -> SheetData {
    SheetData {
        name: name.to_string(),
        rows: rows
            .into_iter()
            .map(|row| {
                row.into_iter()
                    .enumerate()
                    .map(|(i, val)| {
                        let dt = match i {
                            0 => CellDataType::String,
                            1 => CellDataType::Int,
                            2 => CellDataType::Float,
                            _ => CellDataType::String,
                        };
                        make_cell(val, dt)
                    })
                    .collect()
            })
            .collect(),
    }
}

#[test]
fn test_integration_full_workflow_with_engine() {
    let mut engine = ExcelQueryEngine::new().expect("Failed to create engine");

    // 1. Load sheet with header
    let sheet = SheetData {
        name: "employees".to_string(),
        rows: vec![
            vec![
                make_cell(Some("Name"), CellDataType::String),
                make_cell(Some("Age"), CellDataType::Int),
                make_cell(Some("Salary"), CellDataType::Float),
            ],
            vec![
                make_cell(Some("Alice"), CellDataType::String),
                make_cell(Some("30"), CellDataType::Int),
                make_cell(Some("50000.50"), CellDataType::Float),
            ],
            vec![
                make_cell(Some("Bob"), CellDataType::String),
                make_cell(Some("25"), CellDataType::Int),
                make_cell(Some("45000.75"), CellDataType::Float),
            ],
            vec![
                make_cell(Some("Charlie"), CellDataType::String),
                make_cell(Some("35"), CellDataType::Int),
                make_cell(Some("60000.00"), CellDataType::Float),
            ],
        ],
    };
    engine
        .load_with_header("employees", &sheet)
        .expect("Failed to load sheet");

    // 2. Query all data
    let result = engine
        .query(r#"SELECT * FROM "employees" ORDER BY Age"#)
        .expect("Failed to query");
    assert_eq!(result.row_count, 3);
    assert_eq!(result.columns, vec!["Name", "Age", "Salary"]);

    // 3. Query with filter
    let result = engine
        .query(r#"SELECT Name, Salary FROM "employees" WHERE Age > 28"#)
        .expect("Failed to query with filter");
    assert_eq!(result.row_count, 2);
    assert!(result.rows.iter().any(|r| {
        r[0].value.as_deref() == Some("Alice") || r[0].value.as_deref() == Some("Charlie")
    }));

    // 4. Aggregate query
    let result = engine
        .query(r#"SELECT AVG(Age) as avg_age, SUM(Salary) as total_salary FROM "employees""#)
        .expect("Failed to execute aggregate query");
    assert_eq!(result.row_count, 1);
    assert_eq!(result.columns, vec!["avg_age", "total_salary"]);

    // 5. Drop table
    engine
        .drop_table("employees")
        .expect("Failed to drop table");
    assert!(
        !engine
            .table_exists("employees")
            .expect("Failed to check table existence")
    );
}

#[test]
fn test_integration_multi_sheet_operations() {
    let mut engine = ExcelQueryEngine::new().expect("Failed to create engine");

    // Load multiple sheets
    let sheet1 = create_sample_sheet(
        "sheet1",
        vec![
            vec![Some("A"), Some("1"), Some("10.5")],
            vec![Some("B"), Some("2"), Some("20.5")],
            vec![Some("C"), Some("3"), Some("30.5")],
        ],
    );

    let sheet2 = create_sample_sheet(
        "sheet2",
        vec![
            vec![Some("A"), Some("100"), Some("100.0")],
            vec![Some("B"), Some("200"), Some("200.0")],
        ],
    );

    engine
        .load_without_header("sheet1", &sheet1)
        .expect("Failed to load sheet1");
    engine
        .load_without_header("sheet2", &sheet2)
        .expect("Failed to load sheet2");

    // List tables
    let tables = engine.list_tables().expect("Failed to list tables");
    assert_eq!(tables.len(), 2);
    assert!(tables.contains(&"sheet1".to_string()));
    assert!(tables.contains(&"sheet2".to_string()));

    // Query from specific table
    let result = engine
        .query(r#"SELECT * FROM "sheet1" WHERE "c1" > 1"#)
        .expect("Failed to query sheet1");
    assert_eq!(result.row_count, 2);
}

#[test]
fn test_integration_session_multiple_queries() {
    let mut session = QuerySession::new().expect("Failed to create session");

    let sheet = SheetData {
        name: "sales".to_string(),
        rows: vec![
            vec![
                make_cell(Some("Product"), CellDataType::String),
                make_cell(Some("Quantity"), CellDataType::Int),
                make_cell(Some("Price"), CellDataType::Float),
            ],
            vec![
                make_cell(Some("Apple"), CellDataType::String),
                make_cell(Some("10"), CellDataType::Int),
                make_cell(Some("1.50"), CellDataType::Float),
            ],
            vec![
                make_cell(Some("Banana"), CellDataType::String),
                make_cell(Some("20"), CellDataType::Int),
                make_cell(Some("0.80"), CellDataType::Float),
            ],
            vec![
                make_cell(Some("Apple"), CellDataType::String),
                make_cell(Some("15"), CellDataType::Int),
                make_cell(Some("1.50"), CellDataType::Float),
            ],
        ],
    };

    session
        .load_sheet("sales", &sheet, true)
        .expect("Failed to load sheet");

    // Query 1: Group by product
    let result1 = session
        .query(r#"SELECT Product, SUM(Quantity) as total_qty FROM "sales" GROUP BY Product"#)
        .expect("Failed to execute group by query");
    assert_eq!(result1.row_count, 2);

    // Query 2: Filter with conditions
    let cond = FilterCondition {
        column: 0,
        operator: FilterOp::Eq,
        value: "Apple".to_string(),
    };
    let result2 = session
        .filter_rows_on_data(&sheet, "sales", &[cond], true)
        .expect("Failed to filter rows");
    assert_eq!(result2.row_count, 2);

    // Query 3: Sort data
    let sort = SortColumn {
        column: 1,
        descending: true,
    };
    let result3 = session
        .sort_sheet_on_data(&sheet, &[sort])
        .expect("Failed to sort sheet");
    let values: Vec<&str> = result3
        .rows
        .iter()
        .skip(1)
        .map(|r| r[1].value.as_deref().unwrap())
        .collect();
    assert!(values[0] > values[1]);
}

#[test]
fn test_integration_filtering_with_various_operators() {
    let sheet = SheetData {
        name: "test_filter".to_string(),
        rows: vec![
            vec![
                make_cell(Some("Name"), CellDataType::String),
                make_cell(Some("Age"), CellDataType::Int),
            ],
            vec![
                make_cell(Some("Alice"), CellDataType::String),
                make_cell(Some("30"), CellDataType::Int),
            ],
            vec![
                make_cell(Some("Bob"), CellDataType::String),
                make_cell(Some("25"), CellDataType::Int),
            ],
            vec![
                make_cell(Some("Charlie"), CellDataType::String),
                make_cell(Some("35"), CellDataType::Int),
            ],
        ],
    };

    let test_cases = vec![
        (FilterOp::Eq, "30", 1),
        (FilterOp::Ne, "30", 2),
        (FilterOp::Gt, "28", 2),
        (FilterOp::Lt, "28", 1),
        (FilterOp::Ge, "30", 2),
        (FilterOp::Le, "30", 2),
    ];

    for (op, value, expected_count) in test_cases {
        let cond = FilterCondition {
            column: 1,
            operator: op.clone(),
            value: value.to_string(),
        };
        let result = excel_sql::filter_rows_on_data(&sheet, "test_filter", &[cond], true)
            .expect("Failed to filter");
        assert_eq!(
            result.row_count, expected_count,
            "Failed for op {:?} with value {}",
            op, value
        );
    }
}

#[test]
fn test_integration_filtering_with_string_operators() {
    let sheet = SheetData {
        name: "test_strings".to_string(),
        rows: vec![
            vec![make_cell(Some("Name"), CellDataType::String)],
            vec![make_cell(Some("Alice Smith"), CellDataType::String)],
            vec![make_cell(Some("Bob Johnson"), CellDataType::String)],
            vec![make_cell(Some("Alice Williams"), CellDataType::String)],
        ],
    };

    // Contains
    let cond = FilterCondition {
        column: 0,
        operator: FilterOp::Contains,
        value: "Alice".to_string(),
    };
    let result = excel_sql::filter_rows_on_data(&sheet, "test_strings", &[cond], true)
        .expect("Failed to filter with Contains");
    assert_eq!(result.row_count, 2);

    // StartsWith
    let cond = FilterCondition {
        column: 0,
        operator: FilterOp::StartsWith,
        value: "Bob".to_string(),
    };
    let result = excel_sql::filter_rows_on_data(&sheet, "test_strings", &[cond], true)
        .expect("Failed to filter with StartsWith");
    assert_eq!(result.row_count, 1);

    // EndsWith
    let cond = FilterCondition {
        column: 0,
        operator: FilterOp::EndsWith,
        value: "Smith".to_string(),
    };
    let result = excel_sql::filter_rows_on_data(&sheet, "test_strings", &[cond], true)
        .expect("Failed to filter with EndsWith");
    assert_eq!(result.row_count, 1);
}

#[test]
fn test_integration_sorting_multi_column() {
    let sheet = SheetData {
        name: "test_sort".to_string(),
        rows: vec![
            vec![
                make_cell(Some("Dept"), CellDataType::String),
                make_cell(Some("Name"), CellDataType::String),
                make_cell(Some("Salary"), CellDataType::Int),
            ],
            vec![
                make_cell(Some("HR"), CellDataType::String),
                make_cell(Some("Bob"), CellDataType::String),
                make_cell(Some("50000"), CellDataType::Int),
            ],
            vec![
                make_cell(Some("IT"), CellDataType::String),
                make_cell(Some("Alice"), CellDataType::String),
                make_cell(Some("60000"), CellDataType::Int),
            ],
            vec![
                make_cell(Some("HR"), CellDataType::String),
                make_cell(Some("Alice"), CellDataType::String),
                make_cell(Some("55000"), CellDataType::Int),
            ],
            vec![
                make_cell(Some("IT"), CellDataType::String),
                make_cell(Some("Bob"), CellDataType::String),
                make_cell(Some("65000"), CellDataType::Int),
            ],
        ],
    };

    // Sort by Dept (ASC), then by Salary (DESC)
    let sort_columns = vec![
        SortColumn {
            column: 0,
            descending: false,
        },
        SortColumn {
            column: 2,
            descending: true,
        },
    ];

    let result = excel_sql::sort_sheet_on_data(&sheet, &sort_columns).expect("Failed to sort");
    assert_eq!(result.rows.len(), 5);
    // First row should be HR with higher salary
    assert_eq!(result.rows[1][0].value.as_deref(), Some("HR"));
    assert_eq!(result.rows[1][1].value.as_deref(), Some("Bob"));
}

#[test]
fn test_integration_deduplication_all_columns() {
    let sheet = SheetData {
        name: "test_dedup".to_string(),
        rows: vec![
            vec![
                make_cell(Some("ID"), CellDataType::Int),
                make_cell(Some("Name"), CellDataType::String),
            ],
            vec![
                make_cell(Some("1"), CellDataType::Int),
                make_cell(Some("Alice"), CellDataType::String),
            ],
            vec![
                make_cell(Some("2"), CellDataType::Int),
                make_cell(Some("Bob"), CellDataType::String),
            ],
            vec![
                make_cell(Some("1"), CellDataType::Int),
                make_cell(Some("Alice"), CellDataType::String),
            ],
            vec![
                make_cell(Some("3"), CellDataType::Int),
                make_cell(Some("Charlie"), CellDataType::String),
            ],
            vec![
                make_cell(Some("2"), CellDataType::Int),
                make_cell(Some("Bob"), CellDataType::String),
            ],
        ],
    };

    let result = excel_sql::dedup_sheet_on_data(&sheet, &[]).expect("Failed to deduplicate");
    // Header + 3 unique rows = 4
    assert_eq!(result.rows.len(), 4);
    assert_eq!(result.rows[0][0].value.as_deref(), Some("ID"));
}

#[test]
fn test_integration_deduplication_specific_columns() {
    let sheet = SheetData {
        name: "test_dedup_cols".to_string(),
        rows: vec![
            vec![
                make_cell(Some("ID"), CellDataType::Int),
                make_cell(Some("Name"), CellDataType::String),
                make_cell(Some("Dept"), CellDataType::String),
            ],
            vec![
                make_cell(Some("1"), CellDataType::Int),
                make_cell(Some("Alice"), CellDataType::String),
                make_cell(Some("IT"), CellDataType::String),
            ],
            vec![
                make_cell(Some("1"), CellDataType::Int),
                make_cell(Some("Alice"), CellDataType::String),
                make_cell(Some("HR"), CellDataType::String),
            ],
            vec![
                make_cell(Some("2"), CellDataType::Int),
                make_cell(Some("Bob"), CellDataType::String),
                make_cell(Some("IT"), CellDataType::String),
            ],
        ],
    };

    // Deduplicate on ID only
    let result = excel_sql::dedup_sheet_on_data(&sheet, &[0]).expect("Failed to deduplicate");
    // Header + 2 unique IDs = 3
    assert_eq!(result.rows.len(), 3);
}

#[test]
fn test_integration_engine_with_cache() {
    let cache_path = "/tmp/test_excel_sql_cache.db";

    // Clean up if exists
    let _ = std::fs::remove_file(cache_path);

    // Create engine with cache
    let mut engine =
        ExcelQueryEngine::with_cache(cache_path).expect("Failed to create engine with cache");
    assert!(engine.persistent_path.is_some());

    let sheet = create_sample_sheet(
        "cached_data",
        vec![
            vec![Some("A"), Some("1"), Some("10.0")],
            vec![Some("B"), Some("2"), Some("20.0")],
        ],
    );

    engine
        .load_without_header("cached_data", &sheet)
        .expect("Failed to load sheet");

    // Verify data is loaded
    let result = engine
        .query(r#"SELECT COUNT(*) as cnt FROM "cached_data""#)
        .expect("Failed to query");
    assert_eq!(result.row_count, 1);

    // Clean up
    let _ = std::fs::remove_file(cache_path);
}

#[test]
fn test_integration_complex_sql_query() {
    let mut engine = ExcelQueryEngine::new().expect("Failed to create engine");

    let sheet = SheetData {
        name: "complex_data".to_string(),
        rows: vec![
            vec![
                make_cell(Some("Category"), CellDataType::String),
                make_cell(Some("Value"), CellDataType::Int),
            ],
            vec![
                make_cell(Some("A"), CellDataType::String),
                make_cell(Some("10"), CellDataType::Int),
            ],
            vec![
                make_cell(Some("B"), CellDataType::String),
                make_cell(Some("20"), CellDataType::Int),
            ],
            vec![
                make_cell(Some("A"), CellDataType::String),
                make_cell(Some("30"), CellDataType::Int),
            ],
            vec![
                make_cell(Some("B"), CellDataType::String),
                make_cell(Some("40"), CellDataType::Int),
            ],
        ],
    };

    engine
        .load_with_header("complex_data", &sheet)
        .expect("Failed to load sheet");

    // Complex query: GROUP BY, HAVING, ORDER BY
    let result = engine
        .query(
            r#"SELECT Category, SUM(Value) as total, AVG(Value) as avg 
               FROM "complex_data" 
               GROUP BY Category 
               HAVING SUM(Value) > 30 
               ORDER BY total DESC"#,
        )
        .expect("Failed to execute complex query");
    assert_eq!(result.row_count, 1);
    assert_eq!(result.rows[0][0].value.as_deref(), Some("B"));
}

#[test]
fn test_integration_query_with_params() {
    let mut engine = ExcelQueryEngine::new().expect("Failed to create engine");

    let sheet = create_sample_sheet(
        "param_test",
        vec![
            vec![Some("A"), Some("1"), Some("10.0")],
            vec![Some("B"), Some("2"), Some("20.0")],
            vec![Some("C"), Some("3"), Some("30.0")],
        ],
    );

    engine
        .load_without_header("param_test", &sheet)
        .expect("Failed to load sheet");

    // Query with parameters
    let params = [duckdb::types::Value::BigInt(2)];
    let result = engine
        .query_with_params(
            r#"SELECT * FROM "param_test" WHERE "c1" > ?1 ORDER BY "c1""#,
            &params,
        )
        .expect("Failed to query with params");
    assert_eq!(result.row_count, 2);
}

#[test]
fn test_integration_empty_and_null_values() {
    let mut engine = ExcelQueryEngine::new().expect("Failed to create engine");

    let sheet = SheetData {
        name: "null_test".to_string(),
        rows: vec![
            vec![
                make_cell(Some("A"), CellDataType::String),
                make_cell(Some("B"), CellDataType::String),
            ],
            vec![
                make_cell(Some("value1"), CellDataType::String),
                make_cell(None, CellDataType::Empty),
            ],
            vec![
                make_cell(None, CellDataType::Empty),
                make_cell(Some("value2"), CellDataType::String),
            ],
            vec![
                make_cell(Some("value3"), CellDataType::String),
                make_cell(Some("value4"), CellDataType::String),
            ],
        ],
    };

    engine
        .load_with_header("null_test", &sheet)
        .expect("Failed to load sheet");

    // Query for non-null values in first column
    let result = engine
        .query(r#"SELECT * FROM "null_test" WHERE "A" IS NOT NULL"#)
        .expect("Failed to query non-null values");
    assert_eq!(result.row_count, 2);
}

#[test]
fn test_integration_session_clear_and_reuse() {
    let mut session = QuerySession::new().expect("Failed to create session");

    let sheet1 = create_sample_sheet("temp1", vec![vec![Some("A"), Some("1"), Some("10.0")]]);

    let sheet2 = create_sample_sheet("temp2", vec![vec![Some("B"), Some("2"), Some("20.0")]]);

    session
        .load_sheet("temp1", &sheet1, false)
        .expect("Failed to load sheet1");
    session
        .load_sheet("temp2", &sheet2, false)
        .expect("Failed to load sheet2");

    assert_eq!(session.list_tables().unwrap().len(), 2);

    // Clear all tables
    session.clear().expect("Failed to clear session");
    assert_eq!(session.list_tables().unwrap().len(), 0);

    // Reload and verify
    session
        .load_sheet("temp1", &sheet1, false)
        .expect("Failed to reload sheet1");
    assert_eq!(session.list_tables().unwrap().len(), 1);
}

#[test]
fn test_integration_sql_query_on_data_multiple_sheets() {
    let sheets = vec![
        SheetData {
            name: "sheet_a".to_string(),
            rows: vec![
                vec![
                    make_cell(Some("Key"), CellDataType::Int),
                    make_cell(Some("ValueA"), CellDataType::String),
                ],
                vec![
                    make_cell(Some("1"), CellDataType::Int),
                    make_cell(Some("A1"), CellDataType::String),
                ],
            ],
        },
        SheetData {
            name: "sheet_b".to_string(),
            rows: vec![
                vec![
                    make_cell(Some("Key"), CellDataType::Int),
                    make_cell(Some("ValueB"), CellDataType::String),
                ],
                vec![
                    make_cell(Some("1"), CellDataType::Int),
                    make_cell(Some("B1"), CellDataType::String),
                ],
            ],
        },
    ];

    let result = excel_sql::sql_query_on_data(&sheets, "SELECT * FROM \"sheet_a\"", true)
        .expect("Failed to query multiple sheets");
    assert_eq!(result.row_count, 1);
    assert_eq!(result.columns, vec!["Key", "ValueA"]);
}

#[test]
fn test_integration_large_dataset_performance() {
    let mut engine = ExcelQueryEngine::new().expect("Failed to create engine");

    // Create a larger dataset
    let mut rows = vec![vec![
        make_cell(Some("ID"), CellDataType::Int),
        make_cell(Some("Value"), CellDataType::Int),
    ]];

    for i in 0..1000 {
        rows.push(vec![
            make_cell(Some(&i.to_string()), CellDataType::Int),
            make_cell(Some(&(i * 2).to_string()), CellDataType::Int),
        ]);
    }

    let sheet = SheetData {
        name: "large_data".to_string(),
        rows,
    };

    engine
        .load_with_header("large_data", &sheet)
        .expect("Failed to load large sheet");

    // Query with filter and aggregation
    let result = engine
        .query(r#"SELECT COUNT(*) as cnt, SUM(Value) as sum FROM "large_data" WHERE ID > 500"#)
        .expect("Failed to query large dataset");
    assert_eq!(result.row_count, 1);

    // Parse the count value
    let count_str = result.rows[0][0].value.as_deref().unwrap();
    let count: i64 = count_str.parse().unwrap();
    assert_eq!(count, 500);
}

#[test]
fn test_integration_filter_multiple_conditions() {
    let sheet = SheetData {
        name: "multi_cond".to_string(),
        rows: vec![
            vec![
                make_cell(Some("Name"), CellDataType::String),
                make_cell(Some("Age"), CellDataType::Int),
                make_cell(Some("Salary"), CellDataType::Int),
            ],
            vec![
                make_cell(Some("Alice"), CellDataType::String),
                make_cell(Some("30"), CellDataType::Int),
                make_cell(Some("50000"), CellDataType::Int),
            ],
            vec![
                make_cell(Some("Bob"), CellDataType::String),
                make_cell(Some("25"), CellDataType::Int),
                make_cell(Some("60000"), CellDataType::Int),
            ],
            vec![
                make_cell(Some("Charlie"), CellDataType::String),
                make_cell(Some("35"), CellDataType::Int),
                make_cell(Some("70000"), CellDataType::Int),
            ],
        ],
    };

    let conditions = vec![
        FilterCondition {
            column: 1,
            operator: FilterOp::Ge,
            value: "28".to_string(),
        },
        FilterCondition {
            column: 2,
            operator: FilterOp::Lt,
            value: "65000".to_string(),
        },
    ];

    let result = excel_sql::filter_rows_on_data(&sheet, "multi_cond", &conditions, true)
        .expect("Failed to filter with multiple conditions");
    assert_eq!(result.row_count, 1);
    assert_eq!(result.rows[0][0].value.as_deref(), Some("Alice"));
}

#[test]
fn test_integration_data_type_conversion() {
    let mut engine = ExcelQueryEngine::new().expect("Failed to create engine");

    let sheet = SheetData {
        name: "types".to_string(),
        rows: vec![
            vec![
                make_cell(Some("StrVal"), CellDataType::String),
                make_cell(Some("IntVal"), CellDataType::Int),
                make_cell(Some("FloatVal"), CellDataType::Float),
                make_cell(Some("BoolVal"), CellDataType::Bool),
            ],
            vec![
                make_cell(Some("text"), CellDataType::String),
                make_cell(Some("42"), CellDataType::Int),
                make_cell(Some("3.14"), CellDataType::Float),
                make_cell(Some("true"), CellDataType::Bool),
            ],
        ],
    };

    engine
        .load_with_header("types", &sheet)
        .expect("Failed to load sheet");

    // Query and verify types are preserved
    let result = engine
        .query(r#"SELECT * FROM "types""#)
        .expect("Failed to query");
    assert_eq!(result.row_count, 1);
    assert_eq!(result.rows[0][0].data_type, CellDataType::String);
    assert_eq!(result.rows[0][1].data_type, CellDataType::Int);
    assert_eq!(result.rows[0][2].data_type, CellDataType::Float);
    assert_eq!(result.rows[0][3].data_type, CellDataType::Bool);
}
