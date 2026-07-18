---
name: excel
description: Read, write, format, and analyze Excel files using the excel-cli tool. Covers file management, cell/range operations, formulas, data processing, styling, charts, and advanced features like pivot tables, data validation, VBA, protection, and images.
arguments:
  - name: path
    description: Absolute path to the Excel file
    required: true
  - name: sheet
    description: Worksheet name
    required: false
  - name: cell
    description: Cell reference (e.g. A1)
    required: false
  - name: range
    description: Range reference (e.g. A1:C10)
    required: false
---

# Excel File Operations

Use `excel-cli` to work with Excel files programmatically. All commands output JSON by default; pass `--pretty` for human-readable formatting.

## Invocation

Build the CLI first if not already built:

```bash
cargo build --package excel-cli --release
```

The binary is at `./target/release/excel-cli`.

For SQL query support, build with:

```bash
cargo build --package excel-cli --release --features sql
```

## Safety Guarantees

Every write operation runs through the security module: hash checksum → automatic backup to `.backups/` → execute. No data loss is possible — if anything goes wrong, use `excel-cli rollback <path> <backup-path>` to restore.

All destructive commands support `--dry-run` to preview the effect before applying changes.

## Task Routing

Determine the operation domain, then consult `references/cli-reference.md` for exact command syntax.

| Domain | What you can do |
|--------|----------------|
| **File & Sheet** | Create files, list sheets, add/delete/rename sheets, backup, rollback, set sheet visibility |
| **Read & Search** | Read cells/ranges, search workbook-wide or per-sheet, inspect file metadata and history, list/get named ranges |
| **Write** | Write cells/ranges (JSON or CSV), append/insert/delete rows, batch modify with transactions |
| **Formula** | Set formulas, evaluate (single or batch), auto-fill, trace dependencies, explain in natural language |
| **Data** | Filter rows by condition, sort, deduplicate, SQL queries against sheet data, auto-filter |
| **Format** | Cell styling (font, color, borders, alignment, number format), merge cells, conditional formatting, charts, tables |
| **Advanced** | Comments, named range CRUD, VBA import/export, data validation, pivot tables, slicers, sparklines, freeze panes, sheet protection, page setup, images/shapes |

## Cross-Cutting Workflows

### Safe Write Pattern

Always preview with `--dry-run` before applying:

```bash
# Step 1: preview
excel-cli cell write data.xlsx Sheet1 A1 "New Value" --dry-run

# Step 2: execute
excel-cli cell write data.xlsx Sheet1 A1 "New Value"
```

### Read → Analyze → Write

```bash
# Read existing data
excel-cli range read report.xlsx Sheet1 A1:D100 --mode compact

# Analyze (e.g. with SQL)
excel-cli data sql report.xlsx Sheet1 "SELECT A, SUM(D) FROM t GROUP BY A"

# Write results
excel-cli sheet add report.xlsx Summary
excel-cli range write report.xlsx Summary A1:B5 '[["Category","Total"],["A",500],["B",300]]'

# Format output
excel-cli format set report.xlsx Summary A1:B1 '{"bold":true,"bg_color":"#4472C4","font_color":"#FFFFFF"}'
```

### Batch Operations

For multiple writes, prefer `batch modify` over individual commands — it supports transactional semantics with `--strategy all-or-nothing`:

```bash
excel-cli batch modify data.xlsx --strategy all-or-nothing --operations '[
  {"op": "rename_sheet", "old_name": "Sheet1", "new_name": "Data"},
  {"op": "write_cell", "sheet": "Data", "row": 1, "col": 1, "value": "Title"},
  {"op": "set_format", "sheet": "Data", "range": "A1:E1", "style": {"bold":true,"font_size":16}}
]'
```

### Report Generation

```bash
# Create file
excel-cli file create report.xlsx --sheet "Monthly Report"

# Write headers and data
excel-cli range write "report.xlsx" "Monthly Report" A1:C4 '[["Month","Revenue","Cost"],["Jan",1200,800],["Feb",1500,900],["Mar",1100,850]]'

# Add summary formula
excel-cli formula set "report.xlsx" "Monthly Report" C5 "=SUM(C2:C4)" --eval

# Style header row
excel-cli format set "report.xlsx" "Monthly Report" A1:C1 '{"bold":true,"font_size":14,"bg_color":"#4472C4","font_color":"#FFFFFF"}'

# Merge title
excel-cli format merge "report.xlsx" "Monthly Report" A1:C1 --value "Monthly Financial Report"

# Add chart
excel-cli chart create "report.xlsx" "Monthly Report" A1:B4 column --title "Monthly Revenue" --position E2

# Freeze header
excel-cli freeze-pane set "report.xlsx" "Monthly Report" --rows 1
```

## SQL Query Notes

SQL queries require the `sql` feature flag at build time (`--features sql`). The current worksheet is mapped to table `t`, with columns named A, B, C, ... matching Excel column letters.

Supported clauses: SELECT, WHERE, ORDER BY, GROUP BY, LIMIT. Supported aggregates: COUNT, SUM, AVG, MIN, MAX.

```bash
excel-cli data sql sales.xlsx Sheet1 "SELECT A, SUM(C) FROM t GROUP BY A ORDER BY SUM(C) DESC LIMIT 10"
```
