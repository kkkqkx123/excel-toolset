# CLI Command Reference

Full syntax for all `excel-cli` subcommands organized by functional domain.

## Global Flags

| Flag | Effect |
|------|--------|
| `--pretty` | Pretty-print JSON output |
| `--format json\|text` | Output format (text only supported by diff commands) |

All write operations support `--dry-run` to preview effects without modifying the file.

---

## File & Sheet Management

### Create File

```bash
excel-cli file create <path> [--sheet <name>]
```

Creates a new workbook. Default sheet name is `Sheet1`.

### File Info

```bash
excel-cli file info <path>
```

Returns sheet count, names, file hash, and metadata.

### Backup

```bash
excel-cli file backup <path> [--output <dest>]
```

Always creates a timestamped backup in `.backups/`. `--output` optionally copies to an additional location.

### Rollback

```bash
excel-cli rollback <path> <backup-path>
```

Restores file from a backup.

### Sheet Operations

```bash
# List all sheet names
excel-cli sheet list <path>

# Add a sheet
excel-cli sheet add <path> <name>

# Delete a sheet
excel-cli sheet delete <path> <name>

# Rename a sheet
excel-cli sheet rename <path> <old-name> <new-name>

# Set visibility
excel-cli sheet set-visibility <path> <name> --visibility <mode> [--dry-run]
```

Visibility modes: `visible`, `hidden`, `very_hidden` (cannot be unhidden via Excel UI).

---

## Reading & Search

### Cell Read

```bash
excel-cli cell read <path> <sheet> <cell>
```

Standard Excel cell reference (e.g. `A1`, `B3`, `Z100`).

### Range Read

```bash
excel-cli range read <path> <sheet> <range> [--mode <mode>] [--truncate <n>]
```

Output modes:
- `detailed` (default): structured data with row/column indices
- `compact`: plain 2D array matrix
- `csv`: CSV-formatted text

### Overview

```bash
# Basic overview
excel-cli overview <path>

# Blueprint mode (detailed structure: tables, formulas, named ranges)
excel-cli overview <path> --blueprint
```

### History

```bash
excel-cli history <path>
```

Shows operation history for the file.

### Search

```bash
# Workbook-wide search
excel-cli search workbook <path> <pattern> \
  [--match-type contains|exact|regex] \
  [--search-type both|value|formula] \
  [--case-sensitive] \
  [--sheets <sheet1>,<sheet2>]

# Single-sheet search
excel-cli search sheet <path> <sheet> <pattern> \
  [--match-type contains|exact|regex] \
  [--search-type both|value|formula] \
  [--case-sensitive]
```

Match types: `contains` (default, substring), `exact`, `regex`.
Search types: `both` (default, values and formulas), `value`, `formula`.

### Named Range (Read)

```bash
excel-cli named-range list <path>
excel-cli named-range get <path> <name>
```

---

## Writing & Modification

### Cell Write

```bash
excel-cli cell write <path> <sheet> <cell> <value> [--dry-run]
```

Type auto-inference: numeric strings become numbers, `true`/`false` become booleans.

### Range Write

```bash
# From JSON array
excel-cli range write <path> <sheet> <range> '<json-data>' [--dry-run]

# From CSV file
excel-cli range write-csv <path> <sheet> <range> <csv-file> [--dry-run]

# Clear range
excel-cli range clear <path> <sheet> <range> [--dry-run]
```

JSON data is a 2D array: `'[["Header1","Header2"],["Val1",42],["Val2",99]]'`.

### Row Operations

```bash
# Append row at end
excel-cli data append-row <path> <sheet> <val1> <val2> ... [--dry-run]

# Insert row at position (shifts existing rows down)
excel-cli data insert-row <path> <sheet> <row> <val1> <val2> ... [--dry-run]

# Delete row
excel-cli data delete-row <path> <sheet> <row> [--dry-run]
```

### Batch Modify

```bash
excel-cli batch modify <path> --operations '<json>' \
  [--dry-run] \
  [--strategy best-effort|all-or-nothing|dry-run] \
  [--validate-only]
```

Strategies:
- `best-effort` (default): continue on error, report failures
- `all-or-nothing`: roll back all changes if any operation fails
- `dry-run`: validate all operations without executing

Supported operation types:

| op | Required fields | Description |
|----|----------------|-------------|
| `write_cell` | `sheet`, `row`, `col`, `value` | Write a single cell |
| `write_range` | `sheet`, `range`, `data` | Write a 2D array to a range |
| `add_sheet` | `name` | Add a worksheet |
| `delete_sheet` | `name` | Delete a worksheet |
| `rename_sheet` | `old_name`, `new_name` | Rename a worksheet |
| `set_format` | `sheet`, `range`, `style` | Apply cell formatting |
| `merge_cells` | `sheet`, `range` | Merge a range of cells |
| `append_row` | `sheet`, `values` | Append a row at end |
| `insert_row` | `sheet`, `row`, `values` | Insert a row at position |
| `delete_row` | `sheet`, `start_row`, `end_row` | Delete rows |
| `set_formula` | `sheet`, `cell`, `formula` | Set a formula in a cell |

### Validate Refs

```bash
excel-cli batch validate-refs <path> <sheet> <formula>
```

Validates formula references without executing.

---

## Formulas

The formula engine supports 100+ built-in functions across 8 categories: Math, Text, Logic, Date/Time, Lookup, Dynamic Array, Financial, Statistics.

### Set & Evaluate

```bash
# Set formula
excel-cli formula set <path> <sheet> <cell> <formula> [--eval] [--dry-run]

# Single evaluation
excel-cli formula eval <path> <sheet> <cell> <formula> [--no-eval] [--dry-run]

# Batch evaluation
excel-cli formula eval-batch <path> <sheet> '<json-formulas>' [--dry-run]
```

Batch format: `[["cell","=FORMULA"],...]`.

### Read & Refresh

```bash
excel-cli formula read <path> <sheet> <cell>
excel-cli formula refresh <path> <sheet> [--dry-run]
```

### Calculation Mode

```bash
excel-cli formula calc-mode <path> [--mode auto|manual] [--dry-run]
```

### Trace Dependencies

```bash
excel-cli formula trace <path> <sheet> <cell>
```

Returns precedents (cells the formula depends on) and dependents (formulas that reference this cell).

### Explain

```bash
# Natural language explanation
excel-cli formula explain <path> <sheet> <cell> [--language en|zh]

# Business logic interpretation
excel-cli formula explain-logic <path> <sheet> <cell> [--language en|zh]
```

### Auto-Fill

```bash
excel-cli formula fill <path> <sheet> <source> <target-range> [--dry-run]
```

Relative references adjust automatically during fill (e.g. `=B1+C1` filled from A1 to A2 becomes `=B2+C2`).

---

## Data Processing

### Filter

```bash
excel-cli data filter <path> <sheet> <column> <op> <value>
```

Read-only operation, returns matching rows. Column is 1-indexed.

Supported operators: `eq`, `ne`, `gt`, `lt`, `gte`, `lte`, `contains`.

### Sort

```bash
excel-cli data sort <path> <sheet> <column> [--desc] [--dry-run]
```

Modifies the file. Prefer `--dry-run` for preview.

### Deduplicate

```bash
excel-cli data dedup <path> <sheet> [--column <col>] [--dry-run]
```

Without `--column`, compares entire rows. Keeps the first occurrence.

### SQL Query

Requires `--features sql` at build time.

```bash
excel-cli data sql <path> <sheet> <query> [--session] [--cache]
```

The current worksheet is mapped to table `t`. Columns use Excel letter names (A, B, C, ...).

Supported clauses: SELECT, WHERE, ORDER BY, GROUP BY, LIMIT.
Supported aggregates: COUNT, SUM, AVG, MIN, MAX.

```bash
# Basic query
excel-cli data sql data.xlsx Sheet1 "SELECT * FROM t"

# Filtered & sorted
excel-cli data sql data.xlsx Sheet1 "SELECT A, B, C FROM t WHERE B > 100 ORDER BY C DESC"

# Aggregation
excel-cli data sql data.xlsx Sheet1 "SELECT COUNT(*) as cnt, AVG(C) as avg_val FROM t"

# Grouped aggregation
excel-cli data sql data.xlsx Sheet1 "SELECT A, SUM(C) as total FROM t GROUP BY A"

# Limit
excel-cli data sql data.xlsx Sheet1 "SELECT * FROM t LIMIT 10"
```

### Auto-Filter

```bash
excel-cli auto-filter set <path> <sheet> <range> [--dry-run]
excel-cli auto-filter remove <path> <sheet> [--dry-run]
excel-cli auto-filter get <path> <sheet>
```

---

## Formatting & Charts

### Cell Style

```bash
excel-cli format set <path> <sheet> <range> '<style-json>' [--dry-run]
```

Style properties:

| Property | Type | Example |
|----------|------|---------|
| `bold` | bool | `true` |
| `italic` | bool | `false` |
| `underline` | bool | `true` |
| `font_size` | u8 | `14` |
| `font_color` | string (hex) | `"#FF0000"` |
| `bg_color` | string (hex) | `"#FFFF00"` |
| `num_format` | string | `"#,##0.00"` |
| `alignment` | object | `{"horizontal":"center","vertical":"center"}` |
| `border` | object | `{"color":"#000000","style":"thin"}` |

### Merge Cells

```bash
excel-cli format merge <path> <sheet> <range> [--value <v>] [--dry-run]
```

### Conditional Format

```bash
excel-cli conditional-format add <path> <sheet> <range> <rule-type> <condition> \
  [--style '<json>'] \
  [--config '<json>'] \
  [--dry-run]

excel-cli conditional-format remove <path> <sheet> <range> [--dry-run]
```

Rule types:

| Type | Condition example | Description |
|------|------------------|-------------|
| `cell_value` | `>100` | Value-based condition |
| `formula` | `=A1>AVERAGE(A:A)` | Formula-based condition |
| `above_average` | (ignored) | Highlight above average |
| `top10` | `5` | Top N items |
| `duplicate` | (ignored) | Duplicate values |
| `text_contains` | `keyword` | Text contains |
| `date_occurring` | `this_month` | Date condition |

### Chart

```bash
excel-cli chart create <path> <sheet> <range> <chart-type> \
  [--title <title>] \
  [--position <cell>] \
  [--trendline '<json>'] \
  [--y-error-bars '<json>'] \
  [--x-error-bars '<json>'] \
  [--log-base <n>] \
  [--dry-run]
```

Chart types: `column`, `bar`, `line`, `pie`, `area`, `scatter`.

### Table

```bash
excel-cli table create <path> --config '<json>' [--dry-run]
excel-cli table remove <path> <name> [--dry-run]
excel-cli table list <path>
excel-cli table get <path> <name>
```

---

## Advanced Features

### Comments

```bash
excel-cli comments get <path> <sheet> <cell>
excel-cli comments add <path> <sheet> <cell> <text> [--dry-run]
excel-cli comments update <path> <sheet> <cell> <text> [--dry-run]
excel-cli comments delete <path> <sheet> <cell> [--dry-run]
```

### Named Range (Write)

```bash
excel-cli named-range create <path> <name> <range> [--sheet <name>] [--dry-run]
excel-cli named-range delete <path> <name> [--dry-run]
```

### VBA

```bash
excel-cli vba export <path> <output-file>
excel-cli vba import <path> <vba-file> [--dry-run]
excel-cli vba has <path>
```

### Data Validation

```bash
excel-cli data-validation add <path> <sheet> --config '<json>' [--dry-run]
excel-cli data-validation remove <path> <sheet> <range> [--dry-run]
```

Example dropdown list config:

```json
{
  "range": "A1:A100",
  "validation_type": "list",
  "criteria": {
    "source": "OptionA,OptionB,OptionC"
  }
}
```

### Pivot Table

```bash
excel-cli pivot-table create <path> --config '<json>' [--dry-run]
```

Example config:

```json
{
  "sheet": "Sheet1",
  "data_range": "A1:E500",
  "rows": ["Category", "Product"],
  "columns": ["Region"],
  "values": [
    {"field": "Amount", "aggregation": "sum"},
    {"field": "Quantity", "aggregation": "count"}
  ],
  "filters": ["Year"]
}
```

### Slicer

```bash
excel-cli slicer create <path> --config '<json>' [--dry-run]
```

### Sparkline

```bash
excel-cli sparkline add <path> <sheet> <source-range> <target-cell> \
  [--sparkline-type line|column|winlose] \
  [--style <n>] \
  [--dry-run]

excel-cli sparkline remove <path> <sheet> <target-cell> [--dry-run]
```

Source range uses sheet-qualified format: `'Sheet1'!A1:E1`.

### Freeze Pane

```bash
excel-cli freeze-pane set <path> <sheet> [--rows <n>] [--cols <n>] [--dry-run]
excel-cli freeze-pane clear <path> <sheet> [--dry-run]
```

### Protection

```bash
excel-cli protection protect <path> <sheet> [--password <pwd>] [--options '<json>'] [--dry-run]
excel-cli protection unprotect <path> <sheet> [--dry-run]
excel-cli protection is-protected <path> <sheet>
```

### Page Setup

```bash
excel-cli page-setup configure <path> <sheet> --config '<json>' [--dry-run]
excel-cli page-setup page-breaks <path> --config '<json>' [--dry-run]
excel-cli page-setup clear-breaks <path> <sheet> [--dry-run]
```

### Image & Shapes

```bash
excel-cli image insert <path> --config '<json>' [--dry-run]
excel-cli image remove <path> <sheet> <anchor-cell> [--dry-run]
excel-cli image shape-insert <path> --config '<json>' [--dry-run]
```

Supported shape types: `rectangle`, `ellipse`, `line`.
