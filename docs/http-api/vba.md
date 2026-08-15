## VBA

### POST /api/vba/export
导出 VBA 宏代码。

**请求体**：
```json
{
  "path": "data.xlsm",
  "output": "vba_output.bas"
}
```

### POST /api/vba/import
导入 VBA 宏代码。

**请求体**：
```json
{
  "path": "data.xlsm",
  "vba_path": "macro.bas"
}
```

### POST /api/vba/has
检查文件是否包含 VBA 宏。

**请求体**：
```json
{
  "path": "data.xlsm"
}
```

**响应**：
```json
{
  "success": true,
  "data": {
    "has_vba": true
  }
}
```

---

