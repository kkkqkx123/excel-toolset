# Entry Points: CLI and HTTP

Both entry points are independent binary crates in the workspace. They share common logic through `excel-core` and `excel-diff` but have no dependencies on each other.

## CLI Entry (`excel-cli`)

### Command Tree

```
excel
├── file
│   ├── create <path> [--sheet]
│   ├── info <path>
│   ├── save <path>
│   └── backup <path> [--output]
├── sheet
│   ├── list <path>
│   ├── add <path> <name>
│   ├── delete <path> <name>
│   └── rename <path> <old> <new>
├── cell
│   ├── read <path> <sheet> <cell>
│   └── write <path> <sheet> <cell> <value> [--dry-run]
├── range
│   ├── read <path> <sheet> <range>
│   ├── write <path> <sheet> <range> <data> [--dry-run]
│   └── clear <path> <sheet> <range> [--dry-run]
├── data
│   ├── append-row <path> <sheet> <values...> [--dry-run]
│   ├── insert-row <path> <sheet> <row> <values...> [--dry-run]
│   ├── delete-row <path> <sheet> <row> [--dry-run]
│   ├── filter <path> <sheet> <column> <condition>
│   ├── sort <path> <sheet> <column> [--desc] [--dry-run]
│   ├── deduplicate <path> <sheet> [--column] [--dry-run]
│   └── sql <path> <query>
├── formula
│   ├── set <path> <sheet> <cell> <formula> [--dry-run]
│   └── refresh <path> <sheet> [--dry-run]
├── format
│   ├── set <path> <sheet> <range> <style-json> [--dry-run]
│   └── merge <path> <sheet> <range> [--dry-run]
├── chart
│   └── create <path> <sheet> <range> <type> [--dry-run]
├── vba
│   ├── export <path> <output>
│   └── import <path> <vba-file> [--dry-run]
├── diff
│   ├── file <old-path> <new-path> [--sheet] [--range]
│   └── range <old-path> <new-path> <sheet> <range>
└── rollback <path> <backup-timestamp>
```

### Output Format

Default output is JSON:

```json
{
  "success": true,
  "message": "Cell written successfully",
  "file_hash": "sha256:abc123...",
  "data": { /* operation-specific result */ },
  "diff": { /* diff if applicable */ },
  "backup_info": { /* backup metadata if not dry-run */ }
}
```

With `--pretty`, the JSON is formatted for human readability.

### Dry-run Support

All write operations support `--dry-run`. In this mode:
- No file modifications occur
- Expected diff is computed and returned
- Backup info is omitted

### Example Usage

```bash
# Read a cell
excel cell read test.xlsx Sheet1 A1

# Write a cell (dry-run preview)
excel cell write test.xlsx Sheet1 A1 "Hello World" --dry-run

# Create a diff
excel diff file old.xlsx new.xlsx
```

## HTTP Entry (`excel-http`)

### REST API Endpoints

```
GET    /api/file/info/{path}
POST   /api/file/create
POST   /api/file/save/{path}
POST   /api/file/backup/{path}

GET    /api/sheet/list/{path}
POST   /api/sheet/add
POST   /api/sheet/delete
POST   /api/sheet/rename

GET    /api/cell/read/{path}/{sheet}/{cell}
POST   /api/cell/write
GET    /api/range/read/{path}/{sheet}/{range}
POST   /api/range/write
POST   /api/range/clear

POST   /api/data/append-row
POST   /api/data/insert-row
POST   /api/data/delete-row
GET    /api/data/filter
POST   /api/data/sort
POST   /api/data/deduplicate
POST   /api/advanced/sql

POST   /api/formula/set
POST   /api/formula/refresh

POST   /api/format/set
POST   /api/cell/merge

POST   /api/advanced/chart

POST   /api/vba/export
POST   /api/vba/import

POST   /api/diff/file
POST   /api/diff/range
POST   /api/file/rollback
```

### Request/Response Format

All requests and responses use JSON.

Request body example:
```json
{
  "file_path": "test.xlsx",
  "sheet_name": "Sheet1",
  "cell_ref": "A1",
  "value": "Hello World",
  "dry_run": false
}
```

Response format matches CLI output:
```json
{
  "success": true,
  "message": "Cell written successfully",
  "file_hash": "sha256:abc123...",
  "data": { /* operation-specific result */ },
  "diff": { /* diff if applicable */ },
  "backup_info": { /* backup metadata if not dry-run */ }
}
```

### Example Usage

```bash
# Start the server
excel-http

# Read a cell
curl http://localhost:3000/api/cell/read/test.xlsx/Sheet1/A1

# Write a cell
curl -X POST http://localhost:3000/api/cell/write \
  -H "Content-Type: application/json" \
  -d '{"file_path":"test.xlsx","sheet_name":"Sheet1","cell_ref":"A1","value":"Hello World"}'
```

## Shared Logic Between Entries

Both entry points use the same underlying logic from `excel-core` and `excel-diff`:

1. **Parameter validation**: Both validate inputs before calling core functions
2. **Error handling**: Both convert internal errors to standardized JSON responses
3. **Security integration**: Both wrap write operations with security checks (hashing, backup)
4. **Diff generation**: Both can optionally generate diffs for write operations

However, they remain completely independent — changes to one do not affect the other.