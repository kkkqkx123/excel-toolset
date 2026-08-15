## 服务启动

```bash
cargo run --package excel-http --release
```

默认监听 `127.0.0.1:3000`，端口可通过 `PORT` 环境变量覆盖。

## 通用约定

### 请求方法

除 `/health` 使用 GET 外，所有 API 端点统一使用 POST 方法，参数通过 JSON 请求体传递。

### 响应格式

所有接口统一 `ApiResponse<T>` 格式：

```json
{
  "success": true,
  "data": { ... },
  "error": null
}
```

错误响应：
```json
{
  "success": false,
  "data": null,
  "error": {
    "code": "ERROR_CODE",
    "message": "错误描述"
  }
}
```

---

## 健康检查

### GET /health
返回服务状态。

**响应**：
```json
{
  "success": true,
  "data": {
    "status": "ok",
    "timestamp": "2026-06-15T10:00:00Z"
  }
}
```

---


## 资源目录
- [## 文件操作](./file.md)
- [## 工作表操作](./sheet.md)
- [## 冻结窗格](./freeze-panes.md)
- [## 单元格操作](./cell.md)
- [## 区域操作](./range.md)
- [## 批量操作](./batch.md)
- [## 数据处理](./data.md)
- [## 公式操作](./formula.md)
- [## 搜索](./search.md)
- [## 格式操作](./format.md)
- [## 图表](./chart.md)
- [## 批注](./comments.md)
- [## 命名范围](./named-ranges.md)
- [## 条件格式](./conditional-format.md)
- [## VBA](./vba.md)
- [## Diff 对比](./diff.md)
- [## 表格](./table.md)
- [## 数据验证](./data-validation.md)
- [## 数据透视表](./pivot-table.md)
- [## 切片器](./slicer.md)
- [## 迷你图](./sparkline.md)
- [## 工作簿概览](./workbook-overview.md)
- [## 自动筛选](./auto-filter.md)
- [## 工作表保护](./protection.md)
- [## 页面设置](./page-setup.md)
- [## 图片/形状](./image.md)
