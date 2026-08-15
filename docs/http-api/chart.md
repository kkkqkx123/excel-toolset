## 图表

### POST /api/chart/create
创建图表。

**请求体**：
```json
{
  "path": "data.xlsx",
  "sheet": "Sheet1",
  "range": "A1:B10",
  "chart_type": "column",
  "title": "销量统计",
  "position": "E5",
  "trendline": {
    "trend_type": "linear",
    "display_equation": true
  },
  "y_error_bars": {
    "error_type": "standard_error",
    "direction": "both"
  },
  "x_error_bars": {
    "error_type": "fixed_value",
    "value": 1.0
  },
  "log_base": 10
}
```

**字段**：
| 字段 | 类型 | 必选 | 说明 |
|------|------|------|------|
| path | string | 是 | 文件路径 |
| sheet | string | 是 | 工作表名 |
| range | string | 是 | 数据区域 |
| chart_type | string | 是 | 图表类型 |
| title | string | 否 | 图表标题 |
| position | string | 否 | 放置位置（单元格引用） |
| trendline | object | 否 | 趋势线配置 |
| y_error_bars | object | 否 | Y 轴误差线配置 |
| x_error_bars | object | 否 | X 轴误差线配置 |
| log_base | u16 | 否 | 对数刻度底数 |
| dry_run | boolean | 否 | 模拟执行不写入 |

**支持的 chart_type**：
| 值 | 说明 |
|----|------|
| `column` | 柱状图 |
| `bar` | 条形图 |
| `line` | 折线图 |
| `pie` | 饼图 |
| `area` | 面积图 |
| `scatter` | 散点图 |

---

