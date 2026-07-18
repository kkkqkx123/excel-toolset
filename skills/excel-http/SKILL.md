---
name: excel-http
description: Interact with Excel files via the excel-http REST API server. Covers all file, sheet, cell, range, formula, data, formatting, chart, and advanced feature operations through HTTP endpoints.
arguments:
  - name: base_url
    description: Base URL of the running excel-http server (default http://localhost:3000)
    required: false
  - name: path
    description: Path to the Excel file on the server filesystem
    required: false
  - name: sheet
    description: Worksheet name
    required: false
---

# Excel HTTP API

Use the `excel-http` REST API server to work with Excel files programmatically over HTTP. All endpoints accept JSON request bodies and return a unified `ApiResponse<T>` envelope.

## Server Startup

```bash
cargo run --package excel-http --release
```

The server listens on `127.0.0.1:3000` by default. Override the port with the `PORT` environment variable.

## Response Format

All responses use a unified envelope:

```json
{
  "success": true,
  "data": { "sheet_count": 3, "sheets": ["Sheet1", "Sheet2"] },
  "error": null
}
```

Error response:

```json
{
  "success": false,
  "data": null,
  "error": { "code": "FILE_NOT_FOUND", "message": "File does not exist" }
}
```

## Conventions

- All endpoints use `POST` except `/health` (GET) and `/api/data/sql_session/:id` (DELETE).
- Request parameters are sent in the JSON body — not as URL parameters.
- Most write endpoints accept a `dry_run` field (`false` by default) for preview.
- Safety guarantees: hash checksum → automatic backup → execute. Rollback via `/api/file/rollback`.

## Task Routing

Determine the operation domain, then consult `references/api-reference.md` for exact endpoint paths and request body schemas.

| Domain | Endpoint prefix |
|--------|----------------|
| **Health** | `GET /health` |
| **File** | `/api/file/*` — info, create, backup, rollback |
| **Sheet** | `/api/sheet/*` — list, add, delete, rename, visibility |
| **Cell** | `/api/cell/*` — read, write |
| **Range** | `/api/range/*` — read, write, write-from-csv, clear |
| **Batch** | `/api/batch/*` — modify (transactional), validate_formula |
| **Data** | `/api/data/*` — append-row, insert-row, delete-row, filter, sort, dedup, sql |
| **SQL Session** | `/api/data/sql_session` (POST create, DELETE close) |
| **Formula** | `/api/formula/*` — set, refresh, read, calc-mode, evaluate, evaluate-batch, fill, trace_dependencies, explain, explain_logic |
| **Search** | `/api/search/*` — workbook, sheet |
| **Format** | `/api/format/set`, `/api/cell/merge` |
| **Chart** | `/api/chart/create` |
| **Comments** | `/api/comments/*` — get, add, update, delete |
| **Named Ranges** | `/api/named_ranges/*` — list, get_value, create, delete |
| **Conditional Format** | `/api/conditional_format/*` — add, remove |
| **VBA** | `/api/vba/*` — export, import, has |
| **Diff** | `/api/diff/*` — file, range, semantic, formula_dependencies |
| **Table** | `/api/table/*` — create, remove, list, get |
| **Data Validation** | `/api/data_validation/*` — add, remove |
| **Pivot Table** | `/api/pivot_table/create` |
| **Slicer** | `/api/slicer/create` |
| **Sparkline** | `/api/sparkline/*` — add, remove |
| **Workbook Overview** | `/api/workbook/*` — overview, history, sheet_overview |
| **Auto-Filter** | `/api/auto-filter/*` — set, remove, get |
| **Protection** | `/api/protection/sheet/*` — protect, unprotect, is-protected |
| **Page Setup** | `/api/page-setup/*` — configure, page-breaks/set, page-breaks/clear |
| **Image** | `/api/image/*` — insert, remove, shape/insert |
| **Freeze Panes** | `/api/freeze-panes/*` — set, clear |

## Cross-Cutting Workflows

### Safe Write Pattern

Always preview with `dry_run: true` before applying:

```bash
# Step 1: preview
curl -s -X POST http://localhost:3000/api/cell/write \
  -H "Content-Type: application/json" \
  -d '{"path":"data.xlsx","sheet":"Sheet1","cell":"A1","value":"New","dry_run":true}'

# Step 2: execute
curl -s -X POST http://localhost:3000/api/cell/write \
  -H "Content-Type: application/json" \
  -d '{"path":"data.xlsx","sheet":"Sheet1","cell":"A1","value":"New"}'
```

### Read → Analyze → Write

```bash
# Read existing data
curl -s -X POST http://localhost:3000/api/range/read \
  -H "Content-Type: application/json" \
  -d '{"path":"report.xlsx","sheet":"Sheet1","range":"A1:D100","mode":"compact"}'

# SQL analysis
curl -s -X POST http://localhost:3000/api/data/sql \
  -H "Content-Type: application/json" \
  -d '{"path":"report.xlsx","sheet":"Sheet1","query":"SELECT A, SUM(D) FROM t GROUP BY A"}'

# Create result sheet and write
curl -s -X POST http://localhost:3000/api/sheet/add \
  -H "Content-Type: application/json" \
  -d '{"path":"report.xlsx","name":"Summary"}'

curl -s -X POST http://localhost:3000/api/range/write \
  -H "Content-Type: application/json" \
  -d '{"path":"report.xlsx","sheet":"Summary","range":"A1:B3","data":[["Category","Total"],["A",500],["B",300]]}'
```

### Batch Operations with Transactional Safety

```bash
curl -s -X POST http://localhost:3000/api/batch/modify \
  -H "Content-Type: application/json" \
  -d '{
    "path": "data.xlsx",
    "operations": [
      {"op": "rename_sheet", "old_name": "Sheet1", "new_name": "Data"},
      {"op": "write_cell", "sheet": "Data", "row": 1, "col": 1, "value": "Title"},
      {"op": "set_format", "sheet": "Data", "range": "A1:E1", "style": {"bold":true,"font_size":16}}
    ],
    "strategy": "all-or-nothing"
  }'
```

### Report Generation

```bash
# Create file
curl -s -X POST http://localhost:3000/api/file/create \
  -H "Content-Type: application/json" \
  -d '{"path":"report.xlsx","sheet":"Monthly Report"}'

# Write data
curl -s -X POST http://localhost:3000/api/range/write \
  -H "Content-Type: application/json" \
  -d '{"path":"report.xlsx","sheet":"Monthly Report","range":"A1:C4","data":[["Month","Revenue","Cost"],["Jan",1200,800],["Feb",1500,900]]}'

# Add formula
curl -s -X POST http://localhost:3000/api/formula/set \
  -H "Content-Type: application/json" \
  -d '{"path":"report.xlsx","sheet":"Monthly Report","cell":"C5","formula":"=SUM(C2:C4)","eval":true}'

# Style headers
curl -s -X POST http://localhost:3000/api/format/set \
  -H "Content-Type: application/json" \
  -d '{"path":"report.xlsx","sheet":"Monthly Report","range":"A1:C1","style":{"bold":true,"font_size":14,"bg_color":"#4472C4","font_color":"#FFFFFF"}}'

# Add chart
curl -s -X POST http://localhost:3000/api/chart/create \
  -H "Content-Type: application/json" \
  -d '{"path":"report.xlsx","sheet":"Monthly Report","range":"A1:B4","chart_type":"column","title":"Monthly Revenue","position":"E2"}'

# Freeze header row
curl -s -X POST http://localhost:3000/api/freeze-panes/set \
  -H "Content-Type: application/json" \
  -d '{"path":"report.xlsx","sheet":"Monthly Report","rows":1,"cols":0}'
```

## SQL Session Management

For multiple queries against the same sheet, use session mode to share context:

```bash
# Create session
SESSION=$(curl -s -X POST http://localhost:3000/api/data/sql_session \
  -H "Content-Type: application/json" \
  -d '{"path":"data.xlsx","sheet":"Sheet1"}' | jq -r '.data.session_id')

# Run queries (use session_id as needed by implementation)
# ...

# Close session when done
curl -s -X DELETE "http://localhost:3000/api/data/sql_session/$SESSION"
```

## Diff via HTTP

Unlike the CLI where diff warrants a separate skill due to its distinct invocation pattern, the HTTP API exposes diff as endpoints within the same server. Request bodies use `old_path` and `new_path`:

```bash
# File-level diff
curl -s -X POST http://localhost:3000/api/diff/file \
  -H "Content-Type: application/json" \
  -d '{"old_path":"v1.xlsx","new_path":"v2.xlsx","semantic":false}'

# Semantic diff
curl -s -X POST http://localhost:3000/api/diff/semantic \
  -H "Content-Type: application/json" \
  -d '{"old_path":"v1.xlsx","new_path":"v2.xlsx"}'

# Formula dependency diff
curl -s -X POST http://localhost:3000/api/diff/formula_dependencies \
  -H "Content-Type: application/json" \
  -d '{"old_path":"v1.xlsx","new_path":"v2.xlsx","sheet":"Sheet1"}'
```
