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
