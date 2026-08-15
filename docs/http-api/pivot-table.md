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

