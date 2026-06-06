# Excel HTTP 服务

基于 Axum 框架的 Excel 操作 HTTP API 服务。

## 目录结构

```
src/
├── main.rs              # 服务入口
└── http/                # HTTP 模块
    ├── mod.rs           # 模块声明
    ├── router.rs        # 路由配置
    ├── handlers/        # 基础处理器
    ├── data_operations/ # 数据操作
    ├── formula/         # 公式相关
    ├── formatting/      # 格式相关
    ├── advanced/        # 高级特性
    └── middleware/      # 中间件
```

## 快速开始

### 启动服务

```bash
cargo run --release
```

服务将在 `http://0.0.0.0:3000` 启动。

### 健康检查

```bash
curl http://localhost:3000/health
```

## API 端点

### 基础操作

#### 文件操作
- `GET /api/file/info/:path` - 获取文件信息
- `POST /api/file/create` - 创建新文件
- `POST /api/file/backup` - 备份文件

#### 工作表操作
- `GET /api/sheet/list/:path` - 列出工作表
- `POST /api/sheet/add` - 添加工作表
- `POST /api/sheet/delete` - 删除工作表
- `POST /api/sheet/rename` - 重命名工作表

#### 单元格操作
- `GET /api/cell/read/:path/:sheet/:cell` - 读取单元格
- `POST /api/cell/write` - 写入单元格

#### 区域操作
- `GET /api/range/read/:path/:sheet/:range` - 读取区域
- `POST /api/range/write` - 写入区域
- `POST /api/range/write-from-csv` - 从 CSV 写入区域
- `POST /api/range/clear` - 清除区域

#### 批量操作
- `POST /api/batch/modify` - 批量修改

### 数据操作

#### 行操作
- `POST /api/data/append-row` - 追加行
- `POST /api/data/insert-row` - 插入行
- `POST /api/data/delete-row` - 删除行

#### 数据处理
- `POST /api/data/filter` - 过滤数据
- `POST /api/data/sort` - 排序数据
- `POST /api/data/dedup` - 去重数据
- `POST /api/data/sql` - SQL 查询

### 公式操作

#### 基础公式
- `POST /api/formula/set` - 设置公式
- `POST /api/formula/refresh` - 刷新公式

#### 公式分析
- `POST /api/formula/trace_dependencies` - 追踪依赖
- `POST /api/formula/explain` - 解释公式
- `POST /api/formula/explain_logic` - 解释公式逻辑

### 格式操作

#### 单元格格式
- `POST /api/format/set` - 设置格式
- `POST /api/cell/merge` - 合并单元格

#### 条件格式
- `POST /api/conditional_format/add` - 添加条件格式
- `POST /api/conditional_format/remove` - 移除条件格式

### 高级特性

#### 图表
- `POST /api/chart/create` - 创建图表

#### 评论
- `POST /api/comments/get` - 获取评论
- `POST /api/comments/add` - 添加评论
- `POST /api/comments/update` - 更新评论
- `POST /api/comments/delete` - 删除评论

#### 命名区域
- `GET /api/named_ranges/list/:path` - 列出命名区域
- `POST /api/named_ranges/get_value` - 获取命名区域值
- `POST /api/named_ranges/create` - 创建命名区域
- `POST /api/named_ranges/delete` - 删除命名区域

#### VBA 操作
- `POST /api/vba/export` - 导出 VBA
- `POST /api/vba/import` - 导入 VBA

#### 搜索
- `POST /api/search/workbook` - 搜索工作簿
- `POST /api/search/sheet` - 搜索工作表

### 差异对比

- `POST /api/diff/file` - 文件对比
- `POST /api/diff/range` - 区域对比

## 使用示例

### 读取单元格

```bash
curl "http://localhost:3000/api/cell/read/test.xlsx/Sheet1/A1"
```

### 写入单元格

```bash
curl -X POST http://localhost:3000/api/cell/write \
  -H "Content-Type: application/json" \
  -d '{
    "path": "test.xlsx",
    "sheet": "Sheet1",
    "cell": "A1",
    "value": "Hello World"
  }'
```

### 写入区域

```bash
curl -X POST http://localhost:3000/api/range/write \
  -H "Content-Type: application/json" \
  -d '{
    "path": "test.xlsx",
    "sheet": "Sheet1",
    "range": "A1:C3",
    "data": [
      ["A1", "B1", "C1"],
      ["A2", "B2", "C2"],
      ["A3", "B3", "C3"]
    ]
  }'
```

### 追加行

```bash
curl -X POST http://localhost:3000/api/data/append-row \
  -H "Content-Type: application/json" \
  -d '{
    "path": "test.xlsx",
    "sheet": "Sheet1",
    "values": ["Value1", "Value2", "Value3"]
  }'
```

### SQL 查询

```bash
curl -X POST http://localhost:3000/api/data/sql \
  -H "Content-Type: application/json" \
  -d '{
    "path": "test.xlsx",
    "sheet": "Sheet1",
    "query": "SELECT * FROM Sheet1 WHERE A > 10"
  }'
```

### 搜索

```bash
curl -X POST http://localhost:3000/api/search/workbook \
  -H "Content-Type: application/json" \
  -d '{
    "path": "test.xlsx",
    "pattern": "keyword",
    "search_type": "both",
    "match_type": "contains",
    "case_sensitive": false
  }'
```

## 响应格式

### 成功响应

```json
{
  "success": true,
  "data": {
    // 响应数据
  }
}
```

### 错误响应

```json
{
  "success": false,
  "error": {
    "code": "ERROR_CODE",
    "message": "错误描述"
  }
}
```

## Dry Run 模式

所有写操作都支持 `dry_run` 参数，用于预览操作而不实际修改文件：

```json
{
  "path": "test.xlsx",
  "sheet": "Sheet1",
  "value": "test",
  "dry_run": true
}
```

## 安全性

所有写操作都会：
1. 自动创建备份
2. 计算文件哈希
3. 支持回滚

## 性能优化

- 使用 Tokio 异步运行时
- 高效的内存管理
- 批量操作优化

## 开发

### 添加新的 API 端点

1. 在相应的子目录创建处理函数
2. 在 `router.rs` 中添加路由
3. 在对应的 `mod.rs` 中导出模块

### 添加中间件

在 `middleware/` 目录创建新的中间件模块，然后在 `router.rs` 中应用。

## 构建

```bash
# 开发构建
cargo build

# 发布构建
cargo build --release
```

## 测试

```bash
# 运行所有测试
cargo test

# 运行特定测试
cargo test test_name
```

## 文档

详细的架构和重构文档请参考：
- [HTTP 包重构文档](../../docs/architecture/http-refactoring.md)
- [项目架构文档](../../docs/architecture/)