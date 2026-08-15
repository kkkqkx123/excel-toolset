## 批量操作

### POST /api/batch/modify
执行批量操作。

**请求体**：
```json
{
  "path": "data.xlsx",
  "operations": [
    {
      "op": "write_cell",
      "sheet": "Sheet1",
      "row": 1,
      "col": 1,
      "value": "Hello"
    },
    {
      "op": "add_sheet",
      "name": "NewSheet"
    }
  ],
  "strategy": "best-effort",
  "dry_run": false,
  "validate_only": false
}
```

**字段**：
| 字段 | 类型 | 必选 | 默认值 | 说明 |
|------|------|------|--------|------|
| path | string | 是 | - | 文件路径 |
| operations | array | 是 | - | 操作列表 |
| strategy | string | 否 | `best-effort` | 策略：`best-effort`（失败继续）、`all-or-nothing`（事务式）、`dry-run`（仿真） |
| dry_run | boolean | 否 | false | 模拟执行不写入 |
| validate_only | boolean | 否 | false | 仅验证请求不执行 |

**支持的操作类型 (`op`)**：
| 操作 | 字段 |
|------|------|
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

### POST /api/batch/validate_formula
验证公式引用有效性。

**请求体**：
```json
{
  "path": "data.xlsx",
  "sheet": "Sheet1",
  "formula": "=SUM(A1:B10)"
}
```

---

