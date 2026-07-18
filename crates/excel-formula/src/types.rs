//! AST types used by the formula parser and evaluator.

use std::fmt;

/// A parsed formula AST node.
#[derive(Debug, Clone, PartialEq)]
pub enum AstNode {
    /// A numeric literal
    Number(f64),
    /// A string literal (without quotes)
    String(String),
    /// A boolean literal (TRUE/FALSE)
    Bool(bool),
    /// A function call with arguments
    Function { name: String, args: Vec<AstNode> },
    /// A single cell reference (A1, $C$2, Sheet2!B3)
    CellRef(CellReference),
    /// A range reference (A1:B10, Sheet2!A1:B10)
    Range(RangeReference),
    /// Binary operation (left op right)
    BinaryOp {
        op: BinOp,
        left: Box<AstNode>,
        right: Box<AstNode>,
    },
    /// Unary operation (e.g., -5, +5, 50%)
    UnaryOp { op: UnaryOp, operand: Box<AstNode> },
    /// Array literal {1,2,3;4,5,6}
    Array(Vec<Vec<AstNode>>),
    /// Excel error value (#DIV/0!, #N/A, etc.)
    Error(ExcelError),
    /// Empty argument (used in function args like SUM(,1))
    EmptyArg,
}

/// A single cell reference with absolute/relative addressing.
#[derive(Debug, Clone, PartialEq)]
pub struct CellReference {
    /// Column index (0-based)
    pub col: u32,
    /// Whether the column reference is absolute ($C)
    pub col_absolute: bool,
    /// Row index (0-based)
    pub row: u32,
    /// Whether the row reference is absolute ($1)
    pub row_absolute: bool,
    /// Optional sheet name for cross-sheet reference
    pub sheet: Option<String>,
    /// Original string representation
    pub original: String,
}

/// A range reference spanning from start to end cell.
#[derive(Debug, Clone, PartialEq)]
pub struct RangeReference {
    pub start: CellReference,
    pub end: CellReference,
    pub original: String,
}

/// Binary operators supported in formulas.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Pow,
    Concat,
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
}

impl fmt::Display for BinOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BinOp::Add => write!(f, "+"),
            BinOp::Sub => write!(f, "-"),
            BinOp::Mul => write!(f, "*"),
            BinOp::Div => write!(f, "/"),
            BinOp::Pow => write!(f, "^"),
            BinOp::Concat => write!(f, "&"),
            BinOp::Eq => write!(f, "="),
            BinOp::Ne => write!(f, "<>"),
            BinOp::Lt => write!(f, "<"),
            BinOp::Gt => write!(f, ">"),
            BinOp::Le => write!(f, "<="),
            BinOp::Ge => write!(f, ">="),
        }
    }
}

/// Unary operators supported in formulas.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UnaryOp {
    Neg,
    Plus,
    Percent,
    /// Implicit intersection (@)
    ImplicitIntersection,
}

/// Standard Excel error values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExcelError {
    Div0,
    NA,
    Name,
    Null,
    Num,
    Ref,
    Value,
    Spill,
    Calc,
    /// Catch-all for unknown errors
    Value_(String),
}

impl ExcelError {
    pub fn as_str(&self) -> &str {
        match self {
            ExcelError::Div0 => "#DIV/0!",
            ExcelError::NA => "#N/A",
            ExcelError::Name => "#NAME?",
            ExcelError::Null => "#NULL!",
            ExcelError::Num => "#NUM!",
            ExcelError::Ref => "#REF!",
            ExcelError::Value => "#VALUE!",
            ExcelError::Spill => "#SPILL!",
            ExcelError::Calc => "#CALC!",
            ExcelError::Value_(_) => "#VALUE!",
        }
    }
}

impl fmt::Display for ExcelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
