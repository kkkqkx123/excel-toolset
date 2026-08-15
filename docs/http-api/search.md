## 搜索

### POST /api/search/workbook
全工作簿搜索。

**请求体**：
```json
{
  "path": "data.xlsx",
  "query": {
    "pattern": "keyword",
    "match_type": "contains",
    "search_type": "both",
    "case_sensitive": false,
    "sheets": ["Sheet1", "Sheet2"]
  }
}
```

**字段说明**：
| 字段 | 可选值 | 说明 |
|------|--------|------|
| match_type | `contains`, `exact`, `regex` | 匹配方式 |
| search_type | `both`, `value`, `formula` | 搜索内容类型 |
| case_sensitive | `true`/`false` | 是否区分大小写 |
| sheets | 数组或 null | 限定搜索的工作表，null 表示全部 |

### POST /api/search/sheet
单工作表搜索。

**请求体**：
```json
{
  "path": "data.xlsx",
  "sheet": "Sheet1",
  "query": {
    "pattern": "keyword",
    "match_type": "contains",
    "search_type": "both",
    "case_sensitive": false
  }
}
```

---

