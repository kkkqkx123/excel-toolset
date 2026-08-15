## 条件格式

### POST /api/conditional_format/add
添加条件格式规则。

**请求体**：
```json
{
  "path": "data.xlsx",
  "sheet": "Sheet1",
  "range": "A1:A10",
  "rule_type": "cell_value",
  "condition": ">100",
  "style": {
    "font_color": "#FF0000",
    "bold": true
  },
  "config": {
    "fill_color": "#00FF00"
  }
}
```

**字段**：
| 字段 | 类型 | 必选 | 说明 |
|------|------|------|------|
| path | string | 是 | 文件路径 |
| sheet | string | 是 | 工作表名 |
| range | string | 是 | 应用区域 |
| rule_type | string | 是 | 规则类型 |
| condition | string | 是 | 条件表达式 |
| style | object | 否 | 格式样式 |
| config | object | 否 | 高级配置（DataBar/ColorScale/IconSet） |
| dry_run | boolean | 否 | 模拟执行不写入 |

**支持的 rule_type**：
| 值 | 说明 |
|----|------|
| `cell_value` | 单元格值条件 |
| `formula` | 公式条件 |
| `above_average` | 高于平均值 |
| `top10` | 前 N 项 |
| `duplicate` | 重复值高亮 |
| `text_contains` | 文本包含 |
| `date_occurring` | 日期条件 |

### POST /api/conditional_format/remove
移除条件格式规则。

**请求体**：
```json
{
  "path": "data.xlsx",
  "sheet": "Sheet1",
  "range": "A1:A10"
}
```

---

