---
name: excel-diff
description: Compare Excel files at file, range, semantic, and formula-dependency levels. Install as a Git diff driver for version-controlled Excel file comparison.
arguments:
  - name: old_path
    description: Absolute path to the old/left Excel file
    required: true
  - name: new_path
    description: Absolute path to the new/right Excel file
    required: true
  - name: sheet
    description: Worksheet name (optional, compares all sheets if omitted)
    required: false
---

# Excel Diff

Use `excel-cli diff` to compare Excel files programmatically. Output defaults to JSON; pass `--format text` for human-readable diff output.

## Invocation

```bash
cargo build --package excel-cli --release
```

The binary is at `./target/release/excel-cli`.

Global flags: `--pretty` (pretty-print JSON), `--format json|text` (text format only supported by diff commands).

## File-Level Diff

```bash
excel-cli diff file <old-path> <new-path> [--sheet <name>] [--semantic]
```

Compares two workbooks. Without `--sheet`, compares all sheets. With `--semantic`, generates a natural-language change summary.

Human-readable output:

```bash
excel-cli --format text diff file old.xlsx new.xlsx
```

## Range Diff

```bash
excel-cli diff range <old-path> <new-path> <sheet> <range> [--semantic]
```

Compares a specific range within the same sheet across two files.

## Semantic Diff

```bash
excel-cli diff semantic <old-path> <new-path>
```

Generates a structured semantic change report. Converts raw cell-level diffs into business-readable descriptions like "Cell A1 in Sheet1 changed from 'Old Value' to 'New Value'".

## Formula Dependency Graph Diff

```bash
excel-cli diff formula-deps <old-path> <new-path> <sheet>
```

Compares formula dependency topology between two versions:
- Added dependency relationships
- Removed dependency relationships
- Modified dependency relationships
- Circular reference detection

## Git Integration

### Install Git Diff Driver

```bash
# Current repository (default patterns: *.xlsx, *.xls, *.xlsm, *.xlsb)
excel-cli diff install-git-driver

# Global installation (all repositories)
excel-cli diff install-git-driver --global

# Custom file patterns
excel-cli diff install-git-driver --patterns '*.xlsx,*.xlsm'
```

Adds to `.gitattributes` (repo) or `~/.config/git/attributes` (global):

```
*.xlsx diff=excel-diff
*.xls diff=excel-diff
*.xlsm diff=excel-diff
*.xlsb diff=excel-diff
```

After installation, `git diff` on Excel files uses the excel-diff engine for readable comparison instead of raw binary diffs.

### Uninstall Git Diff Driver

```bash
excel-cli diff uninstall-git-driver        # current repo
excel-cli diff uninstall-git-driver --global  # global
```

### One-Off Script

```bash
./scripts/install-global.sh
```

## Workflow Examples

### Reviewing Excel Changes

```bash
# Compare two versions entirely
excel-cli --format text diff file v1.xlsx v2.xlsx

# Focus on a specific sheet
excel-cli diff file v1.xlsx v2.xlsx --sheet "Financial Data"

# Semantic-level understanding
excel-cli diff semantic v1.xlsx v2.xlsx
```

### Tracking Formula Logic Changes

```bash
# Trace dependency changes
excel-cli diff formula-deps v1.xlsx v2.xlsx Sheet1

# Compare critical data ranges semantically
excel-cli diff range v1.xlsx v2.xlsx Sheet1 A1:F50 --semantic
```

### Git Workflow

```bash
# Install global driver once
excel-cli diff install-git-driver --global

# git diff now works on Excel files directly
git diff HEAD~1 data.xlsx
git log -p -- data.xlsx
```
