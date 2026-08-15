## Diff 对比

### POST /api/diff/file
对比两个 Excel 文件的差异。

**请求体**：
```json
{
  "old_path": "old.xlsx",
  "new_path": "new.xlsx",
  "sheet": "Sheet1",
  "semantic": false
}
```

**字段**：
| 字段 | 类型 | 必选 | 说明 |
|------|------|------|------|
| old_path | string | 是 | 旧文件路径 |
| new_path | string | 是 | 新文件路径 |
| sheet | string | 否 | 限定工作表，不传则对比全部 |
| semantic | boolean | 否 | 生成语义级差异报告 |

### POST /api/diff/range
对比两个文件指定区域的差异。

**请求体**：
```json
{
  "old_path": "old.xlsx",
  "new_path": "new.xlsx",
  "sheet": "Sheet1",
  "range": "A1:C10",
  "semantic": false
}
```

### POST /api/diff/semantic
生成语义级差异报告。

**请求体**：
```json
{
  "old_path": "old.xlsx",
  "new_path": "new.xlsx"
}
```
返回结构化的语义差异摘要，包含 Summary 和 Detail 两种详细度。

### POST /api/diff/formula_dependencies
对比两个文件的公式依赖图差异。

**请求体**：
```json
{
  "old_path": "old.xlsx",
  "new_path": "new.xlsx",
  "sheet": "Sheet1"
}
```
返回依赖图变更详情，包含循环检测结果。

---

