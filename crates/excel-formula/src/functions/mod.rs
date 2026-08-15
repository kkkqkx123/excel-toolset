//! Function registry and module declarations.

pub mod datetime;
pub mod dynamic;
pub mod financial;
pub mod logical;
pub mod lookup;
pub mod math;
pub mod statistical;
pub mod text;

use crate::engine::DataProvider;
use excel_types::CellValue;
use std::collections::HashMap;
use std::sync::Arc;

/// Create the default function registry with all built-in functions.
pub fn create_registry()
-> HashMap<String, Arc<dyn Fn(&[CellValue], &dyn DataProvider) -> CellValue + Send + Sync>> {
    let mut registry: HashMap<
        String,
        Arc<dyn Fn(&[CellValue], &dyn DataProvider) -> CellValue + Send + Sync>,
    > = HashMap::new();

    math::register(&mut registry);
    text::register(&mut registry);
    logical::register(&mut registry);
    datetime::register(&mut registry);
    lookup::register(&mut registry);
    dynamic::register(&mut registry);
    financial::register(&mut registry);
    statistical::register(&mut registry);

    registry
}

// ---------------------------------------------------------------------------
// Range-marker aware argument helpers.
//
// When a formula function receives a range argument (e.g. `SUM(C2:C6)`), the
// evaluator expands it inline as:
//     [sentinel: -(cols + 1_000_000.0), rows: Number(n_rows), v, v, ...]
// so lookup-style functions can reconstruct the 2D table. Aggregate functions
// (SUM/AVERAGE/COUNT/...) must treat that block as its *data values* and must
// NOT count the two sentinels — before this helper, `SUM(C2:C6)` over five
// cells added `-1000001 + 5 + 1 + 2 + 3 + 4 + 5 = -999981`.
// ---------------------------------------------------------------------------

const RANGE_MARKER_THRESHOLD: f64 = -900_000.0;
const RANGE_MARKER_OFFSET: f64 = 1_000_000.0;

/// Iterates over the *data* values inside `args`, expanding every range-marker
/// block and skipping its sentinel pair.
fn range_data_iter(args: &[CellValue]) -> impl Iterator<Item = &CellValue> {
    let mut blocks: Vec<&CellValue> = Vec::new();
    let mut i = 0;
    while i < args.len() {
        if let CellValue::Number(n) = &args[i]
            && *n < RANGE_MARKER_THRESHOLD {
                let cols = (-(*n + RANGE_MARKER_OFFSET)) as usize;
                if let Some(CellValue::Number(rows)) = args.get(i + 1) {
                    let rows = *rows as usize;
                    let total = cols * rows;
                    let end = (i + 2 + total).min(args.len());
                    blocks.extend(args[i + 2..end].iter());
                    i = end;
                    continue;
                }
            }
        blocks.push(&args[i]);
        i += 1;
    }
    blocks.into_iter()
}

/// Flattens `args` into numbers, expanding range-marker blocks.
pub fn flatten_numbers(args: &[CellValue]) -> Vec<f64> {
    let mut numbers = Vec::new();
    for arg in range_data_iter(args) {
        match arg {
            CellValue::Number(n) => numbers.push(*n),
            CellValue::String(s) => {
                if let Ok(n) = s.parse::<f64>() {
                    numbers.push(n);
                }
            }
            CellValue::Bool(true) => numbers.push(1.0),
            CellValue::Bool(false) => numbers.push(0.0),
            _ => {}
        }
    }
    numbers
}

/// Counts values in `args` like Excel's COUNT: numbers and datetimes only,
/// expanding range-marker blocks.
pub fn count_values(args: &[CellValue]) -> f64 {
    range_data_iter(args)
        .filter(|a| matches!(a, CellValue::Number(_) | CellValue::DateTime(_)))
        .count() as f64
}

/// Counts non-empty values in `args` like Excel's COUNTA, expanding
/// range-marker blocks.
pub fn count_non_empty(args: &[CellValue]) -> f64 {
    range_data_iter(args)
        .filter(|a| !matches!(a, CellValue::Empty))
        .count() as f64
}
