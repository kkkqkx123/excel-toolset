## 工作表保护

### POST /api/protection/sheet/protect
保护工作表。

**请求体**：
```json
{
  "path": "data.xlsx",
  "sheet": "Sheet1",
  "password": "secret",
  "options": {
    "select_locked_cells": false,
    "select_unlocked_cells": true,
    "format_cells": false
  },
  "dry_run": false
}
```

**字段**：
| 字段 | 类型 | 必选 | 说明 |
|------|------|------|------|
| path | string | 是 | 文件路径 |
| sheet | string | 是 | 工作表名 |
| password | string | 否 | 保护密码 |
| options | object | 否 | ProtectionOptions 配置 |
| dry_run | boolean | 否 | 模拟执行不写入 |

### POST /api/protection/sheet/unprotect
解除工作表保护。

**请求体**：
```json
{
  "path": "data.xlsx",
  "sheet": "Sheet1"
}
```

### POST /api/protection/sheet/is-protected
检查工作表保护状态。

**请求体**：
```json
{
  "path": "data.xlsx",
  "sheet": "Sheet1"
}
```

---

