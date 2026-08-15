## 区域操作

### POST /api/range/read
读取区域数据。

**请求体**：
```json
{
  "path": "data.xlsx",
  "sheet": "Sheet1",
  "range": "A1:C5",
  "mode": "detailed",
  "truncate": 100
}
```

**字段**：
| 字段 | 类型 | 必选 | 默认值 | 说明 |
|------|------|------|--------|------|
| path | string | 是 | - | 文件路径 |
| sheet | string | 是 | - | 工作表名 |
| range | string | 是 | - | 区域引用，如 `A1:C5` |
| mode | string | 否 | `detailed` | 输出模式：`detailed`（含行号列号）、`compact`（纯数据矩阵）、`csv` |
| truncate | u32 | 否 | - | 限制返回行数 |

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

