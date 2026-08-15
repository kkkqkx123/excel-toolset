## 工作表操作

### POST /api/sheet/list
列出所有工作表。

**请求体**：
```json
{
  "path": "data.xlsx"
}
```

### POST /api/sheet/add
添加新工作表。

**请求体**：
```json
{
  "path": "data.xlsx",
  "name": "NewSheet"
}
```

### POST /api/sheet/delete
删除工作表。

**请求体**：
```json
{
  "path": "data.xlsx",
  "name": "Sheet2"
}
```

### POST /api/sheet/rename
重命名工作表。

**请求体**：
```json
{
  "path": "data.xlsx",
  "old_name": "OldSheet",
  "new_name": "NewSheet"
}
```

### POST /api/sheet/visibility
设置工作表可见性。

**请求体**：
```json
{
  "path": "data.xlsx",
  "name": "Sheet1",
  "visibility": "hidden"
}
```

**字段**：
| 字段 | 类型 | 必选 | 说明 |
|------|------|------|------|
| path | string | 是 | 文件路径 |
| name | string | 是 | 工作表名 |
| visibility | string | 是 | `visible`（可见）、`hidden`（隐藏）、`very_hidden`（深度隐藏） |
| dry_run | boolean | 否 | 模拟执行不写入 |

---

