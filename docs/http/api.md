# Excel HTTP API 参考

## 服务启动

```bash
cargo run --package excel-http --release
```

默认监听 `127.0.0.1:3000`。

## 通用约定

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

### 路径参数 vs 请求体

- `GET` 请求：路径参数（如 `/api/file/info/:path`）
- `POST` 请求：JSON 请求体

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

### GET /api/file/info/:path
获取 Excel 文件信息。

**路径参数**：
- `path`：文件路径

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

### GET /api/sheet/list/:path
列出所有工作表。

**路径参数**：
- `path`：文件路径

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

---

## 单元格操作

### GET /api/cell/read/:path/:sheet/:cell
读取单元格值。

**路径参数**：
- `path`：文件路径
- `sheet`：工作表名
- `cell`：单元格引用，如 `A1`

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

### GET /api/range/read/:path/:sheet/:range
读取区域数据。

**路径参数**：
- `path`：文件路径
- `sheet`：工作表名
- `range`：区域引用，如 `A1:C5`

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
  ]
}
```

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
  "query": "SELECT * FROM t WHERE A > 10 ORDER BY B DESC"
}
```

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
  "formula": "=SUM(A1:B1)"
}
```

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
  "range": "A1:C3"
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
  }
}
```

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
  "title": "销量统计"
}
```

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

### GET /api/named_ranges/list/:path
列出所有命名范围。

**路径参数**：
- `path`：文件路径

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
  "sheet": "Sheet1"
}
```

- `sheet` 可选，不传则对比全部工作表

### POST /api/diff/range
对比两个文件指定区域的差异。

**请求体**：
```json
{
  "old_path": "old.xlsx",
  "new_path": "new.xlsx",
  "sheet": "Sheet1",
  "range": "A1:C10"
}
```
