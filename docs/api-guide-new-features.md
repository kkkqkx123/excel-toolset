# 新增功能API使用指南

本文档提供新增功能的详细使用示例。

## 公式分析API

### 1. 追踪单元格依赖关系

**端点**: `POST /api/formula/trace_dependencies`

**请求体**:
```json
{
  "path": "/path/to/workbook.xlsx",
  "sheet": "Sheet1",
  "cell": "C5"
}
```

**响应示例**:
```json
{
  "status": "success",
  "data": {
    "cell": "C5",
    "direct_precedents": [
      "Sheet1!A1",
      "Sheet1!B2",
      "Sheet1!A2"
    ],
    "direct_dependents": [
      "Sheet1!D6",
      "Sheet1!E7"
    ],
    "all_precedents": [
      "Sheet1!A1",
      "Sheet1!A2",
      "Sheet1!B2",
      "Sheet2!C10"
    ],
    "all_dependents": [
      "Sheet1!D6",
      "Sheet1!E7",
      "Sheet1!F8"
    ]
  }
}
```

**说明**:
- `direct_precedents`: 直接引用的单元格
- `direct_dependents`: 直接引用此单元格的单元格
- `all_precedents`: 所有前驱单元格（递归）
- `all_dependents`: 所有后继单元格（递归）

---

### 2. 解释公式

**端点**: `POST /api/formula/explain`

**请求体**:
```json
{
  "path": "/path/to/workbook.xlsx",
  "sheet": "Sheet1",
  "cell": "C5",
  "language": "zh"
}
```

**响应示例**:
```json
{
  "status": "success",
  "data": {
    "cell": "C5",
    "formula": "=SUM(A1:A10)",
    "function_name": "SUM",
    "arguments": ["A1:A10"],
    "description": "SUM 函数: 计算参数的总和。公式: SUM(A1:A10)",
    "language": "zh"
  }
}
```

**说明**:
- 支持语言: `zh` (中文), `en` (英文)
- 返回函数名称、参数和自然语言描述

---

### 3. 深度公式逻辑分析

**端点**: `POST /api/formula/explain_logic`

**请求体**:
```json
{
  "path": "/path/to/workbook.xlsx",
  "sheet": "Sheet1",
  "cell": "C5",
  "language": "zh"
}
```

**响应示例**:
```json
{
  "status": "success",
  "data": {
    "cell": "C5",
    "formula": "=SUM(A1:A10)",
    "logic_flow": [
      {
        "step_number": 1,
        "operation": "读取数据源",
        "input": "Sheet1!A1, Sheet1!A2, Sheet1!A3, ..., Sheet1!A10",
        "result": "从 10 个单元格读取数据"
      },
      {
        "step_number": 2,
        "operation": "计算总和",
        "input": "A1:A10",
        "result": "所有参数的数值之和"
      }
    ],
    "data_sources": [
      "Sheet1!A1",
      "Sheet1!A2",
      "Sheet1!A3",
      "Sheet1!A4",
      "Sheet1!A5",
      "Sheet1!A6",
      "Sheet1!A7",
      "Sheet1!A8",
      "Sheet1!A9",
      "Sheet1!A10"
    ],
    "calculation_result": "1500"
  }
}
```

**说明**:
- 提供详细的计算步骤
- 列出所有数据源
- 显示计算结果

---

## 搜索API

### 1. 搜索工作簿

**端点**: `POST /api/search/workbook`

**请求体**:
```json
{
  "path": "/path/to/workbook.xlsx",
  "pattern": "1000",
  "search_type": "value",
  "match_type": "exact",
  "case_sensitive": false
}
```

**响应示例**:
```json
{
  "status": "success",
  "data": {
    "query": "1000",
    "matches": [
      {
        "sheet": "Sheet1",
        "cell": "B5",
        "value": "1000",
        "formula": null,
        "context": [
          ["", "", ""],
          ["", "1000", ""],
          ["", "", ""]
        ]
      },
      {
        "sheet": "Sheet2",
        "cell": "A3",
        "value": "1000",
        "formula": null,
        "context": [
          ["", "", ""],
          ["", "1000", ""],
          ["", "", ""]
        ]
      }
    ],
    "total_matches": 2
  }
}
```

**参数说明**:
- `search_type`: `value` (搜索值), `formula` (搜索公式), `both` (两者)
- `match_type`: `exact` (精确匹配), `contains` (包含匹配), `regex` (正则表达式)
- `case_sensitive`: `true` (大小写敏感), `false` (不敏感)

---

### 2. 搜索指定工作表

**端点**: `POST /api/search/sheet`

**请求体**:
```json
{
  "path": "/path/to/workbook.xlsx",
  "sheet": "Sheet1",
  "pattern": "SUM\\(",
  "search_type": "formula",
  "match_type": "regex",
  "case_sensitive": false
}
```

**说明**:
- 仅在指定工作表中搜索
- 支持正则表达式模式

---

## 批注管理API

### 1. 读取批注

**端点**: `POST /api/comments/get`

**请求体**:
```json
{
  "path": "/path/to/workbook.xlsx",
  "sheet": "Sheet1",
  "cell": "B5"
}
```

**响应示例**:
```json
{
  "status": "success",
  "data": {
    "author": "张三",
    "text": "这是关键数据，请谨慎修改",
    "created_at": "2024-01-15T10:30:00Z"
  }
}
```

---

### 2. 添加批注

**端点**: `POST /api/comments/add`

**请求体**:
```json
{
  "path": "/path/to/workbook.xlsx",
  "sheet": "Sheet1",
  "cell": "B5",
  "comment": "已验证，数据正确",
  "dry_run": false
}
```

**响应示例**:
```json
{
  "status": "success",
  "data": {
    "path": "/path/to/workbook.xlsx",
    "created_backup": true,
    "changes_made": true,
    "message": "Comment added to B5 in sheet Sheet1"
  }
}
```

---

### 3. 更新批注

**端点**: `POST /api/comments/update`

**请求体**:
```json
{
  "path": "/path/to/workbook.xlsx",
  "sheet": "Sheet1",
  "cell": "B5",
  "comment": "数据已更新为最新版本",
  "dry_run": false
}
```

---

### 4. 删除批注

**端点**: `POST /api/comments/delete`

**请求体**:
```json
{
  "path": "/path/to/workbook.xlsx",
  "sheet": "Sheet1",
  "cell": "B5",
  "dry_run": false
}
```

---

## 命名范围管理API

### 1. 列出所有命名范围

**端点**: `GET /api/named_ranges/list/{path}`

**响应示例**:
```json
{
  "status": "success",
  "data": [
    {
      "name": "SalesData",
      "refers_to": "Sheet1!A1:D100",
      "sheet": "Sheet1",
      "comment": null
    },
    {
      "name": "TaxRate",
      "refers_to": "Sheet2!B2",
      "sheet": "Sheet2",
      "comment": null
    }
  ]
}
```

---

### 2. 获取命名范围的值

**端点**: `POST /api/named_ranges/get_value`

**请求体**:
```json
{
  "path": "/path/to/workbook.xlsx",
  "name": "SalesData"
}
```

**响应示例**:
```json
{
  "status": "success",
  "data": [
    [
      {"row": 0, "col": 0, "value": "Product", "data_type": "String", "formula": null},
      {"row": 0, "col": 1, "value": "Quantity", "data_type": "String", "formula": null}
    ],
    [
      {"row": 1, "col": 0, "value": "A", "data_type": "String", "formula": null},
      {"row": 1, "col": 1, "value": "100", "data_type": "Number", "formula": null}
    ]
  ]
}
```

---

### 3. 创建命名范围

**端点**: `POST /api/named_ranges/create`

**请求体**:
```json
{
  "path": "/path/to/workbook.xlsx",
  "name": "QuarterlyTotal",
  "range": "E1:E4",
  "sheet": "Sheet1",
  "dry_run": false
}
```

**响应示例**:
```json
{
  "status": "success",
  "data": {
    "path": "/path/to/workbook.xlsx",
    "created_backup": true,
    "changes_made": true,
    "message": "Named range 'QuarterlyTotal' created"
  }
}
```

---

### 4. 删除命名范围

**端点**: `POST /api/named_ranges/delete`

**请求体**:
```json
{
  "path": "/path/to/workbook.xlsx",
  "name": "OldRange",
  "dry_run": false
}
```

---

## 条件格式API

### 1. 添加条件格式

**端点**: `POST /api/conditional_format/add`

**请求体**:
```json
{
  "path": "/path/to/workbook.xlsx",
  "sheet": "Sheet1",
  "range": "A1:A100",
  "rule_type": "cellvalue",
  "condition": ">1000",
  "format": {
    "font_color": "FF0000",
    "bold": true,
    "background_color": "FFFF00"
  },
  "dry_run": false
}
```

**参数说明**:
- `rule_type`: `cellvalue`, `formula`, `aboveaverage`, `top10`, `duplicate`, `textcontains`, `dateoccurring`
- `condition`: 条件表达式（如 `>1000`, `=SUM(A1)>100`）
- `format`: 格式样式

---

### 2. 删除条件格式

**端点**: `POST /api/conditional_format/remove`

**请求体**:
```json
{
  "path": "/path/to/workbook.xlsx",
  "sheet": "Sheet1",
  "range": "A1:A100",
  "dry_run": false
}
```

---

## 通用参数

### Dry Run 模式
所有写操作都支持 `dry_run` 参数：
- `true`: 预览操作，不实际修改文件
- `false`: 执行实际操作

### 备份机制
写操作会自动创建备份（如果 `create_backup` 为 true）

---

## 错误处理

所有API遵循统一的错误响应格式：

```json
{
  "status": "error",
  "error": {
    "code": "SHEET_NOT_FOUND",
    "message": "Sheet 'InvalidSheet' not found in workbook"
  }
}
```

---

## 使用建议

1. **公式分析**: 使用 `explain_logic` 获取最详细的公式说明
2. **搜索**: 使用正则表达式进行复杂搜索
3. **批注**: 在关键数据上添加批注以便协作
4. **命名范围**: 为常用范围创建命名范围，提高可读性
5. **条件格式**: 使用条件格式突出显示重要数据
6. **Dry Run**: 在执行危险操作前先用 dry_run 测试

---

## 性能建议

1. **搜索**: 尽量指定工作表，避免全工作簿搜索
2. **公式分析**: 依赖追踪在大文件上可能较慢
3. **批注**: 批量操作时考虑使用批量API（待实现）

---

## 示例场景

### 场景1: 分析复杂公式
```bash
curl -X POST http://localhost:3000/api/formula/explain_logic \
  -H "Content-Type: application/json" \
  -d '{
    "path": "/data/report.xlsx",
    "sheet": "Summary",
    "cell": "D25",
    "language": "zh"
  }'
```

### 场景2: 搜索所有错误公式
```bash
curl -X POST http://localhost:3000/api/search/workbook \
  -H "Content-Type: application/json" \
  -d '{
    "path": "/data/workbook.xlsx",
    "pattern": "^ERROR",
    "search_type": "formula",
    "match_type": "regex",
    "case_sensitive": false
  }'
```

### 场景3: 为关键数据添加批注
```bash
curl -X POST http://localhost:3000/api/comments/add \
  -H "Content-Type: application/json" \
  -d '{
    "path": "/data/budget.xlsx",
    "sheet": "Q4",
    "cell": "B10",
    "comment": "最终审计数字，不可更改",
    "dry_run": false
  }'
```

### 场景4: 为销售额创建命名范围
```bash
curl -X POST http://localhost:3000/api/named_ranges/create \
  -H "Content-Type: application/json" \
  -d '{
    "path": "/data/sales.xlsx",
    "name": "Q4Sales",
    "range": "B2:B1000",
    "sheet": "Sales",
    "dry_run": false
  }'
```

### 场景5: 高亮显示超额数据
```bash
curl -X POST http://localhost:3000/api/conditional_format/add \
  -H "Content-Type: application/json" \
  -d '{
    "path": "/data/budget.xlsx",
    "sheet": "Q4",
    "range": "C2:C100",
    "rule_type": "cellvalue",
    "condition": ">100000",
    "format": {
      "font_color": "FF0000",
      "bold": true
    },
    "dry_run": false
  }'
```