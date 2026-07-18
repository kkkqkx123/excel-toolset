# HTTP API Endpoint Reference

All endpoints use `POST` with JSON body unless noted otherwise. Base URL: `http://localhost:3000`.

Response envelope: `{"success": bool, "data": T|null, "error": {"code": string, "message": string}|null}`.

## Health

### `GET /health`

No request body. Returns server status with timestamp.

---

## File Management

### `POST /api/file/info`

```json
{ "path": "data.xlsx" }
```

Returns sheet count, sheet names, file hash.

### `POST /api/file/create`

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `path` | string | yes | — | File path |
| `sheet` | string | no | `"Sheet1"` | Initial sheet name |

```json
{ "path": "output.xlsx", "sheet": "Sheet1" }
```

### `POST /api/file/backup`

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `path` | string | yes | File path |
| `output` | string | no | Additional copy destination |

```json
{ "path": "data.xlsx", "output": "/tmp/backup.xlsx" }
```

### `POST /api/file/rollback`

```json
{ "path": "data.xlsx", "backup_path": ".backups/backup_20260615.xlsx" }
```

---

## Sheet Management

### `POST /api/sheet/list`

```json
{ "path": "data.xlsx" }
```

### `POST /api/sheet/add`

```json
{ "path": "data.xlsx", "name": "NewSheet" }
```

### `POST /api/sheet/delete`

```json
{ "path": "data.xlsx", "name": "Sheet2" }
```

### `POST /api/sheet/rename`

```json
{ "path": "data.xlsx", "old_name": "OldSheet", "new_name": "NewSheet" }
```

### `POST /api/sheet/visibility`

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `path` | string | yes | File path |
| `name` | string | yes | Sheet name |
| `visibility` | string | yes | `visible`, `hidden`, `very_hidden` |
| `dry_run` | boolean | no | Preview only |

```json
{ "path": "data.xlsx", "name": "Sheet1", "visibility": "hidden" }
```

---

## Freeze Panes

### `POST /api/freeze-panes/set`

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `path` | string | yes | — | File path |
| `sheet` | string | yes | — | Sheet name |
| `rows` | u32 | no | 0 | Rows to freeze from top |
| `cols` | u16 | no | 0 | Columns to freeze from left |
| `dry_run` | boolean | no | false | Preview only |

### `POST /api/freeze-panes/clear`

```json
{ "path": "data.xlsx", "sheet": "Sheet1" }
```

---

## Cell Operations

### `POST /api/cell/read`

```json
{ "path": "data.xlsx", "sheet": "Sheet1", "cell": "A1" }
```

### `POST /api/cell/write`

```json
{ "path": "data.xlsx", "sheet": "Sheet1", "cell": "A1", "value": "Hello" }
```

Value type is auto-inferred: numeric strings become numbers, `true`/`false` become booleans.

---

## Range Operations

### `POST /api/range/read`

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `path` | string | yes | — | File path |
| `sheet` | string | yes | — | Sheet name |
| `range` | string | yes | — | e.g. `"A1:C5"` |
| `mode` | string | no | `"detailed"` | `detailed`, `compact`, `csv` |
| `truncate` | u32 | no | — | Max rows to return |

### `POST /api/range/write`

```json
{ "path": "data.xlsx", "sheet": "Sheet1", "range": "A1:B2", "data": [["name","age"],["Alice",30]] }
```

### `POST /api/range/write-from-csv`

```json
{ "path": "data.xlsx", "sheet": "Sheet1", "range": "A1:C10", "csv_path": "input.csv" }
```

### `POST /api/range/clear`

```json
{ "path": "data.xlsx", "sheet": "Sheet1", "range": "A1:Z100" }
```

---

## Batch Operations

### `POST /api/batch/modify`

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `path` | string | yes | — | File path |
| `operations` | array | yes | — | Operation list |
| `strategy` | string | no | `"best-effort"` | `best-effort`, `all-or-nothing`, `dry-run` |
| `dry_run` | boolean | no | false | Preview all operations |
| `validate_only` | boolean | no | false | Validate only, no execution |

Supported operation types:

| op | Required fields |
|----|----------------|
| `write_cell` | `sheet`, `row`, `col`, `value` |
| `write_range` | `sheet`, `range`, `data` |
| `add_sheet` | `name` |
| `delete_sheet` | `name` |
| `rename_sheet` | `old_name`, `new_name` |
| `set_format` | `sheet`, `range`, `style` |
| `merge_cells` | `sheet`, `range` |
| `append_row` | `sheet`, `values` |
| `insert_row` | `sheet`, `row`, `values` |
| `delete_row` | `sheet`, `start_row`, `end_row` |
| `set_formula` | `sheet`, `cell`, `formula` |

### `POST /api/batch/validate_formula`

```json
{ "path": "data.xlsx", "sheet": "Sheet1", "formula": "=SUM(A1:B10)" }
```

---

## Data Processing

### `POST /api/data/append-row`

```json
{ "path": "data.xlsx", "sheet": "Sheet1", "values": ["Alice", 30, "Engineer"] }
```

### `POST /api/data/insert-row`

```json
{ "path": "data.xlsx", "sheet": "Sheet1", "row": 3, "values": ["Bob", 25, "Designer"] }
```

### `POST /api/data/delete-row`

```json
{ "path": "data.xlsx", "sheet": "Sheet1", "row": 3 }
```

### `POST /api/data/filter`

Operators: `eq`, `ne`, `gt`, `lt`, `gte`, `lte`, `contains`.

```json
{
  "path": "data.xlsx",
  "sheet": "Sheet1",
  "conditions": [
    { "column": 2, "operator": "gt", "value": "25" }
  ]
}
```

### `POST /api/data/sort`

```json
{
  "path": "data.xlsx",
  "sheet": "Sheet1",
  "sort_columns": [
    { "column": 2, "descending": false }
  ]
}
```

### `POST /api/data/dedup`

Pass empty `columns` array to compare entire rows.

```json
{ "path": "data.xlsx", "sheet": "Sheet1", "columns": [1, 2] }
```

### `POST /api/data/sql`

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `path` | string | yes | — | File path |
| `sheet` | string | yes | — | Sheet name (maps to table `t`) |
| `query` | string | yes | — | SQL statement |
| `session` | boolean | no | false | Enable session mode |
| `cache` | boolean | no | false | Enable query cache |

Columns are named A, B, C, ... matching Excel column letters. Supported clauses: SELECT, WHERE, ORDER BY, GROUP BY, LIMIT. Supported aggregates: COUNT, SUM, AVG, MIN, MAX.

```json
{
  "path": "data.xlsx",
  "sheet": "Sheet1",
  "query": "SELECT A, SUM(C) FROM t WHERE B > 100 GROUP BY A ORDER BY SUM(C) DESC LIMIT 10"
}
```

### `POST /api/data/sql_session`

Creates a session for multiple queries sharing context.

```json
{ "path": "data.xlsx", "sheet": "Sheet1" }
```

Returns `{ "session_id": "abc123..." }`.

### `DELETE /api/data/sql_session/:id`

Closes the specified SQL session.

---

## Formula Operations

### `POST /api/formula/set`

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `path` | string | yes | — | File path |
| `sheet` | string | yes | — | Sheet name |
| `cell` | string | yes | — | Cell reference |
| `formula` | string | yes | — | Formula string |
| `eval` | boolean | no | false | Evaluate after setting |
| `dry_run` | boolean | no | false | Preview only |

```json
{ "path": "data.xlsx", "sheet": "Sheet1", "cell": "C1", "formula": "=SUM(A1:B1)", "eval": true }
```

### `POST /api/formula/refresh`

```json
{ "path": "data.xlsx", "sheet": "Sheet1" }
```

### `POST /api/formula/read`

```json
{ "path": "data.xlsx", "sheet": "Sheet1", "cell": "C1" }
```

### `POST /api/formula/calc-mode`

```json
{ "path": "data.xlsx", "mode": "auto" }
```

`mode`: `auto` or `manual`.

### `POST /api/formula/evaluate`

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `path` | string | yes | — | File path |
| `sheet` | string | yes | — | Sheet name |
| `cell` | string | yes | — | Cell reference |
| `formula` | string | yes | — | Formula string |
| `no_eval` | boolean | no | false | Set formula without evaluating |
| `dry_run` | boolean | no | false | Preview only |

### `POST /api/formula/evaluate-batch`

```json
{
  "path": "data.xlsx",
  "sheet": "Sheet1",
  "formulas": [["A1", "=SUM(B1:B5)"], ["A2", "=AVERAGE(B1:B5)"]],
  "dry_run": false
}
```

### `POST /api/formula/trace_dependencies`

```json
{ "path": "data.xlsx", "sheet": "Sheet1", "cell": "C1" }
```

Returns precedents and dependents.

### `POST /api/formula/explain`

```json
{ "path": "data.xlsx", "sheet": "Sheet1", "cell": "C1", "language": "en" }
```

### `POST /api/formula/explain_logic`

```json
{ "path": "data.xlsx", "sheet": "Sheet1", "cell": "D1", "language": "zh" }
```

### `POST /api/formula/fill`

```json
{ "path": "data.xlsx", "sheet": "Sheet1", "source": "A1", "target_range": "A2:A10", "dry_run": false }
```

---

## Search

### `POST /api/search/workbook`

| Field | Values | Description |
|-------|--------|-------------|
| `match_type` | `contains`, `exact`, `regex` | Match mode |
| `search_type` | `both`, `value`, `formula` | What to search |
| `case_sensitive` | boolean | Case sensitivity |
| `sheets` | array or null | Limit to sheets, null = all |

```json
{
  "path": "data.xlsx",
  "query": {
    "pattern": "keyword",
    "match_type": "contains",
    "search_type": "both",
    "case_sensitive": false,
    "sheets": ["Sheet1", "Sheet2"]
  }
}
```

### `POST /api/search/sheet`

```json
{
  "path": "data.xlsx",
  "sheet": "Sheet1",
  "query": {
    "pattern": "keyword",
    "match_type": "contains",
    "search_type": "both",
    "case_sensitive": false
  }
}
```

---

## Formatting

### `POST /api/format/set`

Style properties: `bold`, `italic`, `underline`, `font_size`, `font_color` (hex), `bg_color` (hex), `num_format`, `alignment` (`{"horizontal":"center","vertical":"center"}`), `border` (`{"color":"#000000","style":"thin"}`).

```json
{
  "path": "data.xlsx",
  "sheet": "Sheet1",
  "range": "A1:A10",
  "style": {
    "bold": true,
    "font_size": 14,
    "font_color": "#FF0000",
    "bg_color": "#FFFF00",
    "border": { "color": "#000000", "style": "thin" },
    "alignment": { "horizontal": "center", "vertical": "center" }
  }
}
```

### `POST /api/cell/merge`

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `path` | string | yes | File path |
| `sheet` | string | yes | Sheet name |
| `range` | string | yes | Merge range |
| `value` | string | no | Cell value after merge |

---

## Chart

### `POST /api/chart/create`

Chart types: `column`, `bar`, `line`, `pie`, `area`, `scatter`.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `path` | string | yes | File path |
| `sheet` | string | yes | Sheet name |
| `range` | string | yes | Data range |
| `chart_type` | string | yes | Chart type |
| `title` | string | no | Chart title |
| `position` | string | no | Anchor cell (e.g. `"E5"`) |
| `trendline` | object | no | `{"trend_type":"linear","display_equation":true}` |
| `y_error_bars` | object | no | `{"error_type":"standard_error","direction":"both"}` |
| `x_error_bars` | object | no | `{"error_type":"fixed_value","value":1.0}` |
| `log_base` | u16 | no | Logarithmic scale base |
| `dry_run` | boolean | no | Preview only |

---

## Comments

### `POST /api/comments/get`

```json
{ "path": "data.xlsx", "sheet": "Sheet1", "cell": "A1" }
```

### `POST /api/comments/add`

```json
{ "path": "data.xlsx", "sheet": "Sheet1", "cell": "A1", "text": "Review needed" }
```

### `POST /api/comments/update`

```json
{ "path": "data.xlsx", "sheet": "Sheet1", "cell": "A1", "text": "Updated comment" }
```

### `POST /api/comments/delete`

```json
{ "path": "data.xlsx", "sheet": "Sheet1", "cell": "A1" }
```

---

## Named Ranges

### `POST /api/named_ranges/list`

```json
{ "path": "data.xlsx" }
```

### `POST /api/named_ranges/get_value`

```json
{ "path": "data.xlsx", "name": "SalesData" }
```

### `POST /api/named_ranges/create`

```json
{ "path": "data.xlsx", "name": "SalesData", "range": "A1:C10", "sheet": "Sheet1" }
```

### `POST /api/named_ranges/delete`

```json
{ "path": "data.xlsx", "name": "SalesData" }
```

---

## Conditional Formatting

### `POST /api/conditional_format/add`

Rule types: `cell_value`, `formula`, `above_average`, `top10`, `duplicate`, `text_contains`, `date_occurring`.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `path` | string | yes | File path |
| `sheet` | string | yes | Sheet name |
| `range` | string | yes | Target range |
| `rule_type` | string | yes | Rule type |
| `condition` | string | yes | Condition expression |
| `style` | object | no | Format style |
| `config` | object | no | Advanced config (DataBar/ColorScale/IconSet) |
| `dry_run` | boolean | no | Preview only |

```json
{
  "path": "data.xlsx",
  "sheet": "Sheet1",
  "range": "A1:A10",
  "rule_type": "cell_value",
  "condition": ">100",
  "style": { "font_color": "#FF0000", "bold": true }
}
```

### `POST /api/conditional_format/remove`

```json
{ "path": "data.xlsx", "sheet": "Sheet1", "range": "A1:A10" }
```

---

## VBA

### `POST /api/vba/export`

```json
{ "path": "data.xlsm", "output": "vba_output.bas" }
```

### `POST /api/vba/import`

```json
{ "path": "data.xlsm", "vba_path": "macro.bas" }
```

### `POST /api/vba/has`

```json
{ "path": "data.xlsm" }
```

Returns `{ "has_vba": true }`.

---

## Diff

### `POST /api/diff/file`

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `old_path` | string | yes | Old file path |
| `new_path` | string | yes | New file path |
| `sheet` | string | no | Limit to sheet, omit for all |
| `semantic` | boolean | no | Generate semantic report |

### `POST /api/diff/range`

```json
{
  "old_path": "old.xlsx",
  "new_path": "new.xlsx",
  "sheet": "Sheet1",
  "range": "A1:C10",
  "semantic": false
}
```

### `POST /api/diff/semantic`

```json
{ "old_path": "old.xlsx", "new_path": "new.xlsx" }
```

Returns structured semantic diff with Summary and Detail levels.

### `POST /api/diff/formula_dependencies`

```json
{ "old_path": "old.xlsx", "new_path": "new.xlsx", "sheet": "Sheet1" }
```

Returns dependency graph changes and circular reference detection.

---

## Table

### `POST /api/table/create`

```json
{
  "path": "data.xlsx",
  "config": { "sheet": "Sheet1", "range": "A1:D10", "name": "SalesTable", "style": 1 }
}
```

### `POST /api/table/remove`

```json
{ "path": "data.xlsx", "name": "SalesTable" }
```

### `POST /api/table/list`

```json
{ "path": "data.xlsx" }
```

### `POST /api/table/get`

```json
{ "path": "data.xlsx", "name": "SalesTable" }
```

---

## Data Validation

### `POST /api/data_validation/add`

```json
{
  "path": "data.xlsx",
  "sheet": "Sheet1",
  "config": {
    "range": "A1:A10",
    "validation_type": "list",
    "criteria": { "source": "Option1,Option2,Option3" }
  }
}
```

### `POST /api/data_validation/remove`

```json
{ "path": "data.xlsx", "sheet": "Sheet1", "range": "A1:A10" }
```

---

## Pivot Table

### `POST /api/pivot_table/create`

```json
{
  "path": "data.xlsx",
  "config": {
    "sheet": "Sheet1",
    "data_range": "A1:E100",
    "rows": ["Category"],
    "columns": ["Region"],
    "values": [{ "field": "Amount", "aggregation": "sum" }],
    "filters": ["Year"]
  }
}
```

---

## Slicer

### `POST /api/slicer/create`

```json
{
  "path": "data.xlsx",
  "config": { "sheet": "Sheet1", "pivot_table": "PivotTable1", "field": "Category", "position": "G2" }
}
```

---

## Sparkline

### `POST /api/sparkline/add`

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `path` | string | yes | — | File path |
| `sheet` | string | yes | — | Sheet name |
| `source_range` | string | yes | — | e.g. `"'Sheet1'!A1:E1"` |
| `sparkline_type` | string | no | `"line"` | `line`, `column`, `winlose` |
| `target_cell` | string | yes | — | Target cell |
| `style` | u8 | no | — | Style number (0-35) |
| `dry_run` | boolean | no | false | Preview only |

### `POST /api/sparkline/remove`

```json
{ "path": "data.xlsx", "sheet": "Sheet1", "target_cell": "F1" }
```

---

## Workbook Overview

### `POST /api/workbook/overview`

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `path` | string | yes | — | File path |
| `blueprint` | boolean | no | false | Detailed structure info |

### `POST /api/workbook/history`

```json
{ "path": "data.xlsx" }
```

### `POST /api/workbook/sheet_overview`

```json
{ "path": "data.xlsx", "sheet": "Sheet1" }
```

---

## Auto-Filter

### `POST /api/auto-filter/set`

```json
{ "path": "data.xlsx", "sheet": "Sheet1", "range": "A1:D100", "dry_run": false }
```

### `POST /api/auto-filter/remove`

```json
{ "path": "data.xlsx", "sheet": "Sheet1" }
```

### `POST /api/auto-filter/get`

```json
{ "path": "data.xlsx", "sheet": "Sheet1" }
```

---

## Sheet Protection

### `POST /api/protection/sheet/protect`

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `path` | string | yes | File path |
| `sheet` | string | yes | Sheet name |
| `password` | string | no | Protection password |
| `options` | object | no | `ProtectionOptions` config |
| `dry_run` | boolean | no | Preview only |

### `POST /api/protection/sheet/unprotect`

```json
{ "path": "data.xlsx", "sheet": "Sheet1" }
```

### `POST /api/protection/sheet/is-protected`

```json
{ "path": "data.xlsx", "sheet": "Sheet1" }
```

---

## Page Setup

### `POST /api/page-setup/configure`

```json
{
  "path": "data.xlsx",
  "sheet": "Sheet1",
  "config": {
    "orientation": "landscape",
    "paper_size": 9,
    "margins": { "top": 0.75, "bottom": 0.75, "left": 0.7, "right": 0.7 }
  }
}
```

### `POST /api/page-setup/page-breaks/set`

```json
{
  "path": "data.xlsx",
  "config": { "sheet": "Sheet1", "row_breaks": [10, 25], "column_breaks": [5] }
}
```

### `POST /api/page-setup/page-breaks/clear`

```json
{ "path": "data.xlsx", "sheet": "Sheet1" }
```

---

## Image & Shapes

### `POST /api/image/insert`

```json
{
  "path": "data.xlsx",
  "config": { "sheet": "Sheet1", "image_path": "logo.png", "anchor_cell": "B2", "width": 200, "height": 100 }
}
```

### `POST /api/image/remove`

```json
{ "path": "data.xlsx", "sheet": "Sheet1", "anchor_cell": "B2" }
```

### `POST /api/image/shape/insert`

Supported shapes: `rectangle`, `ellipse`, `line`.

```json
{
  "path": "data.xlsx",
  "config": { "sheet": "Sheet1", "shape_type": "rectangle", "anchor_cell": "D5", "width": 100, "height": 50 }
}
```
