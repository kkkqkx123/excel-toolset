//! Recursive AST evaluator.

use std::cmp::Ordering;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use excel_types::CellValue;

use crate::engine::DataProvider;
use crate::types::*;

#[derive(Debug, Clone)]
pub struct EvalError {
    pub msg: String,
}

impl std::fmt::Display for EvalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Evaluation error: {}", self.msg)
    }
}

impl std::error::Error for EvalError {}

impl EvalError {
    pub fn new(msg: impl Into<String>) -> Self {
        Self { msg: msg.into() }
    }

    pub fn parse(msg: impl Into<String>) -> Self {
        Self { msg: msg.into() }
    }
}

pub type EvalResult<T> = std::result::Result<T, EvalError>;

/// Recursive evaluator for formula AST nodes.
pub struct Evaluator<'a, P: DataProvider> {
    provider: Arc<P>,
    function_registry: &'a HashMap<
        String,
        Arc<dyn Fn(&[CellValue], &dyn DataProvider) -> CellValue + Send + Sync>,
    >,
    eval_stack: &'a Mutex<Vec<String>>,
    /// Current sheet context for resolving unqualified references.
    current_sheet: String,
}

impl<'a, P: DataProvider> Evaluator<'a, P> {
    pub fn new(
        provider: Arc<P>,
        function_registry: &'a HashMap<
            String,
            Arc<dyn Fn(&[CellValue], &dyn DataProvider) -> CellValue + Send + Sync>,
        >,
        eval_stack: &'a Mutex<Vec<String>>,
    ) -> Self {
        Self {
            provider,
            function_registry,
            eval_stack,
            current_sheet: String::new(),
        }
    }

    /// Access the inner data provider.
    pub(crate) fn data_provider(&self) -> &Arc<P> {
        &self.provider
    }

    /// Evaluate an AST node in the context of a given sheet.
    pub fn evaluate(&mut self, sheet: &str, node: &AstNode) -> EvalResult<CellValue> {
        self.current_sheet = sheet.to_string();
        self.eval_node(node)
    }

    fn eval_node(&self, node: &AstNode) -> EvalResult<CellValue> {
        match node {
            AstNode::Number(n) => Ok(CellValue::Number(*n)),
            AstNode::String(s) => Ok(CellValue::String(s.clone())),
            AstNode::Bool(b) => Ok(CellValue::Bool(*b)),
            AstNode::EmptyArg => Ok(CellValue::Empty),
            AstNode::Error(e) => Ok(CellValue::Error(e.as_str().to_string())),
            AstNode::CellRef(r) => self.eval_cell_ref(r),
            AstNode::Range(range) => {
                // A bare range reference is unusual. Try implicit intersection or return first cell.
                self.eval_cell_ref(&range.start)
            }
            AstNode::BinaryOp { op, left, right } => self.eval_binary(*op, left, right),
            AstNode::UnaryOp { op, operand } => self.eval_unary(*op, operand),
            AstNode::Function { name, args } => self.eval_function(name, args),
            AstNode::Array(rows) => {
                // Return first element for scalar evaluation
                if let Some(first_row) = rows.first() {
                    if let Some(first_cell) = first_row.first() {
                        return self.eval_node(first_cell);
                    }
                }
                Ok(CellValue::Empty)
            }
        }
    }

    fn eval_cell_ref(&self, r: &CellReference) -> EvalResult<CellValue> {
        let sheet = r.sheet.as_deref().unwrap_or(&self.current_sheet);
        let cell_key = format!("{}!{}", sheet, r.original);

        // Circular reference detection
        {
            let mut stack = self.eval_stack.lock().expect("eval_stack lock");
            if stack.contains(&cell_key) {
                return Ok(CellValue::Error("#CIRCULAR!".into()));
            }
            stack.push(cell_key.clone());
        }

        let result = self
            .provider
            .get_cell(sheet, r.row, r.col)
            .unwrap_or(CellValue::Empty);

        // Pop from stack
        {
            let mut stack = self.eval_stack.lock().expect("eval_stack lock");
            stack.pop();
        }

        Ok(result)
    }

    fn eval_binary(&self, op: BinOp, left: &AstNode, right: &AstNode) -> EvalResult<CellValue> {
        // Handle range construction: A1:B10 is parsed as a range, not a binary op.
        // But if we somehow get : as a binary op, construct a range.
        match op {
            BinOp::Add => self.eval_arithmetic(op, left, right, |a, b| a + b),
            BinOp::Sub => self.eval_arithmetic(op, left, right, |a, b| a - b),
            BinOp::Mul => self.eval_arithmetic(op, left, right, |a, b| a * b),
            BinOp::Div => {
                let left_val = self.eval_node(left)?;
                let right_val = self.eval_node(right)?;
                let (l, r) = (to_number(&left_val), to_number(&right_val));
                match (l, r) {
                    (Some(a), Some(b)) => {
                        if b == 0.0 {
                            Ok(CellValue::Error("#DIV/0!".into()))
                        } else {
                            Ok(CellValue::Number(a / b))
                        }
                    }
                    _ => Ok(CellValue::Error("#VALUE!".into())),
                }
            }
            BinOp::Pow => {
                let left_val = self.eval_node(left)?;
                let right_val = self.eval_node(right)?;
                let (l, r) = (to_number(&left_val), to_number(&right_val));
                match (l, r) {
                    (Some(a), Some(b)) => Ok(CellValue::Number(a.powf(b))),
                    _ => Ok(CellValue::Error("#VALUE!".into())),
                }
            }
            BinOp::Concat => {
                let left_val = self.eval_node(left)?;
                let right_val = self.eval_node(right)?;
                let l = cell_value_to_string(&left_val);
                let r = cell_value_to_string(&right_val);
                Ok(CellValue::String(format!("{}{}", l, r)))
            }
            BinOp::Eq => self.eval_comparison(left, right, PartialEq::eq),
            BinOp::Ne => self.eval_comparison(left, right, |a, b| !a.eq(b)),
            BinOp::Lt => self.eval_comparison(left, right, |a, b| {
                partial_cmp_cell_values(a, b) == Some(Ordering::Less)
            }),
            BinOp::Gt => self.eval_comparison(left, right, |a, b| {
                partial_cmp_cell_values(a, b) == Some(Ordering::Greater)
            }),
            BinOp::Le => self.eval_comparison(left, right, |a, b| {
                matches!(
                    partial_cmp_cell_values(a, b),
                    Some(Ordering::Less | Ordering::Equal)
                )
            }),
            BinOp::Ge => self.eval_comparison(left, right, |a, b| {
                matches!(
                    partial_cmp_cell_values(a, b),
                    Some(Ordering::Greater | Ordering::Equal)
                )
            }),
        }
    }

    fn eval_arithmetic<F>(
        &self,
        _op: BinOp,
        left: &AstNode,
        right: &AstNode,
        func: F,
    ) -> EvalResult<CellValue>
    where
        F: Fn(f64, f64) -> f64,
    {
        let left_val = self.eval_node(left)?;
        let right_val = self.eval_node(right)?;
        let (l, r) = (to_number(&left_val), to_number(&right_val));
        match (l, r) {
            (Some(a), Some(b)) => Ok(CellValue::Number(func(a, b))),
            _ => Ok(CellValue::Error("#VALUE!".into())),
        }
    }

    fn eval_comparison<F>(&self, left: &AstNode, right: &AstNode, func: F) -> EvalResult<CellValue>
    where
        F: Fn(&CellValue, &CellValue) -> bool,
    {
        let left_val = self.eval_node(left)?;
        let right_val = self.eval_node(right)?;
        Ok(CellValue::Bool(func(&left_val, &right_val)))
    }

    fn eval_unary(&self, op: UnaryOp, operand: &AstNode) -> EvalResult<CellValue> {
        match op {
            UnaryOp::Neg => {
                let val = self.eval_node(operand)?;
                match to_number(&val) {
                    Some(n) => Ok(CellValue::Number(-n)),
                    None => Ok(CellValue::Error("#VALUE!".into())),
                }
            }
            UnaryOp::Plus => self.eval_node(operand),
            UnaryOp::Percent => {
                let val = self.eval_node(operand)?;
                match to_number(&val) {
                    Some(n) => Ok(CellValue::Number(n / 100.0)),
                    None => Ok(CellValue::Error("#VALUE!".into())),
                }
            }
            UnaryOp::ImplicitIntersection => {
                // For now, just return the operand value
                self.eval_node(operand)
            }
        }
    }

    fn eval_function(&self, name: &str, args: &[AstNode]) -> EvalResult<CellValue> {
        let mut evaluated_args: Vec<CellValue> = Vec::with_capacity(args.len());
        for arg in args {
            match arg {
                AstNode::Range(range) => {
                    // Expand Range arg inline into evaluated CellValues.
                    // The expansion is prefixed with dimension markers:
                    //   [col_marker, row_count, cell1, cell2, ...]
                    // where col_marker = -(cols + 1_000_000.0) is a sentinel
                    // that lookup functions can detect to reconstruct the 2D table.
                    let sheet = range.start.sheet.as_deref().unwrap_or(&self.current_sheet);
                    let n_cols = range.end.col - range.start.col + 1;
                    let n_rows = range.end.row - range.start.row + 1;

                    // Sentinel: negative number encoding column count
                    evaluated_args.push(CellValue::Number(-(n_cols as f64 + 1_000_000.0)));
                    evaluated_args.push(CellValue::Number(n_rows as f64));

                    let data = self.provider.get_range(
                        sheet,
                        range.start.row,
                        range.start.col,
                        range.end.row,
                        range.end.col,
                    );
                    for row in &data {
                        for cell in row {
                            evaluated_args.push(cell.clone());
                        }
                    }
                }
                _ => {
                    let val = self.eval_node(arg)?;
                    evaluated_args.push(val);
                }
            }
        }

        // Look up the function
        let func = self.function_registry.get(name);
        match func {
            Some(f) => {
                // Pass DataProvider as &dyn DataProvider for range-aware functions
                let provider: &dyn DataProvider = &*self.provider;
                Ok(f(&evaluated_args, provider))
            }
            None => Ok(CellValue::Error(format!("#NAME?({})", name))),
        }
    }
}

// --- Utility functions ---

pub fn to_number(val: &CellValue) -> Option<f64> {
    match val {
        CellValue::Number(n) => Some(*n),
        CellValue::String(s) => s.parse::<f64>().ok(),
        CellValue::Bool(true) => Some(1.0),
        CellValue::Bool(false) => Some(0.0),
        CellValue::DateTime(_) => None, // Excel stores dates as serial numbers, but we keep them typed
        CellValue::Empty => Some(0.0),
        CellValue::Error(_) => None,
    }
}

pub fn cell_value_to_string(val: &CellValue) -> String {
    match val {
        CellValue::String(s) => s.clone(),
        CellValue::Number(n) => {
            // Remove trailing zeros and decimal point if whole number
            if *n == (*n as i64) as f64 && n.is_finite() {
                format!("{}", *n as i64)
            } else {
                format!("{}", n)
            }
        }
        CellValue::Bool(true) => "TRUE".to_string(),
        CellValue::Bool(false) => "FALSE".to_string(),
        CellValue::DateTime(_) => "".to_string(),
        CellValue::Empty => "".to_string(),
        CellValue::Error(e) => e.clone(),
    }
}

/// Compare two CellValues for ordering (compatible with Excel semantics).
///
/// Numbers < Strings (Excel convention: numbers sort before text).
/// Errors always return None (incomparable).
pub fn partial_cmp_cell_values(a: &CellValue, b: &CellValue) -> Option<Ordering> {
    match (a, b) {
        (CellValue::Number(a), CellValue::Number(b)) => a.partial_cmp(b),
        (CellValue::String(a), CellValue::String(b)) => Some(a.cmp(b)),
        (CellValue::Bool(a), CellValue::Bool(b)) => Some(a.cmp(b)),
        (CellValue::Empty, CellValue::Empty) => Some(Ordering::Equal),
        (CellValue::Number(_), CellValue::String(_)) => Some(Ordering::Less),
        (CellValue::String(_), CellValue::Number(_)) => Some(Ordering::Greater),
        (CellValue::Bool(_), CellValue::Number(_)) => Some(Ordering::Greater),
        (CellValue::Number(_), CellValue::Bool(_)) => Some(Ordering::Less),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_number_basic() {
        assert_eq!(to_number(&CellValue::Number(42.0)), Some(42.0));
        assert_eq!(to_number(&CellValue::String("3.14".into())), Some(3.14));
        assert_eq!(to_number(&CellValue::Bool(true)), Some(1.0));
        assert_eq!(to_number(&CellValue::Bool(false)), Some(0.0));
        assert_eq!(to_number(&CellValue::Empty), Some(0.0));
        assert_eq!(to_number(&CellValue::Error("#N/A".into())), None);
    }
}
