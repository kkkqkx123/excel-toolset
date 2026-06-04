pub fn sanitize_column_name(name: &str) -> String {
    let sanitized: String = name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if sanitized.is_empty() || sanitized.starts_with(|c: char| c.is_ascii_digit()) {
        format!("col_{}", sanitized)
    } else {
        sanitized
    }
}

#[expect(dead_code)]
pub fn validate_column_index(col: u16, max_cols: usize) -> crate::SqlResult<()> {
    if (col as usize) < max_cols {
        Ok(())
    } else {
        Err(excel_types::AppError::InvalidArgument(format!(
            "Column index {} out of bounds (max {})",
            col,
            max_cols.saturating_sub(1)
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_column_name_keeps_alphanumeric_and_underscore() {
        assert_eq!(sanitize_column_name("hello"), "hello");
        assert_eq!(sanitize_column_name("column_1"), "column_1");
        assert_eq!(sanitize_column_name("A"), "A");
        assert_eq!(sanitize_column_name("_private"), "_private");
    }

    #[test]
    fn test_sanitize_column_name_replaces_special_chars() {
        assert_eq!(sanitize_column_name("col-umn"), "col_umn");
        assert_eq!(sanitize_column_name("col umn"), "col_umn");
        assert_eq!(sanitize_column_name("col.umn"), "col_umn");
        assert_eq!(sanitize_column_name("a!b@c#"), "a_b_c_");
    }

    #[test]
    fn test_sanitize_column_name_empty_or_starts_with_digit() {
        assert_eq!(sanitize_column_name(""), "col_");
        assert_eq!(sanitize_column_name("123abc"), "col_123abc");
        assert_eq!(sanitize_column_name("0"), "col_0");
    }

    #[test]
    fn test_validate_column_index_valid() {
        assert!(validate_column_index(0, 5).is_ok());
        assert!(validate_column_index(4, 5).is_ok());
    }

    #[test]
    fn test_validate_column_index_out_of_bounds() {
        assert!(validate_column_index(5, 5).is_err());
        assert!(validate_column_index(10, 5).is_err());
    }
}
