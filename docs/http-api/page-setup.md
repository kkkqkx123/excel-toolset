## 页面设置

### POST /api/page-setup/configure
配置页面设置（纸张大小、方向、边距等）。

**请求体**：
```json
{
  "path": "data.xlsx",
  "sheet": "Sheet1",
  "config": {
    "orientation": "landscape",
    "paper_size": 9,
    "margins": {
      "top": 0.75,
      "bottom": 0.75,
      "left": 0.7,
      "right": 0.7
    }
  }
}
```

### POST /api/page-setup/page-breaks/set
设置分页符。

**请求体**：
```json
{
  "path": "data.xlsx",
  "config": {
    "sheet": "Sheet1",
    "row_breaks": [10, 25],
    "column_breaks": [5]
  }
}
```

### POST /api/page-setup/page-breaks/clear
清除分页符。

**请求体**：
```json
{
  "path": "data.xlsx",
  "sheet": "Sheet1"
}
```

---

