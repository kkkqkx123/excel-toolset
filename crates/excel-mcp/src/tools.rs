// Tool definitions and handler registry.
//
// Each category is a separate sub-module under tools/.
// This module aggregates all tools and handlers into a single registry.

use std::collections::HashMap;

use crate::server::{ToolDef, ToolHandler};

mod auto_filter;
mod batch;
mod cell;
mod chart;
mod comments;
mod conditional_format;
mod data;
mod data_validation;
mod diff;
mod file;
mod format;
mod formula;
mod freeze_panes;
mod helpers;
mod history;
mod image;
mod named_range;
mod overview;
mod page_setup;
mod pivot_table;
mod range;
mod rollback;
mod search;
mod sheet;
mod sheet_protection;
mod slicer;
mod sparkline;
mod table;
mod vba;

/// Register all tools and their handlers.
/// Returns (tool_definitions, handler_map).
pub fn register_all() -> (Vec<ToolDef>, HashMap<String, ToolHandler>) {
    let mut tools = Vec::new();
    let mut handlers: HashMap<String, ToolHandler> = HashMap::new();

    // File operations
    tools.extend(file::tools());
    file::register(&mut handlers);

    // Sheet operations
    tools.extend(sheet::tools());
    sheet::register(&mut handlers);

    // Cell operations
    tools.extend(cell::tools());
    cell::register(&mut handlers);

    // Range operations
    tools.extend(range::tools());
    range::register(&mut handlers);

    // Data operations
    tools.extend(data::tools());
    data::register(&mut handlers);

    // Formula operations
    tools.extend(formula::tools());
    formula::register(&mut handlers);

    // Format operations
    tools.extend(format::tools());
    format::register(&mut handlers);

    // Chart operations
    tools.extend(chart::tools());
    chart::register(&mut handlers);

    // VBA operations
    tools.extend(vba::tools());
    vba::register(&mut handlers);

    // Diff operations
    tools.extend(diff::tools());
    diff::register(&mut handlers);

    // Batch operations
    tools.extend(batch::tools());
    batch::register(&mut handlers);

    // Rollback
    tools.extend(rollback::tools());
    rollback::register(&mut handlers);

    // Comments
    tools.extend(comments::tools());
    comments::register(&mut handlers);

    // Named ranges
    tools.extend(named_range::tools());
    named_range::register(&mut handlers);

    // Search
    tools.extend(search::tools());
    search::register(&mut handlers);

    // Conditional format
    tools.extend(conditional_format::tools());
    conditional_format::register(&mut handlers);

    // Table
    tools.extend(table::tools());
    table::register(&mut handlers);

    // Data validation
    tools.extend(data_validation::tools());
    data_validation::register(&mut handlers);

    // Pivot table
    tools.extend(pivot_table::tools());
    pivot_table::register(&mut handlers);

    // Slicer
    tools.extend(slicer::tools());
    slicer::register(&mut handlers);

    // Sparkline
    tools.extend(sparkline::tools());
    sparkline::register(&mut handlers);

    // Overview
    tools.extend(overview::tools());
    overview::register(&mut handlers);

    // History
    tools.extend(history::tools());
    history::register(&mut handlers);

    // Freeze panes
    tools.extend(freeze_panes::tools());
    freeze_panes::register(&mut handlers);

    // AutoFilter
    tools.extend(auto_filter::tools());
    auto_filter::register(&mut handlers);

    // Sheet protection
    tools.extend(sheet_protection::tools());
    sheet_protection::register(&mut handlers);

    // Page setup
    tools.extend(page_setup::tools());
    page_setup::register(&mut handlers);

    // Image and shape
    tools.extend(image::tools());
    image::register(&mut handlers);

    (tools, handlers)
}
