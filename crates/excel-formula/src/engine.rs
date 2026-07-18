//! Formula evaluation engine.

use std::collections::HashMap;
use std::sync::Arc;

use excel_types::CellValue;

use crate::evaluator::{EvalResult, Evaluator};
use crate::functions::create_registry;
use crate::parser;
use crate::spill;
use crate::types::AstNode;

/// Trait for providing cell data to the formula engine.
///
/// Implement this trait to connect the engine to a data source
/// (e.g., calamine workbook, in-memory store, or SQL database).
pub trait DataProvider: Send + Sync {
    /// Get the value of a single cell.
    fn get_cell(&self, sheet: &str, row: u32, col: u32) -> Option<CellValue>;

    /// Get all values in a rectangular range.
    fn get_range(
        &self,
        sheet: &str,
        start_row: u32,
        start_col: u32,
        end_row: u32,
        end_col: u32,
    ) -> Vec<Vec<CellValue>>;

    /// Check if a cell exists (even if Empty).
    fn cell_exists(&self, sheet: &str, row: u32, col: u32) -> bool;
}

/// A simple in-memory data provider for testing and isolated evaluation.
#[derive(Clone)]
pub struct InMemoryDataProvider {
    data: Arc<HashMap<String, Vec<Vec<CellValue>>>>,
}

impl InMemoryDataProvider {
    pub fn new() -> Self {
        Self {
            data: Arc::new(HashMap::new()),
        }
    }

    /// Insert a sheet's data into the provider.
    ///
    /// `cells` is a 2D vector where `cells[row][col]` gives the cell value.
    pub fn with_sheet(mut self, sheet_name: impl Into<String>, cells: Vec<Vec<CellValue>>) -> Self {
        Arc::make_mut(&mut self.data).insert(sheet_name.into(), cells);
        self
    }
}

impl DataProvider for InMemoryDataProvider {
    fn get_cell(&self, sheet: &str, row: u32, col: u32) -> Option<CellValue> {
        self.data
            .get(sheet)?
            .get(row as usize)?
            .get(col as usize)
            .cloned()
    }

    fn get_range(
        &self,
        sheet: &str,
        start_row: u32,
        start_col: u32,
        end_row: u32,
        end_col: u32,
    ) -> Vec<Vec<CellValue>> {
        let sheet_data = match self.data.get(sheet) {
            Some(d) => d,
            None => {
                return vec![
                    vec![
                        CellValue::Error("#REF!".into());
                        (end_col - start_col + 1) as usize
                    ];
                    (end_row - start_row + 1) as usize
                ];
            }
        };

        let mut result = Vec::new();
        for r in start_row..=end_row {
            let mut row_data = Vec::new();
            for c in start_col..=end_col {
                let val = sheet_data
                    .get(r as usize)
                    .and_then(|row| row.get(c as usize))
                    .cloned()
                    .unwrap_or(CellValue::Empty);
                row_data.push(val);
            }
            result.push(row_data);
        }
        result
    }

    fn cell_exists(&self, sheet: &str, row: u32, col: u32) -> bool {
        self.data
            .get(sheet)
            .and_then(|d| d.get(row as usize))
            .and_then(|r| r.get(col as usize))
            .is_some()
    }
}

/// Result of a formula evaluation, possibly with dynamic array spill.
#[derive(Debug, Clone)]
pub enum FormulaResult {
    /// A single-cell result.
    Single(CellValue),
    /// A dynamic array that spilled to multiple cells.
    /// Contains the spilled values as 2D array.
    Spill(spill::SpillResult),
}

impl FormulaResult {
    pub fn is_spill(&self) -> bool {
        matches!(self, FormulaResult::Spill(_))
    }

    pub fn to_cell_value(&self) -> CellValue {
        match self {
            FormulaResult::Single(v) => v.clone(),
            FormulaResult::Spill(s) => {
                if s.rows == 1 && s.cols == 1 {
                    s.values
                        .first()
                        .and_then(|r| r.first())
                        .cloned()
                        .unwrap_or(CellValue::Empty)
                } else {
                    // Multi-cell spill cannot be represented as a single value
                    CellValue::Error("#SPILL!".into())
                }
            }
        }
    }
}

/// The main formula evaluation engine.
pub struct FormulaEngine<P: DataProvider> {
    provider: Arc<P>,
    /// Function registry: function name -> implementation.
    /// Each function receives evaluated arguments and the data provider for range context.
    function_registry:
        HashMap<String, Arc<dyn Fn(&[CellValue], &dyn DataProvider) -> CellValue + Send + Sync>>,
    /// Stack for circular reference detection (sheet!cell strings)
    eval_stack: std::sync::Mutex<Vec<String>>,
}

impl<P: DataProvider + 'static> FormulaEngine<P> {
    /// Create a new formula engine with the given data provider.
    pub fn new(provider: P) -> Self {
        let provider = Arc::new(provider);
        let function_registry = create_registry();

        Self {
            provider,
            function_registry,
            eval_stack: std::sync::Mutex::new(Vec::new()),
        }
    }

    /// Evaluate a formula and return the result.
    ///
    /// `sheet` is the current sheet context.
    /// `cell_ref` is for error messages (e.g., "A1").
    /// `formula` is the formula string, with or without a leading `=`.
    pub fn evaluate(
        &self,
        sheet: &str,
        cell_ref: &str,
        formula: &str,
    ) -> EvalResult<FormulaResult> {
        let ast = parser::parse(formula).map_err(|e| crate::evaluator::EvalError::parse(e.msg))?;

        let mut evaluator = Evaluator::new(
            self.provider.clone(),
            &self.function_registry,
            &self.eval_stack,
        );

        let cell_value = evaluator.evaluate(sheet, &ast)?;

        // Check if this is a dynamic array spill function
        if let AstNode::Function { ref name, .. } = ast {
            if spill::is_spill_function(name) {
                let spill_result = spill::try_spill(sheet, &ast, &mut evaluator)?;
                return Ok(FormulaResult::Spill(spill_result));
            }
        }

        Ok(FormulaResult::Single(cell_value))
    }

    /// Register a custom function.
    #[allow(dead_code)]
    pub fn register_function(
        &mut self,
        name: &str,
        func: impl Fn(&[CellValue], &dyn DataProvider) -> CellValue + Send + Sync + 'static,
    ) {
        self.function_registry
            .insert(name.to_uppercase(), Arc::new(func));
    }
}
