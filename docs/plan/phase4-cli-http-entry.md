# 阶段4：CLI + HTTP 双入口封装

**目标**：为所有原子操作添加 CLI 子命令和 HTTP 路由，实现完整可用工具。
**产出**：`excel-tool-gateway` CLI 二进制 + HTTP 服务可启动运行。

---

## 4.1 CLI 模块（cli/commands.rs + cli/mod.rs）

基于 `clap` 子命令模式，一个原子操作 == 一条子命令。

### 4.1.1 命令树结构

```bash
excel
├── file
│   ├── create <path> [--sheet]
│   ├── info <path>
│   ├── save <path>
│   └── backup <path> [--output]
├── sheet
│   ├── list <path>
│   ├── add <path> <name>
│   ├── delete <path> <name>
│   └── rename <path> <old> <new>
├── cell
│   ├── read <path> <sheet> <cell>
│   └── write <path> <sheet> <cell> <value> [--dry-run]
├── range
│   ├── read <path> <sheet> <range>
│   ├── write <path> <sheet> <range> <data> [--dry-run]
│   └── clear <path> <sheet> <range> [--dry-run]
├── data
│   ├── append-row <path> <sheet> <values...> [--dry-run]
│   ├── insert-row <path> <sheet> <row> <values...> [--dry-run]
│   ├── delete-row <path> <sheet> <row> [--dry-run]
│   ├── filter <path> <sheet> <column> <condition>
│   ├── sort <path> <sheet> <column> [--desc] [--dry-run]
│   ├── deduplicate <path> <sheet> [--column] [--dry-run]
│   └── sql <path> <query>
├── formula
│   ├── set <path> <sheet> <cell> <formula> [--dry-run]
│   └── refresh <path> <sheet> [--dry-run]
├── format
│   ├── set <path> <sheet> <range> <style-json> [--dry-run]
│   └── merge <path> <sheet> <range> [--dry-run]
├── chart
│   └── create <path> <sheet> <range> <type> [--dry-run]
├── vba
│   ├── export <path> <output>
│   └── import <path> <vba-file> [--dry-run]
├── diff
│   ├── file <old-path> <new-path> [--sheet] [--range]
│   └── range <old-path> <new-path> <sheet> <range>
└── rollback <path> <backup-timestamp>
```

### 4.1.2 全局参数

```
--format <json|text>    输出格式（默认 json）
--pretty                格式化 JSON 输出
--verbose               详细日志
```

### 4.1.3 CLI 输出规则

- 默认输出纯 JSON，适配 Agent 程序解析
- `--pretty` 格式化 JSON（人工阅读）
- 错误统一输出 `{success: false, message: "错误描述"}`

## 4.2 HTTP 模块（http/router.rs + http/handlers.rs）

基于 `axum` 实现 RESTful API。

### 4.2.1 路由设计

```
GET    /api/file/info/{path}
POST   /api/file/create
POST   /api/file/save/{path}
POST   /api/file/backup/{path}

GET    /api/sheet/list/{path}
POST   /api/sheet/add
POST   /api/sheet/delete
POST   /api/sheet/rename

GET    /api/cell/read/{path}/{sheet}/{cell}
POST   /api/cell/write
GET    /api/range/read/{path}/{sheet}/{range}
POST   /api/range/write
POST   /api/range/clear

POST   /api/data/append-row
POST   /api/data/insert-row
POST   /api/data/delete-row
GET    /api/data/filter
POST   /api/data/sort
POST   /api/data/deduplicate
POST   /api/advanced/sql

POST   /api/formula/set
POST   /api/formula/refresh

POST   /api/format/set
POST   /api/cell/merge

POST   /api/advanced/chart

POST   /api/vba/export
POST   /api/vba/import

POST   /api/diff/file
POST   /api/diff/range
POST   /api/file/rollback
```

### 4.2.2 HTTP 请求/响应规范

**请求**：`Content-Type: application/json`
**响应**：统一 `ApiResponse<T>` JSON 结构

```json
{
  "success": true,
  "message": "操作成功",
  "file_hash": "sha256hex...",
  "data": { /* 业务数据 */ },
  "diff": null,
  "backup_info": null
}
```

### 4.2.3 HTTP 服务启动

```rust
// main.rs
#[cfg(feature = "http")]
async fn run_http_server() {
    let app = Router::new()
        .route("/api/cell/read", get(cell_read_handler))
        .route("/api/cell/write", post(cell_write_handler));
        // ... 所有路由

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
```

## 4.3 CLI/HTTP 入口切换（main.rs）

```rust
fn main() {
    // 检测命令行参数，自动切换模式
    // cli 模式：解析 clap 命令 → 执行
    // http 模式：--serve 或环境变量 EXCEL_HTTP=true
}
```

**切分策略**：
- 默认模式：CLI（`cargo run -- cell read test.xlsx Sheet1 A1`）
- HTTP 模式：`cargo run -- --serve` 或 `EXCEL_MODE=http cargo run`

## 4.4 全局错误处理

统一错误类型：
```rust
pub enum AppError {
    FileNotFound(String),
    SheetNotFound(String),
    InvalidCellRef(String),
    IOError(std::io::Error),
    XlsxError(rust_xlsxwriter::XlsxError),
    CalamineError(calamine::Error),
}
```

实现 `IntoResponse` for HTTP，`Display` for CLI 输出。

## 4.5 验证标准

- [ ] 所有 CLI 子命令可正确执行并输出 JSON
- [ ] 所有 HTTP 路由响应正确状态码和 JSON
- [ ] `--dry-run` 参数在所有写命令中生效
- [ ] CLI 默认 JSON 输出，`--pretty` 格式化
- [ ] 错误情况返回统一 `{success: false}` 结构
- [ ] HTTP 服务启动，健康检查 `/health` 返回 OK
- [ ] 跨平台：Windows/macOS/Linux 均可编译运行
