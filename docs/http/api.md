# Excel HTTP API 参考

## 服务启动

```bash
cargo run --package excel-http --release
```

默认监听 `127.0.0.1:3000`，端口可通过 `PORT` 环境变量覆盖。

## 通用约定

### 请求方法

除 `/health` 使用 GET 外，所有 API 端点统一使用 POST 方法，参数通过 JSON 请求体传递。

### 响应格式

所有接口统一 `ApiResponse<T>` 格式：

```json
{
  "success": true,
  "data": { ... },
  "error": null
}
```

错误响应：
```json
{
  "success": false,
  "data": null,
  "error": {
    "code": "ERROR_CODE",
    "message": "错误描述"
  }
}
```

---

## 健康检查

### GET /health
返回服务状态。

**响应**：
```json
{
  "success": true,
  "data": {
    "status": "ok",
    "timestamp": "2026-06-15T10:00:00Z"
  }
}
```

---

## 文件操作

### POST /api/file/info
获取 Excel 文件信息。

**请求体**：
```json
{
  "path": "data.xlsx"
}
```

**响应**：
```json
{
  "success": true,
  "data": {
    "sheet_count": 3,
    "sheets": ["Sheet1", "Sheet2", "Sheet3"],
    "file_hash": "abcd1234..."
  }
}
```

### POST /api/file/create
创建新的 Excel 文件。

**请求体**：
```json
{
  "path": "output.xlsx",
  "sheet": "Sheet1"
}
```

**字段**：
| 字段 | 类型 | 必选 | 默认值 | 说明 |
|------|------|------|--------|------|
| path | string | 是 | - | 文件路径 |
| sheet | string | 否 | `Sheet1` | 初始工作表名 |

### POST /api/file/backup
创建文件备份。

**请求体**：
```json
{
  "path": "data.xlsx",
  "output": "/tmp/backup.xlsx"
}
```

**字段**：
| 字段 | 类型 | 必选 | 说明 |
|------|------|------|------|
| path | string | 是 | 文件路径 |
| output | string | 否 | 额外复制备份到指定位置 |

### POST /api/file/rollback
从备份回滚文件。

**请求体**：
```json
{
  "path": "data.xlsx",
  "backup_path": ".backups/backup_20260615.xlsx"
}
```

---

## 工作表操作

### POST /api/sheet/list
列出所有工作表。

**请求体**：
```json
{
  "path": "data.xlsx"
}
```

### POST /api/sheet/add
添加新工作表。

**请求体**：
```json
{
  "path": "data.xlsx",
  "name": "NewSheet"
}
```

### POST /api/sheet/delete
删除工作表。

**请求体**：
```json
{
  "path": "data.xlsx",
  "name": "Sheet2"
}
```

### POST /api/sheet/rename
重命名工作表。

**请求体**：
```json
{
  "path": "data.xlsx",
  "old_name": "OldSheet",
  "new_name": "NewSheet"
}
```

### POST /api/sheet/visibility
设置工作表可见性。

**请求体**：
```json
{
  "path": "data.xlsx",
  "name": "Sheet1",
  "visibility": "hidden"
}
```

**字段**：
| 字段 | 类型 | 必选 | 说明 |
|------|------|------|------|
| path | string | 是 | 文件路径 |
| name | string | 是 | 工作表名 |
| visibility | string | 是 | `visible`（可见）、`hidden`（隐藏）、`very_hidden`（深度隐藏） |
| dry_run | boolean | 否 | 模拟执行不写入 |

---

## 冻结窗格

### POST /api/freeze-panes/set
设置冻结窗格。

**请求体**：
```json
{
  "path": "data.xlsx",
  "sheet": "Sheet1",
  "rows": 1,
  "cols": 0,
  "dry_run": false
}
```

**字段**：
| 字段 | 类型 | 必选 | 默认值 | 说明 |
|------|------|------|--------|------|
| path | string | 是 | - | 文件路径 |
| sheet | string | 是 | - | 工作表名 |
| rows | u32 | 否 | 0 | 从顶部冻结的行数 |
| cols | u16 | 否 | 0 | 从左侧冻结的列数 |
| dry_run | boolean | 否 | false | 模拟执行不写入 |

### POST /api/freeze-panes/clear
清除冻结窗格。

**请求体**：
```json
{
  "path": "data.xlsx",
  "sheet": "Sheet1"
}
```

---

## 单元格操作

### POST /api/cell/read
读取单元格值。

**请求体**：
```json
{
  "path": "data.xlsx",
  "sheet": "Sheet1",
  "cell": "A1"
}
```

### POST /api/cell/write
写入单元格。

**请求体**：
```json
{
  "path": "data.xlsx",
  "sheet": "Sheet1",
  "cell": "A1",
  "value": "Hello"
}
```

---

## 区域操作

### POST /api/range/read
读取区域数据。

**请求体**：
```json
{
  "path": "data.xlsx",
  "sheet": "Sheet1",
  "range": "A1:C5",
  "mode": "detailed",
  "truncate": 100
}
```

**字段**：
| 字段 | 类型 | 必选 | 默认值 | 说明 |
|------|------|------|--------|------|
| path | string | 是 | - | 文件路径 |
| sheet | string | 是 | - | 工作表名 |
| range | string | 是 | - | 区域引用，如 `A1:C5` |
| mode | string | 否 | `detailed` | 输出模式：`detailed`（含行号列号）、`compact`（纯数据矩阵）、`csv` |
| truncate | u32 | 否 | - | 限制返回行数 |

### POST /api/range/write
写入区域数据。

**请求体**：
```json
{
  "path": "data.xlsx",
  "sheet": "Sheet1",
  "range": "A1:B2",
  "data": [["name", "age"], ["Alice", 30]]
}
```

### POST /api/range/write-from-csv
从 CSV 文件写入区域。

**请求体**：
```json
{
  "path": "data.xlsx",
  "sheet": "Sheet1",
  "range": "A1:C10",
  "csv_path": "input.csv"
}
```

### POST /api/range/clear
清空区域内容。

**请求体**：
```json
{
  "path": "data.xlsx",
  "sheet": "Sheet1",
  "range": "A1:Z100"
}
```

---

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

## 数据处理

### POST /api/data/append-row
追加行到工作表末尾。

**请求体**：
```json
{
  "path": "data.xlsx",
  "sheet": "Sheet1",
  "values": ["Alice", 30, "Engineer"]
}
```

### POST /api/data/insert-row
在指定行插入数据。

**请求体**：
```json
{
  "path": "data.xlsx",
  "sheet": "Sheet1",
  "row": 3,
  "values": ["Bob", 25, "Designer"]
}
```

### POST /api/data/delete-row
删除指定行。

**请求体**：
```json
{
  "path": "data.xlsx",
  "sheet": "Sheet1",
  "row": 3
}
```

### POST /api/data/filter
过滤行数据。

**请求体**：
```json
{
  "path": "data.xlsx",
  "sheet": "Sheet1",
  "conditions": [
    {
      "column": 2,
      "operator": "gt",
      "value": "25"
    }
  ]
}
```

**支持的操作符**：`eq`, `ne`, `gt`, `lt`, `gte`, `lte`, `contains`

### POST /api/data/sort
排序工作表。

**请求体**：
```json
{
  "path": "data.xlsx",
  "sheet": "Sheet1",
  "sort_columns": [
    {
      "column": 2,
      "descending": false
    }
  ]
}
```

### POST /api/data/dedup
去除重复行。

**请求体**：
```json
{
  "path": "data.xlsx",
  "sheet": "Sheet1",
  "columns": [1, 2]
}
```
- `columns` 为空数组时按整行去重

### POST /api/data/sql
SQL 查询工作表数据。

**请求体**：
```json
{
  "path": "data.xlsx",
  "sheet": "Sheet1",
  "query": "SELECT * FROM t WHERE A > 10 ORDER BY B DESC",
  "session": false,
  "cache": false
}
```

**字段**：
| 字段 | 类型 | 必选 | 默认值 | 说明 |
|------|------|------|--------|------|
| path | string | 是 | - | 文件路径 |
| sheet | string | 是 | - | 工作表名 |
| query | string | 是 | - | SQL 语句 |
| session | boolean | 否 | false | 启用会话模式 |
| cache | boolean | 否 | false | 启用查询缓存 |

### POST /api/data/sql_session
创建 SQL 查询会话，支持多次查询共享上下文。

**请求体**：
```json
{
  "path": "data.xlsx",
  "sheet": "Sheet1"
}
```

**响应**：
```json
{
  "success": true,
  "data": {
    "session_id": "abc123..."
  }
}
```

### DELETE /api/data/sql_session/:id
关闭指定的 SQL 会话。

---

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

## 搜索

### POST /api/search/workbook
全工作簿搜索。

**请求体**：
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

**字段说明**：
| 字段 | 可选值 | 说明 |
|------|--------|------|
| match_type | `contains`, `exact`, `regex` | 匹配方式 |
| search_type | `both`, `value`, `formula` | 搜索内容类型 |
| case_sensitive | `true`/`false` | 是否区分大小写 |
| sheets | 数组或 null | 限定搜索的工作表，null 表示全部 |

### POST /api/search/sheet
单工作表搜索。

**请求体**：
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

## 格式操作

### POST /api/format/set
设置单元格格式。

**请求体**：
```json
{
  "path": "data.xlsx",
  "sheet": "Sheet1",
  "range": "A1:A10",
  "style": {
    "bold": true,
    "italic": false,
    "font_size": 14,
    "font_color": "#FF0000",
    "bg_color": "#FFFF00",
    "border": {
      "color": "#000000",
      "style": "thin"
    },
    "alignment": {
      "horizontal": "center",
      "vertical": "center"
    }
  }
}
```

### POST /api/cell/merge
合并单元格。

**请求体**：
```json
{
  "path": "data.xlsx",
  "sheet": "Sheet1",
  "range": "A1:C3",
  "value": "合并后的值"
}
```

**字段**：
| 字段 | 类型 | 必选 | 说明 |
|------|------|------|------|
| path | string | 是 | 文件路径 |
| sheet | string | 是 | 工作表名 |
| range | string | 是 | 合并区域 |
| value | string | 否 | 合并后单元格的值 |

---

## 图表

### POST /api/chart/create
创建图表。

**请求体**：
```json
{
  "path": "data.xlsx",
  "sheet": "Sheet1",
  "range": "A1:B10",
  "chart_type": "column",
  "title": "销量统计",
  "position": "E5",
  "trendline": {
    "trend_type": "linear",
    "display_equation": true
  },
  "y_error_bars": {
    "error_type": "standard_error",
    "direction": "both"
  },
  "x_error_bars": {
    "error_type": "fixed_value",
    "value": 1.0
  },
  "log_base": 10
}
```

**字段**：
| 字段 | 类型 | 必选 | 说明 |
|------|------|------|------|
| path | string | 是 | 文件路径 |
| sheet | string | 是 | 工作表名 |
| range | string | 是 | 数据区域 |
| chart_type | string | 是 | 图表类型 |
| title | string | 否 | 图表标题 |
| position | string | 否 | 放置位置（单元格引用） |
| trendline | object | 否 | 趋势线配置 |
| y_error_bars | object | 否 | Y 轴误差线配置 |
| x_error_bars | object | 否 | X 轴误差线配置 |
| log_base | u16 | 否 | 对数刻度底数 |
| dry_run | boolean | 否 | 模拟执行不写入 |

**支持的 chart_type**：
| 值 | 说明 |
|----|------|
| `column` | 柱状图 |
| `bar` | 条形图 |
| `line` | 折线图 |
| `pie` | 饼图 |
| `area` | 面积图 |
| `scatter` | 散点图 |

---

## 批注

### POST /api/comments/get
获取单元格批注。

**请求体**：
```json
{
  "path": "data.xlsx",
  "sheet": "Sheet1",
  "cell": "A1"
}
```

### POST /api/comments/add
添加批注。

**请求体**：
```json
{
  "path": "data.xlsx",
  "sheet": "Sheet1",
  "cell": "A1",
  "text": "这是一条批注"
}
```

### POST /api/comments/update
更新批注。

**请求体**：
```json
{
  "path": "data.xlsx",
  "sheet": "Sheet1",
  "cell": "A1",
  "text": "更新后的批注"
}
```

### POST /api/comments/delete
删除批注。

**请求体**：
```json
{
  "path": "data.xlsx",
  "sheet": "Sheet1",
  "cell": "A1"
}
```

---

## 命名范围

### POST /api/named_ranges/list
列出所有命名范围。

**请求体**：
```json
{
  "path": "data.xlsx"
}
```

### POST /api/named_ranges/get_value
获取命名范围的值。

**请求体**：
```json
{
  "path": "data.xlsx",
  "name": "SalesData"
}
```

### POST /api/named_ranges/create
创建命名范围。

**请求体**：
```json
{
  "path": "data.xlsx",
  "name": "SalesData",
  "range": "A1:C10",
  "sheet": "Sheet1"
}
```

### POST /api/named_ranges/delete
删除命名范围。

**请求体**：
```json
{
  "path": "data.xlsx",
  "name": "SalesData"
}
```

---

## 条件格式

### POST /api/conditional_format/add
添加条件格式规则。

**请求体**：
```json
{
  "path": "data.xlsx",
  "sheet": "Sheet1",
  "range": "A1:A10",
  "rule_type": "cell_value",
  "condition": ">100",
  "style": {
    "font_color": "#FF0000",
    "bold": true
  },
  "config": {
    "fill_color": "#00FF00"
  }
}
```

**字段**：
| 字段 | 类型 | 必选 | 说明 |
|------|------|------|------|
| path | string | 是 | 文件路径 |
| sheet | string | 是 | 工作表名 |
| range | string | 是 | 应用区域 |
| rule_type | string | 是 | 规则类型 |
| condition | string | 是 | 条件表达式 |
| style | object | 否 | 格式样式 |
| config | object | 否 | 高级配置（DataBar/ColorScale/IconSet） |
| dry_run | boolean | 否 | 模拟执行不写入 |

**支持的 rule_type**：
| 值 | 说明 |
|----|------|
| `cell_value` | 单元格值条件 |
| `formula` | 公式条件 |
| `above_average` | 高于平均值 |
| `top10` | 前 N 项 |
| `duplicate` | 重复值高亮 |
| `text_contains` | 文本包含 |
| `date_occurring` | 日期条件 |

### POST /api/conditional_format/remove
移除条件格式规则。

**请求体**：
```json
{
  "path": "data.xlsx",
  "sheet": "Sheet1",
  "range": "A1:A10"
}
```

---

## VBA

### POST /api/vba/export
导出 VBA 宏代码。

**请求体**：
```json
{
  "path": "data.xlsm",
  "output": "vba_output.bas"
}
```

### POST /api/vba/import
导入 VBA 宏代码。

**请求体**：
```json
{
  "path": "data.xlsm",
  "vba_path": "macro.bas"
}
```

### POST /api/vba/has
检查文件是否包含 VBA 宏。

**请求体**：
```json
{
  "path": "data.xlsm"
}
```

**响应**：
```json
{
  "success": true,
  "data": {
    "has_vba": true
  }
}
```

---

## Diff 对比

### POST /api/diff/file
对比两个 Excel 文件的差异。

**请求体**：
```json
{
  "old_path": "old.xlsx",
  "new_path": "new.xlsx",
  "sheet": "Sheet1",
  "semantic": false
}
```

**字段**：
| 字段 | 类型 | 必选 | 说明 |
|------|------|------|------|
| old_path | string | 是 | 旧文件路径 |
| new_path | string | 是 | 新文件路径 |
| sheet | string | 否 | 限定工作表，不传则对比全部 |
| semantic | boolean | 否 | 生成语义级差异报告 |

### POST /api/diff/range
对比两个文件指定区域的差异。

**请求体**：
```json
{
  "old_path": "old.xlsx",
  "new_path": "new.xlsx",
  "sheet": "Sheet1",
  "range": "A1:C10",
  "semantic": false
}
```

### POST /api/diff/semantic
生成语义级差异报告。

**请求体**：
```json
{
  "old_path": "old.xlsx",
  "new_path": "new.xlsx"
}
```
返回结构化的语义差异摘要，包含 Summary 和 Detail 两种详细度。

### POST /api/diff/formula_dependencies
对比两个文件的公式依赖图差异。

**请求体**：
```json
{
  "old_path": "old.xlsx",
  "new_path": "new.xlsx",
  "sheet": "Sheet1"
}
```
返回依赖图变更详情，包含循环检测结果。

---

## 表格

### POST /api/table/create
创建表格。

**请求体**：
```json
{
  "path": "data.xlsx",
  "config": {
    "sheet": "Sheet1",
    "range": "A1:D10",
    "name": "SalesTable",
    "style": 1
  }
}
```

### POST /api/table/remove
移除表格。

**请求体**：
```json
{
  "path": "data.xlsx",
  "name": "SalesTable"
}
```

### POST /api/table/list
列出所有表格。

**请求体**：
```json
{
  "path": "data.xlsx"
}
```

### POST /api/table/get
获取表格详细信息。

**请求体**：
```json
{
  "path": "data.xlsx",
  "name": "SalesTable"
}
```

---

## 数据验证

### POST /api/data_validation/add
添加数据验证规则。

**请求体**：
```json
{
  "path": "data.xlsx",
  "sheet": "Sheet1",
  "config": {
    "range": "A1:A10",
    "validation_type": "list",
    "criteria": {
      "source": "Option1,Option2,Option3"
    }
  }
}
```

### POST /api/data_validation/remove
移除数据验证规则。

**请求体**：
```json
{
  "path": "data.xlsx",
  "sheet": "Sheet1",
  "range": "A1:A10"
}
```

---

## 数据透视表

### POST /api/pivot_table/create
创建数据透视表。

**请求体**：
```json
{
  "path": "data.xlsx",
  "config": {
    "sheet": "Sheet1",
    "data_range": "A1:E100",
    "rows": ["Category"],
    "columns": ["Region"],
    "values": [
      {
        "field": "Amount",
        "aggregation": "sum"
      }
    ],
    "filters": ["Year"]
  }
}
```

---

## 切片器

### POST /api/slicer/create
创建切片器。

**请求体**：
```json
{
  "path": "data.xlsx",
  "config": {
    "sheet": "Sheet1",
    "pivot_table": "PivotTable1",
    "field": "Category",
    "position": "G2"
  }
}
```

---

## 迷你图

### POST /api/sparkline/add
添加迷你图。

**请求体**：
```json
{
  "path": "data.xlsx",
  "sheet": "Sheet1",
  "source_range": "'Sheet1'!A1:E1",
  "sparkline_type": "line",
  "target_cell": "F1",
  "style": 1,
  "dry_run": false
}
```

**字段**：
| 字段 | 类型 | 必选 | 默认值 | 说明 |
|------|------|------|--------|------|
| path | string | 是 | - | 文件路径 |
| sheet | string | 是 | - | 工作表名 |
| source_range | string | 是 | - | 数据源区域 |
| sparkline_type | string | 否 | `line` | `line`、`column`、`winlose` |
| target_cell | string | 是 | - | 目标单元格 |
| style | u8 | 否 | - | 样式编号 (0-35) |
| dry_run | boolean | 否 | false | 模拟执行不写入 |

### POST /api/sparkline/remove
移除迷你图。

**请求体**：
```json
{
  "path": "data.xlsx",
  "sheet": "Sheet1",
  "target_cell": "F1"
}
```

---

## 工作簿概览

### POST /api/workbook/overview
获取工作簿概览（含蓝图模式）。

**请求体**：
```json
{
  "path": "data.xlsx",
  "blueprint": true
}
```

**字段**：
| 字段 | 类型 | 必选 | 默认值 | 说明 |
|------|------|------|--------|------|
| path | string | 是 | - | 文件路径 |
| blueprint | boolean | 否 | false | 输出工作簿蓝图（详细结构信息） |

### POST /api/workbook/history
获取文件操作历史。

**请求体**：
```json
{
  "path": "data.xlsx"
}
```

### POST /api/workbook/sheet_overview
获取单个工作表概览。

**请求体**：
```json
{
  "path": "data.xlsx",
  "sheet": "Sheet1"
}
```

---

## 自动筛选

### POST /api/auto-filter/set
设置自动筛选。

**请求体**：
```json
{
  "path": "data.xlsx",
  "sheet": "Sheet1",
  "range": "A1:D100",
  "dry_run": false
}
```

### POST /api/auto-filter/remove
移除自动筛选。

**请求体**：
```json
{
  "path": "data.xlsx",
  "sheet": "Sheet1"
}
```

### POST /api/auto-filter/get
获取自动筛选信息。

**请求体**：
```json
{
  "path": "data.xlsx",
  "sheet": "Sheet1"
}
```

---

## 工作表保护

### POST /api/protection/sheet/protect
保护工作表。

**请求体**：
```json
{
  "path": "data.xlsx",
  "sheet": "Sheet1",
  "password": "secret",
  "options": {
    "select_locked_cells": false,
    "select_unlocked_cells": true,
    "format_cells": false
  },
  "dry_run": false
}
```

**字段**：
| 字段 | 类型 | 必选 | 说明 |
|------|------|------|------|
| path | string | 是 | 文件路径 |
| sheet | string | 是 | 工作表名 |
| password | string | 否 | 保护密码 |
| options | object | 否 | ProtectionOptions 配置 |
| dry_run | boolean | 否 | 模拟执行不写入 |

### POST /api/protection/sheet/unprotect
解除工作表保护。

**请求体**：
```json
{
  "path": "data.xlsx",
  "sheet": "Sheet1"
}
```

### POST /api/protection/sheet/is-protected
检查工作表保护状态。

**请求体**：
```json
{
  "path": "data.xlsx",
  "sheet": "Sheet1"
}
```

---

## 页面设置

### POST /api/page-setup/configure
配置页面设置（纸张大小、方向、边距等）。

**请求体**：
```json
{
  "path": "data.xlsx",
  "sheet": "Sheet1",
  "config": {
    "orientation": "landscape",
    "paper_size": 9,
    "margins": {
      "top": 0.75,
      "bottom": 0.75,
      "left": 0.7,
      "right": 0.7
    }
  }
}
```

### POST /api/page-setup/page-breaks/set
设置分页符。

**请求体**：
```json
{
  "path": "data.xlsx",
  "config": {
    "sheet": "Sheet1",
    "row_breaks": [10, 25],
    "column_breaks": [5]
  }
}
```

### POST /api/page-setup/page-breaks/clear
清除分页符。

**请求体**：
```json
{
  "path": "data.xlsx",
  "sheet": "Sheet1"
}
```

---

## 图片/形状

### POST /api/image/insert
插入图片。

**请求体**：
```json
{
  "path": "data.xlsx",
  "config": {
    "sheet": "Sheet1",
    "image_path": "logo.png",
    "anchor_cell": "B2",
    "width": 200,
    "height": 100
  }
}
```

### POST /api/image/remove
移除图片。

**请求体**：
```json
{
  "path": "data.xlsx",
  "sheet": "Sheet1",
  "anchor_cell": "B2"
}
```

### POST /api/image/shape/insert
插入形状（矩形、椭圆、线条）。

**请求体**：
```json
{
  "path": "data.xlsx",
  "config": {
    "sheet": "Sheet1",
    "shape_type": "rectangle",
    "anchor_cell": "D5",
    "width": 100,
    "height": 50
  }
}
```
