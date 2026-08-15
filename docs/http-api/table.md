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

