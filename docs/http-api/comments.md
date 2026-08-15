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

