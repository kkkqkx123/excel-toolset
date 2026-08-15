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

