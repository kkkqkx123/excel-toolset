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
