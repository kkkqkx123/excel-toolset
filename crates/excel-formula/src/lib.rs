//! Formula evaluation engine for Excel formulas.
//!
//! This crate provides a formula parser and evaluation engine that can
//! evaluate Excel formulas without an Excel client. It supports:
//!
//! - A1-style cell references (relative, absolute, mixed)
//! - Range references and cross-sheet references
//! - Arithmetic, comparison, and text operators
//! - 200+ built-in functions (math, lookup, dynamic, financial, statistical)
//! - Dynamic array spill (FILTER, SORT, UNIQUE, etc.)
//!
//! # Examples
//!
//! ```rust,ignore
//! use excel_formula::{FormulaEngine, InMemoryDataProvider};
//!
//! let provider = InMemoryDataProvider::new();
//! let engine = FormulaEngine::new(provider);
//!
//! let result = engine.evaluate("Sheet1", "A1", "=SUM(1,2,3)").unwrap();
//! ```

pub mod engine;
pub mod evaluator;
pub mod functions;
pub mod parser;
pub mod spill;
pub mod types;

pub use engine::{DataProvider, FormulaEngine, InMemoryDataProvider};
pub use evaluator::EvalResult;
pub use spill::SpillResult;

// Re-export excel_types for convenience
pub use excel_types as excel_types_reexport;
