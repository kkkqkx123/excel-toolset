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

