# 阶段1：项目骨架与基础模块

**目标**：搭建 Cargo Workspace 脚手架，完成全局类型定义、基础文件工具、安全组件。
**产出**：可编译的空壳 workspace，验证 Cargo 依赖、模块结构、类型系统。

---

## 1.1 初始化 Workspace

```bash
# Create workspace root
cargo init excel-tool-gateway --workspace

# Create individual crate directories
mkdir -p crates/excel-core/src
mkdir -p crates/excel-diff/src
mkdir -p crates/excel-cli/src
mkdir -p crates/excel-http/src
```

**Workspace Cargo.toml**：
```toml
[workspace]
resolver = "2"
members = [
    "crates/excel-core",
    "crates/excel-diff",
    "crates/excel-cli",
    "crates/excel-http",
]

[workspace.package]
version = "0.1.0"
edition = "2021"
license = "MIT"
```

**excel-core Cargo.toml**：
```toml
[package]
name = "excel-core"
version.workspace = true
edition.workspace = true

[dependencies]
calamine = "0.31"
rust_xlsxwriter = "0.50"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
sha2 = "0.10"
chrono = { version = "0.4", features = ["serde"] }
```

**excel-diff Cargo.toml**：
```toml
[package]
name = "excel-diff"
version.workspace = true
edition.workspace = true

[dependencies]
excel-core = { path = "../excel-core" }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

**excel-cli Cargo.toml**：
```toml
[package]
name = "excel-cli"
version.workspace = true
edition.workspace = true

[dependencies]
excel-core = { path = "../excel-core" }
excel-diff = { path = "../excel-diff" }
clap = { version = "4", features = ["derive"] }
serde_json = "1"
```

**excel-http Cargo.toml**：
```toml
[package]
name = "excel-http"
version.workspace = true
edition.workspace = true

[dependencies]
excel-core = { path = "../excel-core" }
excel-diff = { path = "../excel-diff" }
axum = "0.7"
tokio = { version = "1.0", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

## 1.2 创建目录结构

```
crates/
├── excel-core/
│   └── src/
│       ├── lib.rs              # 公开 API
│       ├── types.rs            # 全局数据结构中心
│       ├── file_util.rs        # 基础文件工具
│       ├── security.rs         # 安全组件（指纹/备份/dry-run/回滚）
│       ├── excel_read.rs       # Excel 读取
│       ├── excel_write.rs      # Excel 写入
│       ├── excel_data.rs       # 数据加工
│       └── vba_util.rs         # VBA 读写
├── excel-diff/
│   └── src/
│       └── lib.rs              # diff 引擎
├── excel-cli/
│   └── src/
│       └── main.rs             # CLI 入口
└── excel-http/
    └── src/
        └── main.rs             # HTTP 入口
```

**阶段1只实现**：`excel-core` 的 `lib.rs`, `types.rs`, `file_util.rs`, `security.rs`，其余空壳存根。

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

## 1.7 验证标准

- [ ] `cargo build` 通过
- [ ] `cargo test --workspace` 通过
- [ ] `types.rs` 核心结构体定义完整
- [ ] `file_util.rs` 文件操作函数单元测试通过
- [ ] `security.rs` 指纹计算、备份、回滚集成测试通过
