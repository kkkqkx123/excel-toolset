use crate::types::*;

/// Token types for the calculated field expression parser.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Token {
    Number(f64),
    Identifier(String),
    Plus,
    Minus,
    Star,
    Slash,
    LParen,
    RParen,
}

/// AST node for a calculated field expression.
#[derive(Debug, Clone)]
pub(crate) enum Expr {
    Number(f64),
    Field(String),
    Binary(Box<Expr>, BinOp, Box<Expr>),
}

#[derive(Debug, Clone)]
pub(crate) enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
}

/// Tokenize a formula string like "=Revenue - Cost" or "Price * (Qty + 1)".
pub(crate) fn tokenize(formula: &str) -> Result<Vec<Token>> {
    let s = formula.trim();
    let s = s.strip_prefix('=').unwrap_or(s).trim();
    let mut tokens: Vec<Token> = Vec::new();
    let chars: Vec<char> = s.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        match c {
            '+' => {
                tokens.push(Token::Plus);
                i += 1;
            }
            '-' => {
                tokens.push(Token::Minus);
                i += 1;
            }
            '*' => {
                tokens.push(Token::Star);
                i += 1;
            }
            '/' => {
                tokens.push(Token::Slash);
                i += 1;
            }
            '(' => {
                tokens.push(Token::LParen);
                i += 1;
            }
            ')' => {
                tokens.push(Token::RParen);
                i += 1;
            }
            _ if c.is_ascii_digit()
                || (c == '.' && i + 1 < len && chars[i + 1].is_ascii_digit()) =>
            {
                let start = i;
                while i < len && (chars[i].is_ascii_digit() || chars[i] == '.') {
                    i += 1;
                }
                let num_str: String = chars[start..i].iter().collect();
                let num = num_str
                    .parse::<f64>()
                    .map_err(|_| AppError::InvalidInput(format!("Invalid number: {}", num_str)))?;
                tokens.push(Token::Number(num));
            }
            _ if c.is_alphabetic() || c == '_' => {
                let start = i;
                while i < len && (chars[i].is_alphanumeric() || chars[i] == '_' || chars[i] == '.')
                {
                    i += 1;
                }
                let ident: String = chars[start..i].iter().collect();
                tokens.push(Token::Identifier(ident));
            }
            _ => {
                return Err(AppError::InvalidInput(format!(
                    "Unexpected character '{}' in formula: {}",
                    c, formula
                )));
            }
        }
    }

    Ok(tokens)
}

/// Get operator precedence for precedence-climbing parser.
pub(crate) fn precedence(token: &Token) -> u8 {
    match token {
        Token::Plus | Token::Minus => 1,
        Token::Star | Token::Slash => 2,
        _ => 0,
    }
}

/// Parse expression with precedence climbing.
pub(crate) fn parse_expr(tokens: &[Token], pos: usize, min_prec: u8) -> Result<(Expr, usize)> {
    let (mut left, mut pos) = parse_prefix(tokens, pos)?;

    while pos < tokens.len() {
        let prec = precedence(&tokens[pos]);
        if prec < min_prec {
            break;
        }
        let op = match &tokens[pos] {
            Token::Plus => BinOp::Add,
            Token::Minus => BinOp::Sub,
            Token::Star => BinOp::Mul,
            Token::Slash => BinOp::Div,
            _ => break,
        };
        pos += 1;
        let (right, new_pos) = parse_expr(tokens, pos, prec + 1)?;
        left = Expr::Binary(Box::new(left), op, Box::new(right));
        pos = new_pos;
    }

    Ok((left, pos))
}

/// Parse a prefix expression (number, field name, parenthesized, or unary minus).
pub(crate) fn parse_prefix(tokens: &[Token], pos: usize) -> Result<(Expr, usize)> {
    if pos >= tokens.len() {
        return Err(AppError::InvalidInput(
            "Unexpected end of formula".to_string(),
        ));
    }
    match &tokens[pos] {
        Token::Number(n) => Ok((Expr::Number(*n), pos + 1)),
        Token::Identifier(name) => Ok((Expr::Field(name.clone()), pos + 1)),
        Token::LParen => {
            let (inner, pos) = parse_expr(tokens, pos + 1, 0)?;
            if pos >= tokens.len() || tokens[pos] != Token::RParen {
                return Err(AppError::InvalidInput(
                    "Missing closing parenthesis in formula".to_string(),
                ));
            }
            Ok((inner, pos + 1))
        }
        Token::Minus => {
            // Unary minus: parse with higher precedence
            let (inner, pos) = parse_expr(tokens, pos + 1, 3)?;
            Ok((
                Expr::Binary(Box::new(Expr::Number(0.0)), BinOp::Sub, Box::new(inner)),
                pos,
            ))
        }
        _ => Err(AppError::InvalidInput(format!(
            "Unexpected token in formula: {:?}",
            tokens[pos]
        ))),
    }
}

/// Parse a complete formula string into an AST.
pub(crate) fn parse_expression(formula: &str) -> Result<Expr> {
    let tokens = tokenize(formula)?;
    if tokens.is_empty() {
        return Err(AppError::InvalidInput("Empty formula".to_string()));
    }
    let (expr, pos) = parse_expr(&tokens, 0, 0)?;
    if pos < tokens.len() {
        return Err(AppError::InvalidInput(format!(
            "Unexpected trailing tokens in formula: {:?}",
            &tokens[pos..]
        )));
    }
    Ok(expr)
}

/// Evaluate a parsed expression against a single data row.
pub(crate) fn evaluate_expr(expr: &Expr, headers: &[String], row: &[CellData]) -> Result<f64> {
    match expr {
        Expr::Number(n) => Ok(*n),
        Expr::Field(name) => {
            let col_idx = headers.iter().position(|h| h == name).ok_or_else(|| {
                AppError::InvalidInput(format!("Field '{}' not found in column headers", name))
            })?;
            cell_value_to_f64_idx(row, col_idx).ok_or_else(|| {
                AppError::InvalidInput(format!(
                    "Field '{}' contains a non-numeric value in source data",
                    name
                ))
            })
        }
        Expr::Binary(left, op, right) => {
            let l = evaluate_expr(left, headers, row)?;
            let r = evaluate_expr(right, headers, row)?;
            match op {
                BinOp::Add => Ok(l + r),
                BinOp::Sub => Ok(l - r),
                BinOp::Mul => Ok(l * r),
                BinOp::Div => {
                    if r == 0.0 {
                        Err(AppError::InvalidInput(
                            "Division by zero in calculated field formula".to_string(),
                        ))
                    } else {
                        Ok(l / r)
                    }
                }
            }
        }
    }
}

/// Get cell value as f64 by usize index.
pub(crate) fn cell_value_to_f64_idx(row: &[CellData], col: usize) -> Option<f64> {
    row.get(col)
        .and_then(|c| c.value.as_ref())
        .and_then(|v| v.parse::<f64>().ok())
}

/// Process calculated fields: parse formulas, evaluate per row, and append results
/// as new columns to headers and data_rows.
/// Returns the mapping of calculated field name to its new column index.
pub(crate) fn process_calculated_fields(
    config: &PivotTableConfig,
    headers: &mut Vec<String>,
    data_rows: &mut [Vec<CellData>],
) -> Result<Vec<(String, u16)>> {
    let mut calc_cols: Vec<(String, u16)> = Vec::new();

    for cf in &config.calculated_fields {
        // Name conflict detection
        if headers.contains(&cf.name) {
            return Err(AppError::InvalidInput(format!(
                "Calculated field name '{}' conflicts with an existing column header",
                cf.name
            )));
        }

        // Parse expression (use current headers for field name resolution)
        let expr = parse_expression(&cf.formula)?;

        // Evaluate for each row and append result
        let col_idx = headers.len() as u16;
        for row in data_rows.iter_mut() {
            let val = evaluate_expr(&expr, headers, row)?;
            row.push(CellData {
                value: Some(format!("{}", val)),
                data_type: CellDataType::Float,
                formula: None,
            });
        }

        headers.push(cf.name.clone());
        calc_cols.push((cf.name.clone(), col_idx));
    }

    Ok(calc_cols)
}
