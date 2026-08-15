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

