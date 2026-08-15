## 工作簿概览

### POST /api/workbook/overview
获取工作簿概览（含蓝图模式）。

**请求体**：
```json
{
  "path": "data.xlsx",
  "blueprint": true
}
```

**字段**：
| 字段 | 类型 | 必选 | 默认值 | 说明 |
|------|------|------|--------|------|
| path | string | 是 | - | 文件路径 |
| blueprint | boolean | 否 | false | 输出工作簿蓝图（详细结构信息） |

### POST /api/workbook/history
获取文件操作历史。

**请求体**：
```json
{
  "path": "data.xlsx"
}
```

### POST /api/workbook/sheet_overview
获取单个工作表概览。

**请求体**：
```json
{
  "path": "data.xlsx",
  "sheet": "Sheet1"
}
```

---

