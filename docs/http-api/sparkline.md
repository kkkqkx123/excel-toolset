## 迷你图

### POST /api/sparkline/add
添加迷你图。

**请求体**：
```json
{
  "path": "data.xlsx",
  "sheet": "Sheet1",
  "source_range": "'Sheet1'!A1:E1",
  "sparkline_type": "line",
  "target_cell": "F1",
  "style": 1,
  "dry_run": false
}
```

**字段**：
| 字段 | 类型 | 必选 | 默认值 | 说明 |
|------|------|------|--------|------|
| path | string | 是 | - | 文件路径 |
| sheet | string | 是 | - | 工作表名 |
| source_range | string | 是 | - | 数据源区域 |
| sparkline_type | string | 否 | `line` | `line`、`column`、`winlose` |
| target_cell | string | 是 | - | 目标单元格 |
| style | u8 | 否 | - | 样式编号 (0-35) |
| dry_run | boolean | 否 | false | 模拟执行不写入 |

### POST /api/sparkline/remove
移除迷你图。

**请求体**：
```json
{
  "path": "data.xlsx",
  "sheet": "Sheet1",
  "target_cell": "F1"
}
```

---

