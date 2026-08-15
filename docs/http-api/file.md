## 文件操作

### POST /api/file/info
获取 Excel 文件信息。

**请求体**：
```json
{
  "path": "data.xlsx"
}
```

**响应**：
```json
{
  "success": true,
  "data": {
    "sheet_count": 3,
    "sheets": ["Sheet1", "Sheet2", "Sheet3"],
    "file_hash": "abcd1234..."
  }
}
```

### POST /api/file/create
创建新的 Excel 文件。

**请求体**：
```json
{
  "path": "output.xlsx",
  "sheet": "Sheet1"
}
```

**字段**：
| 字段 | 类型 | 必选 | 默认值 | 说明 |
|------|------|------|--------|------|
| path | string | 是 | - | 文件路径 |
| sheet | string | 否 | `Sheet1` | 初始工作表名 |

### POST /api/file/backup
创建文件备份。

**请求体**：
```json
{
  "path": "data.xlsx",
  "output": "/tmp/backup.xlsx"
}
```

**字段**：
| 字段 | 类型 | 必选 | 说明 |
|------|------|------|------|
| path | string | 是 | 文件路径 |
| output | string | 否 | 额外复制备份到指定位置 |

### POST /api/file/rollback
从备份回滚文件。

**请求体**：
```json
{
  "path": "data.xlsx",
  "backup_path": ".backups/backup_20260615.xlsx"
}
```

---

