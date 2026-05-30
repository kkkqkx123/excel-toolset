# 阶段1：项目骨架与基础模块

**目标**：搭建 Rust 项目脚手架，完成全局类型定义、基础文件工具、安全组件。
**产出**：可编译的空壳项目，验证 Cargo 依赖、模块结构、类型系统。

---

## 1.1 初始化项目

```bash
cargo init excel-tool-gateway
```

**Cargo.toml 依赖**：
```toml
[dependencies]
# 核心 Excel 库
calamine = "0.31"
rust_xlsxwriter = "0.50"

# HTTP 服务（可选，可延迟引入）
axum = { version = "0.7", optional = true }
tokio = { version = "1.0", features = ["full"], optional = true }

# CLI 解析
clap = { version = "4", features = ["derive"] }

# 序列化
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# 安全
sha2 = "0.10"
chrono = "0.4"

[features]
default = ["cli"]
cli = ["clap"]
http = ["axum", "tokio"]
```

## 1.2 创建目录结构

```
src/
├── main.rs              # 入口
├── types.rs             # 全局数据结构中心
├── file_util.rs         # 基础文件工具
├── security.rs          # 安全组件（指纹/备份/dry-run/回滚）
├── excel_read.rs        # Excel 读取
├── excel_write.rs       # Excel 写入
├── excel_data.rs        # 数据加工
├── vba_util.rs          # VBA 读写
├── excel_diff.rs        # Diff 对比
├── cli/
│   ├── mod.rs
│   └── commands.rs
└── http/
    ├── mod.rs
    ├── router.rs
    └── handlers.rs
```

**阶段1只实现**：`main.rs`, `types.rs`, `file_util.rs`, `security.rs`, 空壳模块存根。

## 1.3 全局类型定义（types.rs）

核心数据结构，后续所有模块共用：

| 类别 | 结构体/枚举 | 说明 |
|------|------------|------|
| 通用响应 | `ApiResponse<T>` | `{success, message, file_hash, data, diff, backup_info}` |
| 文件信息 | `FileInfo` | `{path, hash, size, sheets, created_at}` |
| 备份信息 | `BackupInfo` | `{backup_path, timestamp, operation, file_hash}` |
| 安全入参 | `SecurityParams` | `{dry_run, create_backup, file_path}` |
| 单元格引用 | `CellRef` | `{sheet, row, col}` / `{sheet, range}` |
| 差异结构 | `CellDiff`, `RowDiff`, `SheetDiff`, `FileDiff` | 分级 diff 输出 |
| 变更类型 | `DiffType` | `Add, Delete, Modify, NoChange` |
| 操作模式 | `OperationMode` | `Live, DryRun` |

## 1.4 基础文件工具（file_util.rs）

| 函数 | 功能 |
|------|------|
| `path_exists(p: &str) -> bool` | 路径存在性检查 |
| `copy_file(src, dst) -> Result<()>` | 文件复制 |
| `create_temp_dir() -> Result<PathBuf>` | 创建临时目录 |
| `append_timestamp(path) -> String` | 追加时间戳生成备份路径 |
| `ensure_parent_dir(path) -> Result<()>` | 确保父目录存在 |
| `file_size(path) -> Result<u64>` | 获取文件大小 |

## 1.5 安全组件（security.rs）

| 函数 | 功能 |
|------|------|
| `compute_file_hash(path) -> Result<String>` | SHA-256 文件指纹 |
| `create_backup(path) -> Result<BackupInfo>` | 自动备份，返回快照信息 |
| `rollback(backup_info) -> Result<()>` | 基于快照回滚文件 |
| `with_security(params, f)` | 安全包装器：指纹→备份→执行→校验 |

**写操作安全链路**（所有写模块统一调用）：
```
compute_file_hash → create_backup → (dry_run 判断) → execute → verify_hash
```

## 1.6 空壳模块存根

创建 `excel_read.rs`, `excel_write.rs`, `excel_data.rs`, `vba_util.rs`, `excel_diff.rs` 的空壳模块，仅含模块声明和占位函数签名，确保 `cargo build` 通过。

## 1.7 main.rs 入口

最小入口：打印欢迎信息，预留 CLI/HTTP 特征门控。

```rust
fn main() {
    println!("Excel Tool Gateway v0.1.0");
    // 后续通过 cargo features 切换 CLI/HTTP
}
```

## 1.8 验证标准

- [ ] `cargo build` 通过
- [ ] `cargo test` 通过
- [ ] `types.rs` 核心结构体定义完整
- [ ] `file_util.rs` 文件操作函数单元测试通过
- [ ] `security.rs` 指纹计算、备份、回滚集成测试通过
