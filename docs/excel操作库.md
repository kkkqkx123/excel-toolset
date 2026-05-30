先直接回答你的两个核心问题：

1. **Rust 做 Excel 工具（HTTP 服务/CLI）首选：calamine + rust_xlsxwriter，或基于它们封装的 rust-excel / excel-mcp-server**。  
2. **只需要“读/改 VBA 代码、不程序内执行” → 完全不需要 xlwings**；用纯文件级库就能处理 .xlsm，VBA 以二进制流读写即可。

下面分三部分讲清楚：选型、推荐组合、VBA 处理方案。

---

## 一、Rust Excel 库大盘点（适合做 HTTP/CLI）

### 1）calamine（必选，读/解析最强）
- 只读为主、极速解析 .xlsx/.xlsm/.xls
- 比 openpyxl **快 9–10 倍**，内存极低
- 支持：单元格、公式、样式、**VBA 二进制流**（可以读/写宏）
- 无头、跨平台、无依赖
- 适合：HTTP 服务端批量读、AI Agent 取数

### 2）rust_xlsxwriter（写最强，纯 Rust）
- 只写，生成 .xlsx/.xlsm
- 完整样式、公式、图表、**可嵌入 VBA 宏**
- 底层类似 libxlsxwriter，Rust 安全+高性能
- 适合：AI Agent 生成报表、写入结果

### 3）rust-excel（封装 calamine + rust_xlsxwriter）
- crates：rust-excel-core / rust-excel-api
- 一站式：读+写+样式+公式+VBA
- 官方带 Axum HTTP 服务端（40+ 接口）、CLI 示例
- 适合：直接拿来做 AI Agent 的 Excel HTTP 微服务

### 4）excel-mcp-server（现成 HTTP+MCP，AI 友好）
- 基于 zavora-xlsx（Rust）+ rmcp
- 43 个工具：单元格读写、筛选、排序、图表、数据透视表、**VBA 导入导出**
- 支持 stdio/HTTP，直接对接 AI Agent（MCP 协议）
- 适合：零开发快速部署

### 5）xls-rs（多格式+CLI+MCP）
- 支持 .xlsx/.xls/.ods/csv/parquet
- 内置 CLI 和 MCP Server，公式计算、pandas 风格操作
- 适合：多格式混杂场景

### 6）不推荐的
- **win32com-rs / xlwings-rs 绑定**：依赖 Windows+Excel，不适合服务端/跨平台。

---

## 二、最终推荐（HTTP 服务端 / CLI）

### ✅ 方案 A：自建轻量服务（最可控）
**核心依赖**
```toml
calamine = "0.31"
rust_xlsxwriter = "0.50"
axum = "0.7"
tokio = { version = "1.0", features = ["full"] }
```
- **读**：calamine → 单元格、公式、样式、**VBA 二进制**
- **写**：rust_xlsxwriter → 单元格、样式、公式、**写入 VBA 流**
- **HTTP**：Axum 暴露 RESTful 接口（/read、/write、/list_sheets、/vba_export、/vba_import）
- **CLI**：用 clap 做子命令，和 HTTP 复用同一套逻辑

### ✅ 方案 B：直接用现成（最快上线）
```bash
# 安装现成 MCP/HTTP 服务
cargo install excel-mcp-server
# 启动 HTTP（AI Agent 直接调用）
BIND_ADDRESS=0.0.0.0:3000 excel-mcp-server http
```
- 开箱即用：读写、样式、图表、**VBA 导入/导出**
- 支持 MCP 协议，LangChain/AutoGen 直接集成

### ✅ 性能与部署
- 单进程 100MB 内存内处理 100万行无压力
- 可打包成静态二进制，无依赖，Linux/Windows/macOS 都能跑
- 适合作为 AI Agent 的“Excel 微服务”，本地或容器部署

---

## 三、VBA 只读写、不执行 → 不需要 xlwings

### 1）xlwings 的唯一不可替代点
- **在程序里执行 VBA 宏、调用 Excel COM、操作 Excel 窗口**。

### 2）你的场景：仅编辑/读取 VBA，手动执行验证
- .xlsm 文件中，VBA 是以 **二进制流（OLE 结构）** 存储的，不是文本
- **calamine 可以读出这个二进制流**，你可以：
  - 保存为 .bas 或 .vba 文本（自己解析/格式化）
  - 修改后，**rust_xlsxwriter 可以把新的 VBA 流写回 .xlsm**
- 全程**不需要安装 Excel、不需要 COM、不需要 xlwings**

### 3）结论
- **只读写 VBA 代码 → 纯 Rust 库足够，xlwings 完全没必要**
- 只有当你需要**程序内自动运行宏、调试 VBA、操作 Excel 界面**时，才需要 xlwings（且只能在 Windows 上用）。

---

## 四、给你最简落地建议
1. **Rust 库**：calamine + rust_xlsxwriter（或直接用 excel-mcp-server）
2. **部署形态**：HTTP 服务（3000端口）+ CLI 二进制
3. **VBA 处理**：用 calamine 读二进制 → 转文本编辑 → rust_xlsxwriter 写回
4. **不要用 xlwings**：除非你要自动执行宏
