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

