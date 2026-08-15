## 冻结窗格

### POST /api/freeze-panes/set
设置冻结窗格。

**请求体**：
```json
{
  "path": "data.xlsx",
  "sheet": "Sheet1",
  "rows": 1,
  "cols": 0,
  "dry_run": false
}
```

**字段**：
| 字段 | 类型 | 必选 | 默认值 | 说明 |
|------|------|------|--------|------|
| path | string | 是 | - | 文件路径 |
| sheet | string | 是 | - | 工作表名 |
| rows | u32 | 否 | 0 | 从顶部冻结的行数 |
| cols | u16 | 否 | 0 | 从左侧冻结的列数 |
| dry_run | boolean | 否 | false | 模拟执行不写入 |

### POST /api/freeze-panes/clear
清除冻结窗格。

**请求体**：
```json
{
  "path": "data.xlsx",
  "sheet": "Sheet1"
}
```

---

