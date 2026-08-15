## 公式操作

### POST /api/formula/set
设置单元格公式。

**请求体**：
```json
{
  "path": "data.xlsx",
  "sheet": "Sheet1",
  "cell": "C1",
  "formula": "=SUM(A1:B1)",
  "eval": true
}
```

**字段**：
| 字段 | 类型 | 必选 | 默认值 | 说明 |
|------|------|------|--------|------|
| path | string | 是 | - | 文件路径 |
| sheet | string | 是 | - | 工作表名 |
| cell | string | 是 | - | 单元格引用 |
| formula | string | 是 | - | 公式字符串 |
| eval | boolean | 否 | false | 设置后立即求值 |
| dry_run | boolean | 否 | false | 模拟执行不写入 |

### POST /api/formula/refresh
刷新工作表中所有公式计算结果。

**请求体**：
```json
{
  "path": "data.xlsx",
  "sheet": "Sheet1"
}
```

### POST /api/formula/read
读取单元格公式。

**请求体**：
```json
{
  "path": "data.xlsx",
  "sheet": "Sheet1",
  "cell": "C1"
}
```

**响应**：
```json
{
  "success": true,
  "data": {
    "formula": "=SUM(A1:B1)"
  }
}
```

### POST /api/formula/calc-mode
设置公式计算模式。

**请求体**：
```json
{
  "path": "data.xlsx",
  "mode": "auto"
}
```
- `mode`：`auto`（自动）或 `manual`（手动）

### POST /api/formula/trace_dependencies
追踪单元格公式依赖链。

**请求体**：
```json
{
  "path": "data.xlsx",
  "sheet": "Sheet1",
  "cell": "C1"
}
```

### POST /api/formula/explain
自然语言解释公式含义。

**请求体**：
```json
{
  "path": "data.xlsx",
  "sheet": "Sheet1",
  "cell": "C1",
  "language": "en"
}
```

### POST /api/formula/explain_logic
解释公式的逻辑结构。

**请求体**：
```json
{
  "path": "data.xlsx",
  "sheet": "Sheet1",
  "cell": "D1",
  "language": "zh"
}
```

### POST /api/formula/evaluate
设置公式并求值。

**请求体**：
```json
{
  "path": "data.xlsx",
  "sheet": "Sheet1",
  "cell": "C1",
  "formula": "=SUM(A1:B1)",
  "no_eval": false,
  "dry_run": false
}
```

**字段**：
| 字段 | 类型 | 必选 | 默认值 | 说明 |
|------|------|------|--------|------|
| path | string | 是 | - | 文件路径 |
| sheet | string | 是 | - | 工作表名 |
| cell | string | 是 | - | 单元格引用 |
| formula | string | 是 | - | 公式字符串 |
| no_eval | boolean | 否 | false | 仅设置公式不求值 |
| dry_run | boolean | 否 | false | 模拟执行不写入 |

### POST /api/formula/evaluate-batch
批量设置公式并求值。

**请求体**：
```json
{
  "path": "data.xlsx",
  "sheet": "Sheet1",
  "formulas": [["A1", "=SUM(B1:B5)"], ["A2", "=AVERAGE(B1:B5)"]],
  "dry_run": false
}
```

### POST /api/formula/fill
自动填充公式。

**请求体**：
```json
{
  "path": "data.xlsx",
  "sheet": "Sheet1",
  "source": "A1",
  "target_range": "A2:A10",
  "dry_run": false
}
```

---

