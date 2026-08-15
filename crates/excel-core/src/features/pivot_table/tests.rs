use crate::types::*;

use super::*;

#[test]
fn test_date_grouping_year() {
    let grouping = DateGrouping {
        column: 0,
        by_year: true,
        by_quarter: false,
        by_month: false,
        by_day: false,
    };
    assert_eq!(
        group_date_value("2024-03-15", &grouping),
        Some("2024".to_string())
    );
}

#[test]
fn test_date_grouping_year_quarter() {
    let grouping = DateGrouping {
        column: 0,
        by_year: true,
        by_quarter: true,
        by_month: false,
        by_day: false,
    };
    assert_eq!(
        group_date_value("2024-03-15", &grouping),
        Some("2024-Q1".to_string())
    );
}

#[test]
fn test_date_grouping_invalid() {
    let grouping = DateGrouping {
        column: 0,
        by_year: true,
        by_quarter: false,
        by_month: false,
        by_day: false,
    };
    assert_eq!(group_date_value("not-a-date", &grouping), None);
}
