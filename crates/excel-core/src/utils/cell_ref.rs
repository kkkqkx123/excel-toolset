use crate::types::{AppError, Result};

/// Parse an A1-style cell reference like "A1" to 0-indexed (row, col).
/// "A1" -> (0, 0), "B3" -> (2, 1), "AA1" -> (0, 26)
pub fn parse_cell_ref(ref_str: &str) -> Result<(u32, u16)> {
    let ref_str = ref_str.trim();
    if ref_str.is_empty() {
        return Err(AppError::InvalidCellRef("Empty cell reference".into()));
    }

    let col_end = ref_str
        .find(|c: char| c.is_ascii_digit())
        .ok_or_else(|| AppError::InvalidCellRef(format!("Invalid cell reference: {}", ref_str)))?;

    let col_part = &ref_str[..col_end];
    let row_part = &ref_str[col_end..];

    if col_part.is_empty() || row_part.is_empty() {
        return Err(AppError::InvalidCellRef(format!(
            "Invalid cell reference: {}",
            ref_str
        )));
    }

    let col = col_to_index(col_part)?;
    let row: u32 = row_part
        .parse()
        .map_err(|_| AppError::InvalidCellRef(format!("Invalid row number: {}", row_part)))?;

    Ok((row.saturating_sub(1), col))
}

/// Parse a range like "A1:C3" to 0-indexed (row1, col1, row2, col2).
pub fn parse_range(range_str: &str) -> Result<(u32, u16, u32, u16)> {
    let range_str = range_str.trim();
    let parts: Vec<&str> = range_str.split(':').collect();
    if parts.len() != 2 {
        return Err(AppError::InvalidRange(format!(
            "Invalid range: {}",
            range_str
        )));
    }

    let (r1, c1) = parse_cell_ref(parts[0])?;
    let (r2, c2) = parse_cell_ref(parts[1])?;

    Ok((r1, c1, r2, c2))
}

/// Convert column letter(s) to 0-indexed column number.
pub fn col_to_index(col: &str) -> Result<u16> {
    let col = col.trim().to_uppercase();
    if col.is_empty() {
        return Err(AppError::InvalidCellRef("Empty column reference".into()));
    }

    let mut result: u16 = 0;
    for c in col.chars() {
        if !c.is_ascii_uppercase() {
            return Err(AppError::InvalidCellRef(format!(
                "Invalid column character: {}",
                c
            )));
        }
        result = result
            .checked_mul(26)
            .and_then(|v| v.checked_add((c as u16) - ('A' as u16) + 1))
            .ok_or_else(|| AppError::InvalidCellRef(format!("Column overflow: {}", col)))?;
    }

    Ok(result - 1)
}

/// Convert 0-indexed column number to A1-style column letters.
pub fn index_to_col(mut idx: u16) -> String {
    let mut result = String::new();
    loop {
        let rem = idx % 26;
        let c = ('A' as u16 + rem) as u32;
        let ch = char::from_u32(c).unwrap_or('?');
        result.insert(0, ch);
        if idx < 26 {
            break;
        }
        idx = idx / 26 - 1;
    }
    result
}

/// Format a (row, col) pair as an A1-style cell reference (0-indexed input).
pub fn format_cell_ref(row: u32, col: u16) -> String {
    format!("{}{}", index_to_col(col), row + 1)
}

/// Format a range from (row1, col1, row2, col2) to "A1:C3" style.
pub fn format_range(r1: u32, c1: u16, r2: u32, c2: u16) -> String {
    format!("{}:{}", format_cell_ref(r1, c1), format_cell_ref(r2, c2))
}

/// Parse a range into normalized (row_start, row_end, col_start, col_end).
pub fn parse_range_normalized(range_str: &str) -> Result<(u32, u32, u16, u16)> {
    let (r1, c1, r2, c2) = parse_range(range_str)?;
    Ok((r1.min(r2), r1.max(r2), c1.min(c2), c1.max(c2)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_col_to_index() {
        assert_eq!(col_to_index("A").unwrap(), 0);
        assert_eq!(col_to_index("Z").unwrap(), 25);
        assert_eq!(col_to_index("AA").unwrap(), 26);
        assert_eq!(col_to_index("AZ").unwrap(), 51);
        assert_eq!(col_to_index("BA").unwrap(), 52);
    }

    #[test]
    fn test_index_to_col() {
        assert_eq!(index_to_col(0), "A");
        assert_eq!(index_to_col(25), "Z");
        assert_eq!(index_to_col(26), "AA");
        assert_eq!(index_to_col(51), "AZ");
        assert_eq!(index_to_col(52), "BA");
    }

    #[test]
    fn test_roundtrip() {
        for i in 0..1000 {
            let col_str = index_to_col(i);
            let idx = col_to_index(&col_str).unwrap();
            assert_eq!(idx, i);
        }
    }

    #[test]
    fn test_parse_cell_ref() {
        let (row, col) = parse_cell_ref("A1").unwrap();
        assert_eq!(row, 0);
        assert_eq!(col, 0);
        let (row, col) = parse_cell_ref("B3").unwrap();
        assert_eq!(row, 2);
        assert_eq!(col, 1);
        let (row, col) = parse_cell_ref("AA10").unwrap();
        assert_eq!(row, 9);
        assert_eq!(col, 26);
    }

    #[test]
    fn test_parse_range() {
        let (r1, c1, r2, c2) = parse_range("A1:C3").unwrap();
        assert_eq!((r1, c1, r2, c2), (0, 0, 2, 2));
    }

    #[test]
    fn test_parse_range_normalized() {
        let (rs, re, cs, ce) = parse_range_normalized("C3:A1").unwrap();
        assert_eq!((rs, re, cs, ce), (0, 2, 0, 2));
    }

    #[test]
    fn test_format_cell_ref() {
        assert_eq!(format_cell_ref(0, 0), "A1");
        assert_eq!(format_cell_ref(2, 1), "B3");
    }

    #[test]
    fn test_invalid_ref() {
        assert!(parse_cell_ref("").is_err());
        assert!(col_to_index("").is_err());
        assert!(col_to_index("1").is_err());
    }
}
