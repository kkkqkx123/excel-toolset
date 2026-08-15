## 格式操作

### POST /api/format/set
设置单元格格式。

**请求体**：
```json
{
  "path": "data.xlsx",
  "sheet": "Sheet1",
  "range": "A1:A10",
  "style": {
    "bold": true,
    "italic": false,
    "font_size": 14,
    "font_color": "#FF0000",
    "bg_color": "#FFFF00",
    "border": {
      "color": "#000000",
      "style": "thin"
    },
    "alignment": {
      "horizontal": "center",
      "vertical": "center"
    }
  }
}
```

### POST /api/cell/merge
合并单元格。

**请求体**：
```json
{
  "path": "data.xlsx",
  "sheet": "Sheet1",
  "range": "A1:C3",
  "value": "合并后的值"
}
```

**字段**：
| 字段 | 类型 | 必选 | 说明 |
|------|------|------|------|
| path | string | 是 | 文件路径 |
| sheet | string | 是 | 工作表名 |
| range | string | 是 | 合并区域 |
| value | string | 否 | 合并后单元格的值 |

---

