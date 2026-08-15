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

