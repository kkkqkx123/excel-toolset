# Diff Subsystem Architecture

## Overview

The diff subsystem (`excel-diff` crate) is an independent library within the workspace. It provides Excel content comparison at multiple granularities and can be used as a standalone git diff driver for `.xlsx` files.

## Relationship to Main Project

```
excel-http ─────┐
                 ├──→ excel-core (read/write/security)
excel-cli ───────┘
        │
        └──→ excel-diff (diff engine)
                 │
                 └──→ excel-core (only read types)
```

`excel-diff` depends ONLY on `excel-core`'s read-related types — not on its write operations. This prevents circular dependency issues and keeps the diff crate lightweight.

## Capabilities

### 1. Multi-granularity diff

| Granularity | Function | Description |
|-------------|----------|-------------|
| File | `diff_files(old_path, new_path)` | Quick hash check, then full content compare |
| Sheet | `diff_sheets(old_path, new_path, sheet)` | Single sheet comparison |
| Range | `diff_range(old_path, new_path, sheet, range)` | Specific cell range comparison |
| In-memory | `compute_diffs(old_data, new_data)` | For write-affiliated diffs |

### 2. Diff types

```rust
pub enum DiffType {
    Add,       // New cell/row/sheet
    Delete,    // Removed cell/row/sheet
    Modify,    // Value or formula changed
    Passive,   // Formula text unchanged, but value changed (auto-calc)
    NoChange,  // Identical
}
```

### 3. Formula noise reduction

The `Passive` type distinguishes actively edited cells from cells whose values changed due to formula recalculation. The algorithm:

1. Compare formula text: if formula text is identical but computed value differs → `Passive`
2. If formula text changes → `Modify`
3. If no formula → compare values directly

### 4. Git diff driver

`excel-diff` can register itself as a git diff driver for `.xlsx` files:

```bash
excel-tool diff install-git-driver
```

This configures:
- `.gitattributes`: `*.xlsx diff=excel-diff`
- `git config`: `diff.excel-diff.command = "excel-tool diff"`

The driver converts binary excel files into structured text output that `git diff` can display cleanly.

## Usage as a Library

```rust
// Standalone file diff
let result = excel_diff::diff_files("old.xlsx", "new.xlsx")?;
println!("{}", serde_json::to_string_pretty(&result)?);

// Sheet-specific diff
let sheet_diff = excel_diff::diff_sheets("old.xlsx", "new.xlsx", "Sheet1")?;

// In-memory diff (used by write operations)
let old_data = excel_core::read::read_sheet_all("file.xlsx", "Sheet1")?;
let cell_diffs = excel_diff::compute_diffs(&old_data, &new_data);
```

## Output Format

### FileDiff

```json
{
  "file_hash_match": false,
  "sheet_names_changed": false,
  "sheets": [
    {
      "sheet_name": "Sheet1",
      "row_count_diff": 0,
      "col_count_diff": 1,
      "cells": [
        {
          "row": 0,
          "col": 0,
          "cell_ref": "A1",
          "diff_type": "Modify",
          "old_value": "100",
          "new_value": "200",
          "old_formula": null,
          "new_formula": null
        }
      ]
    }
  ],
  "summary": {
    "adds": 0,
    "deletes": 0,
    "modifies": 1,
    "passives": 0,
    "total_changes": 1
  }
}
```

## Future Extension

- **Web UI rendering**: The structured JSON output can be directly consumed by a static web frontend (HTML/JS/CSS) for visual diff rendering.
- **AI operation log merge**: Combine LLM tool call history with file-level diff to produce human-readable change reports.
- **History traversal**: Use `excel-diff` with git history to support `log` and `show` commands for temporal comparison.