//! Formula parser — converts formula strings into AST.

use regex::Regex;

use crate::types::*;

/// Parse a formula string into an AST.
///
/// The formula may or may not start with `=`. Trailing whitespace is ignored.
pub fn parse(formula: &str) -> Result<AstNode, ParseError> {
    let trimmed = formula.trim().strip_prefix('=').unwrap_or(formula.trim());
    let trimmed = trimmed.trim();

    if trimmed.is_empty() {
        return Err(ParseError::empty_formula());
    }

    let mut parser = Parser::new(trimmed);
    let node = parser.parse_expression()?;
    parser.skip_ws();
    if parser.pos < parser.input.len() {
        return Err(ParseError::unexpected_token(
            parser.pos,
            format!(
                "unexpected trailing characters: '{}'",
                &parser.input[parser.pos..parser.pos.min(parser.pos + 20)]
            ),
        ));
    }
    Ok(node)
}

#[derive(Debug, Clone)]
pub struct ParseError {
    pub msg: String,
    pub pos: usize,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Parse error at position {}: {}", self.pos, self.msg)
    }
}

impl std::error::Error for ParseError {}

impl ParseError {
    pub fn new(pos: usize, msg: impl Into<String>) -> Self {
        Self {
            msg: msg.into(),
            pos,
        }
    }

    pub fn empty_formula() -> Self {
        Self {
            msg: "empty formula".into(),
            pos: 0,
        }
    }

    pub fn unexpected_token(pos: usize, msg: impl Into<String>) -> Self {
        Self {
            msg: msg.into(),
            pos,
        }
    }
}

struct Parser {
    input: String,
    pos: usize,
}

impl Parser {
    fn new(input: &str) -> Self {
        Self {
            input: input.to_string(),
            pos: 0,
        }
    }

    fn skip_ws(&mut self) {
        while self.pos < self.input.len() && self.input.as_bytes()[self.pos].is_ascii_whitespace() {
            self.pos += 1;
        }
    }

    fn peek(&self) -> Option<u8> {
        if self.pos < self.input.len() {
            Some(self.input.as_bytes()[self.pos])
        } else {
            None
        }
    }

    fn peek_chars(&self, n: usize) -> Option<&str> {
        if self.pos + n <= self.input.len() {
            Some(&self.input[self.pos..self.pos + n])
        } else {
            None
        }
    }

    fn advance(&mut self) {
        self.pos += 1;
    }

    fn advance_n(&mut self, n: usize) {
        self.pos += n;
    }

    fn consume(&mut self) -> Option<u8> {
        let ch = self.peek()?;
        self.advance();
        Some(ch)
    }

    fn expect(&mut self, expected: u8) -> Result<u8, ParseError> {
        match self.peek() {
            Some(ch) if ch == expected => {
                self.advance();
                Ok(ch)
            }
            Some(ch) => Err(ParseError::unexpected_token(
                self.pos,
                format!("expected '{}', found '{}'", expected as char, ch as char),
            )),
            None => Err(ParseError::unexpected_token(
                self.pos,
                format!("expected '{}', found end of input", expected as char),
            )),
        }
    }

    // ---- Grammar entry points ----

    /// expression := comparison
    fn parse_expression(&mut self) -> Result<AstNode, ParseError> {
        self.parse_comparison()
    }

    /// comparison := concat (('=' | '<>' | '<=' | '>=' | '<' | '>') concat)*
    fn parse_comparison(&mut self) -> Result<AstNode, ParseError> {
        let mut left = self.parse_concat()?;

        loop {
            self.skip_ws();
            let op = match self.peek_chars(2) {
                Some("<>") => {
                    self.advance_n(2);
                    BinOp::Ne
                }
                Some("<=") => {
                    self.advance_n(2);
                    BinOp::Le
                }
                Some(">=") => {
                    self.advance_n(2);
                    BinOp::Ge
                }
                _ => match self.peek() {
                    Some(b'=') => {
                        self.advance();
                        BinOp::Eq
                    }
                    Some(b'<') => {
                        self.advance();
                        BinOp::Lt
                    }
                    Some(b'>') => {
                        self.advance();
                        BinOp::Gt
                    }
                    _ => break,
                },
            };

            self.skip_ws();
            let right = self.parse_concat()?;
            left = AstNode::BinaryOp {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    /// concat := add_sub (('&') add_sub)*
    fn parse_concat(&mut self) -> Result<AstNode, ParseError> {
        let mut left = self.parse_add_sub()?;

        loop {
            self.skip_ws();
            if self.peek() == Some(b'&') {
                self.advance();
                self.skip_ws();
                let right = self.parse_add_sub()?;
                left = AstNode::BinaryOp {
                    op: BinOp::Concat,
                    left: Box::new(left),
                    right: Box::new(right),
                };
            } else {
                break;
            }
        }

        Ok(left)
    }

    /// add_sub := mul_div (('+' | '-') mul_div)*
    fn parse_add_sub(&mut self) -> Result<AstNode, ParseError> {
        let mut left = self.parse_mul_div()?;

        loop {
            self.skip_ws();
            match self.peek() {
                Some(b'+') => {
                    self.advance();
                    self.skip_ws();
                    let right = self.parse_mul_div()?;
                    left = AstNode::BinaryOp {
                        op: BinOp::Add,
                        left: Box::new(left),
                        right: Box::new(right),
                    };
                }
                Some(b'-') => {
                    self.advance();
                    self.skip_ws();
                    let right = self.parse_mul_div()?;
                    left = AstNode::BinaryOp {
                        op: BinOp::Sub,
                        left: Box::new(left),
                        right: Box::new(right),
                    };
                }
                _ => break,
            }
        }

        Ok(left)
    }

    /// mul_div := pow (('*' | '/') pow)*
    fn parse_mul_div(&mut self) -> Result<AstNode, ParseError> {
        let mut left = self.parse_pow()?;

        loop {
            self.skip_ws();
            match self.peek() {
                Some(b'*') => {
                    self.advance();
                    self.skip_ws();
                    let right = self.parse_pow()?;
                    left = AstNode::BinaryOp {
                        op: BinOp::Mul,
                        left: Box::new(left),
                        right: Box::new(right),
                    };
                }
                Some(b'/') => {
                    self.advance();
                    self.skip_ws();
                    let right = self.parse_pow()?;
                    left = AstNode::BinaryOp {
                        op: BinOp::Div,
                        left: Box::new(left),
                        right: Box::new(right),
                    };
                }
                _ => break,
            }
        }

        Ok(left)
    }

    /// pow := unary ('^' unary)*
    fn parse_pow(&mut self) -> Result<AstNode, ParseError> {
        let mut left = self.parse_unary()?;

        loop {
            self.skip_ws();
            if self.peek() == Some(b'^') {
                self.advance();
                self.skip_ws();
                let right = self.parse_unary()?;
                left = AstNode::BinaryOp {
                    op: BinOp::Pow,
                    left: Box::new(left),
                    right: Box::new(right),
                };
            } else {
                break;
            }
        }

        Ok(left)
    }

    /// unary := ('-' | '+' | '@')? primary ('%')?
    fn parse_unary(&mut self) -> Result<AstNode, ParseError> {
        self.skip_ws();

        // Handle unary minus, plus, or implicit intersection
        let mut neg = false;
        let mut implicit = false;

        match self.peek() {
            Some(b'-') => {
                self.advance();
                neg = true;
            }
            Some(b'+') => {
                self.advance();
            }
            Some(b'@') => {
                self.advance();
                implicit = true;
            }
            _ => {}
        }

        self.skip_ws();
        let node = self.parse_primary()?;

        // Handle trailing percent (e.g., 50%)
        self.skip_ws();
        let node = if self.peek() == Some(b'%') {
            self.advance();
            AstNode::UnaryOp {
                op: UnaryOp::Percent,
                operand: Box::new(node),
            }
        } else {
            node
        };

        let node = if neg {
            AstNode::UnaryOp {
                op: UnaryOp::Neg,
                operand: Box::new(node),
            }
        } else if implicit {
            AstNode::UnaryOp {
                op: UnaryOp::ImplicitIntersection,
                operand: Box::new(node),
            }
        } else {
            node
        };

        Ok(node)
    }

    /// primary := number | string | cell_ref_range | function_call | '(' expression ')' | array | error
    fn parse_primary(&mut self) -> Result<AstNode, ParseError> {
        self.skip_ws();

        match self.peek() {
            Some(b'(') => {
                self.advance();
                let node = self.parse_expression()?;
                self.skip_ws();
                self.expect(b')')?;
                Ok(node)
            }
            Some(b'"') => self.parse_string(),
            Some(b'{') => self.parse_array(),
            Some(b'#') => self.parse_error_literal(),
            // Number starts with digit, decimal, or +/- (but unary +/- handled above)
            Some(ch) if ch.is_ascii_digit() || ch == b'.' => self.parse_number(),
            // Cell reference, function call, or boolean literal
            Some(ch) if ch.is_ascii_alphabetic() || ch == b'_' || ch == b'$' => {
                self.parse_identifier_or_ref()
            }
            Some(ch) => Err(ParseError::unexpected_token(
                self.pos,
                format!("unexpected character: '{}'", ch as char),
            )),
            None => Err(ParseError::unexpected_token(
                self.pos,
                "unexpected end of input",
            )),
        }
    }

    fn parse_number(&mut self) -> Result<AstNode, ParseError> {
        let start = self.pos;
        while self.pos < self.input.len()
            && (self.input.as_bytes()[self.pos].is_ascii_digit()
                || self.input.as_bytes()[self.pos] == b'.')
        {
            self.pos += 1;
        }
        // Scientific notation: 1e+10, 1E-5
        if self.pos < self.input.len()
            && (self.input.as_bytes()[self.pos] == b'e' || self.input.as_bytes()[self.pos] == b'E')
        {
            self.pos += 1;
            if self.pos < self.input.len()
                && (self.input.as_bytes()[self.pos] == b'+'
                    || self.input.as_bytes()[self.pos] == b'-')
            {
                self.pos += 1;
            }
            while self.pos < self.input.len() && self.input.as_bytes()[self.pos].is_ascii_digit() {
                self.pos += 1;
            }
        }

        let num_str = &self.input[start..self.pos];
        let value = num_str.parse::<f64>().map_err(|_| {
            ParseError::unexpected_token(start, format!("invalid number: '{}'", num_str))
        })?;
        Ok(AstNode::Number(value))
    }

    fn parse_string(&mut self) -> Result<AstNode, ParseError> {
        self.expect(b'"')?;
        let start = self.pos;
        while self.pos < self.input.len() {
            if self.input.as_bytes()[self.pos] == b'"' {
                if self.pos + 1 < self.input.len() && self.input.as_bytes()[self.pos + 1] == b'"' {
                    // Escaped quote
                    self.pos += 2;
                } else {
                    let s = self.input[start..self.pos].to_string();
                    self.advance(); // consume closing "
                    return Ok(AstNode::String(s.replace("\"\"", "\"")));
                }
            } else {
                self.pos += 1;
            }
        }
        Err(ParseError::unexpected_token(
            self.pos,
            "unterminated string",
        ))
    }

    /// Parse an array literal: {1,2;3,4}
    fn parse_array(&mut self) -> Result<AstNode, ParseError> {
        self.expect(b'{')?;
        let mut rows: Vec<Vec<AstNode>> = Vec::new();
        let mut current_row: Vec<AstNode> = Vec::new();

        loop {
            self.skip_ws();
            match self.peek() {
                Some(b'}') => {
                    self.advance();
                    if !current_row.is_empty() {
                        rows.push(current_row);
                    }
                    return Ok(AstNode::Array(rows));
                }
                Some(b';') => {
                    self.advance();
                    rows.push(std::mem::take(&mut current_row));
                }
                Some(b',') => {
                    self.advance();
                }
                _ => {
                    let node = self.parse_expression()?;
                    current_row.push(node);
                }
            }
        }
    }

    fn parse_error_literal(&mut self) -> Result<AstNode, ParseError> {
        let start = self.pos;
        self.advance(); // consume '#'
        let name_start = self.pos;

        while self.pos < self.input.len()
            && (self.input.as_bytes()[self.pos].is_ascii_alphanumeric()
                || self.input.as_bytes()[self.pos] == b'/'
                || self.input.as_bytes()[self.pos] == b'?'
                || self.input.as_bytes()[self.pos] == b'!'
                || self.input.as_bytes()[self.pos] == b'-')
        {
            self.pos += 1;
        }

        let error_name = &self.input[name_start..self.pos];
        let full = &self.input[start..self.pos];

        let error = match error_name.to_uppercase().as_str() {
            "DIV/0!" => ExcelError::Div0,
            "N/A" => ExcelError::NA,
            "NAME?" => ExcelError::Name,
            "NULL!" => ExcelError::Null,
            "NUM!" => ExcelError::Num,
            "REF!" => ExcelError::Ref,
            "VALUE!" => ExcelError::Value,
            "SPILL!" => ExcelError::Spill,
            "CALC!" => ExcelError::Calc,
            _ => ExcelError::Value_(full.to_string()),
        };

        Ok(AstNode::Error(error))
    }

    /// Parse an identifier that could be:
    /// - A boolean literal: TRUE, FALSE
    /// - A function call: SUM(1,2)
    /// - A cell/range reference: A1, $B$2, Sheet2!C3, A1:B10
    fn parse_identifier_or_ref(&mut self) -> Result<AstNode, ParseError> {
        let token = self.read_identifier_token();

        // Check for boolean literals
        if token.eq_ignore_ascii_case("true") {
            // Peek ahead: if next is '(', it's a function call (though unlikely for "true")
            self.skip_ws();
            if self.peek() == Some(b'(') {
                return self.parse_function_call(token);
            }
            return Ok(AstNode::Bool(true));
        }
        if token.eq_ignore_ascii_case("false") {
            self.skip_ws();
            if self.peek() == Some(b'(') {
                return self.parse_function_call(token);
            }
            return Ok(AstNode::Bool(false));
        }

        // Check for function call
        self.skip_ws();
        if self.peek() == Some(b'(') {
            return self.parse_function_call(token);
        }

        // Try to parse as cell/range reference
        self.parse_cell_or_range_ref(&token)
    }

    fn parse_function_call(&mut self, name: String) -> Result<AstNode, ParseError> {
        self.expect(b'(')?;
        let mut args = Vec::new();

        loop {
            self.skip_ws();
            if self.peek() == Some(b')') {
                self.advance();
                break;
            }

            // Check for empty argument (e.g., SUM(,1))
            if self.peek() == Some(b',') {
                args.push(AstNode::EmptyArg);
                self.advance();
                continue;
            }

            let arg = self.parse_expression()?;
            args.push(arg);

            self.skip_ws();
            match self.peek() {
                Some(b',') => {
                    self.advance();
                }
                Some(b')') => {
                    self.advance();
                    break;
                }
                Some(ch) => {
                    return Err(ParseError::unexpected_token(
                        self.pos,
                        format!("expected ',' or ')', found '{}'", ch as char),
                    ));
                }
                None => {
                    return Err(ParseError::unexpected_token(
                        self.pos,
                        "expected ',' or ')', found end of input",
                    ));
                }
            }
        }

        Ok(AstNode::Function {
            name: name.to_uppercase(),
            args,
        })
    }

    fn read_identifier_token(&mut self) -> String {
        let start = self.pos;
        // Regex: sheet names can contain letters, numbers, underscores, spaces (in quotes)
        // For simplicity, read until we hit a non-identifier char
        while self.pos < self.input.len() {
            let ch = self.input.as_bytes()[self.pos];
            if ch.is_ascii_alphanumeric()
                || ch == b'_'
                || ch == b'$'
                || ch == b'!'
                || ch == b':'
                || ch == b'\''
                || ch == b'.'
            {
                self.pos += 1;
            } else {
                break;
            }
        }
        self.input[start..self.pos].to_string()
    }

    fn parse_cell_or_range_ref(&mut self, token: &str) -> Result<AstNode, ParseError> {
        // Try to match: [Sheet!] ColPart RowPart [ : ColPart RowPart ]
        let re =
            Regex::new(r"^(?:'?([^!']+)'?!)?(\$?[A-Za-z]{1,3})(\$?\d+)$").expect("valid regex");
        let range_re = Regex::new(
            r"^(?:'?([^!']+)'?!)?(\$?[A-Za-z]{1,3})(\$?\d+):(\$?[A-Za-z]{1,3})(\$?\d+)$",
        )
        .expect("valid regex");

        if let Some(caps) = range_re.captures(token) {
            let sheet = caps.get(1).map(|m| m.as_str().to_string());
            let col1_str = caps.get(2).expect("col1").as_str();
            let row1_str = caps.get(3).expect("row1").as_str();
            let col2_str = caps.get(4).expect("col2").as_str();
            let row2_str = caps.get(5).expect("row2").as_str();

            let (col1, col1_abs) = parse_col_part(col1_str);
            let (col2, col2_abs) = parse_col_part(col2_str);
            let (row1, row1_abs) = parse_row_part(row1_str);
            let (row2, row2_abs) = parse_row_part(row2_str);

            return Ok(AstNode::Range(RangeReference {
                start: CellReference {
                    col: col1,
                    col_absolute: col1_abs,
                    row: row1,
                    row_absolute: row1_abs,
                    sheet: sheet.clone(),
                    original: format!(
                        "{}{}{}",
                        sheet
                            .as_deref()
                            .map(|s| format!("{}!", s))
                            .unwrap_or_default(),
                        col1_str,
                        row1_str
                    ),
                },
                end: CellReference {
                    col: col2,
                    col_absolute: col2_abs,
                    row: row2,
                    row_absolute: row2_abs,
                    sheet: sheet.clone(),
                    original: format!("{}{}", col2_str, row2_str),
                },
                original: token.to_string(),
            }));
        }

        if let Some(caps) = re.captures(token) {
            let sheet = caps.get(1).map(|m| m.as_str().to_string());
            let col_str = caps.get(2).expect("col").as_str();
            let row_str = caps.get(3).expect("row").as_str();

            let (col, col_abs) = parse_col_part(col_str);
            let (row, row_abs) = parse_row_part(row_str);

            return Ok(AstNode::CellRef(CellReference {
                col,
                col_absolute: col_abs,
                row,
                row_absolute: row_abs,
                sheet,
                original: token.to_string(),
            }));
        }

        // If we reach here, the token couldn't be parsed as a cell ref
        // It might be a named range / defined name
        // For now, treat it as a name error
        Ok(AstNode::Error(ExcelError::Name))
    }
}

fn parse_col_part(s: &str) -> (u32, bool) {
    let absolute = s.starts_with('$');
    let letters = if absolute { &s[1..] } else { s };
    let col = col_letters_to_index(letters);
    (col, absolute)
}

fn parse_row_part(s: &str) -> (u32, bool) {
    let absolute = s.starts_with('$');
    let digits = if absolute { &s[1..] } else { s };
    let row: u32 = digits.parse().unwrap_or(0);
    (row.saturating_sub(1), absolute) // 1-based to 0-based
}

fn col_letters_to_index(s: &str) -> u32 {
    let mut result: u32 = 0;
    for ch in s.bytes() {
        if ch.is_ascii_uppercase() {
            result = result * 26 + (ch - b'A') as u32 + 1;
        } else if ch.is_ascii_lowercase() {
            result = result * 26 + (ch - b'a') as u32 + 1;
        }
    }
    result.saturating_sub(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_number() {
        let node = parse("42").unwrap();
        assert_eq!(node, AstNode::Number(42.0));

        let node = parse("3.14").unwrap();
        assert_eq!(node, AstNode::Number(3.14));

        let node = parse("1e3").unwrap();
        assert_eq!(node, AstNode::Number(1000.0));
    }

    #[test]
    fn test_parse_cell_ref() {
        let node = parse("A1").unwrap();
        assert!(
            matches!(node, AstNode::CellRef(ref r) if r.col == 0 && r.row == 0 && !r.col_absolute && !r.row_absolute)
        );

        let node = parse("$C$2").unwrap();
        assert!(
            matches!(node, AstNode::CellRef(ref r) if r.col == 2 && r.row == 1 && r.col_absolute && r.row_absolute)
        );

        let node = parse("Sheet2!B3").unwrap();
        assert!(matches!(node, AstNode::CellRef(ref r) if r.sheet.as_deref() == Some("Sheet2")));
    }

    #[test]
    fn test_parse_range() {
        let node = parse("A1:B10").unwrap();
        assert!(matches!(node, AstNode::Range(_)));
    }

    #[test]
    fn test_parse_simple_add() {
        let node = parse("1+2").unwrap();
        assert!(matches!(node, AstNode::BinaryOp { op: BinOp::Add, .. }));
    }

    #[test]
    fn test_parse_precedence() {
        // 1 + 2 * 3 should parse as 1 + (2 * 3)
        let node = parse("1+2*3").unwrap();
        match node {
            AstNode::BinaryOp {
                op: BinOp::Add,
                right,
                ..
            } => {
                assert!(matches!(*right, AstNode::BinaryOp { op: BinOp::Mul, .. }));
            }
            _ => panic!("expected Add with Mul right operand"),
        }
    }

    #[test]
    fn test_parse_parens() {
        let node = parse("(1+2)*3").unwrap();
        match node {
            AstNode::BinaryOp {
                op: BinOp::Mul,
                left,
                ..
            } => {
                assert!(matches!(*left, AstNode::BinaryOp { op: BinOp::Add, .. }));
            }
            _ => panic!("expected Mul with Add left operand"),
        }
    }

    #[test]
    fn test_parse_function_call() {
        let node = parse("SUM(1,2,3)").unwrap();
        assert!(matches!(node, AstNode::Function { ref name, .. } if name == "SUM"));
    }

    #[test]
    fn test_parse_unary_neg() {
        let node = parse("-5").unwrap();
        assert!(matches!(
            node,
            AstNode::UnaryOp {
                op: UnaryOp::Neg,
                ..
            }
        ));
    }

    #[test]
    fn test_parse_string() {
        let node = parse("\"hello\"").unwrap();
        assert_eq!(node, AstNode::String("hello".to_string()));
    }

    #[test]
    fn test_parse_bool() {
        assert_eq!(parse("TRUE").unwrap(), AstNode::Bool(true));
        assert_eq!(parse("FALSE").unwrap(), AstNode::Bool(false));
    }

    #[test]
    fn test_parse_error() {
        let node = parse("#DIV/0!").unwrap();
        assert!(matches!(node, AstNode::Error(ExcelError::Div0)));
    }

    #[test]
    fn test_parse_comparison() {
        let node = parse("A1>10").unwrap();
        assert!(matches!(node, AstNode::BinaryOp { op: BinOp::Gt, .. }));
    }

    #[test]
    fn test_parse_concat() {
        let node = parse("\"Hello\"&\" \"&\"World\"").unwrap();
        assert!(matches!(
            node,
            AstNode::BinaryOp {
                op: BinOp::Concat,
                ..
            }
        ));
    }

    #[test]
    fn test_parse_range_in_function() {
        let node = parse("SUM(A1:B10)").unwrap();
        assert!(matches!(node, AstNode::Function { ref name, .. } if name == "SUM"));
    }
}
