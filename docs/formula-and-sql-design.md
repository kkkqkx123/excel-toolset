# 公式重算与 SQL 查询集成设计方案

## 一、`refresh_formulas` 重算标记方案

### 当前问题

`rust_xlsxwriter` 和 `calamine` 均不包含公式计算引擎，无法在无头模式下实时计算 Excel 公式值。任何尝试实现自研公式计算引擎的方案都存在以下问题：

- 需要实现完整的 Excel 函数表（400+ 函数）
- 需要处理循环引用、跨工作表引用、命名范围等复杂场景
- 实现成本高，且与 Excel 原生计算结果可能存在细微差异

### 核心原则

> **公式计算是 Excel 应用程序的责任，不是无头库的责任。**

### 实现方案：设置重算标记 + 清除缓存值

文件被 Excel/WPS 打开时，如果检测到 `<calcPr fullCalcOnLoad="1">` 标记，会自动重新计算所有公式并更新缓存值。

#### 具体实现

修改 `excel_write::refresh_formulas`，做两件事：

1. **清除公式单元格的缓存值 `<v>`**：强制 Excel 重新计算
2. **设置 Workbook 级别的 `fullCalcOnLoad` 属性**：确保打开时触发重算

```rust
pub fn refresh_formulas(
    path: &str,
    params: &SecurityParams,
    sheet: &str,
) -> Result<WriteResult> {
    modify_file(path, params, |old_data, wb| {
        let mut new_data = old_data.clone();

        // 1. 清除公式单元格的缓存值
        for (name, data) in new_data.iter_mut() {
            if name == sheet || sheet == "*" {
                for row in data.rows.iter_mut() {
                    for cell in row.iter_mut() {
                        if cell.formula.is_some() {
                            cell.value = None;
                        }
                    }
                }
            }
        }

        // 2. 重新写入所有数据
        *wb = Workbook::new();
        for (name, data) in new_data.iter() {
            let ws = wb.add_worksheet();
            ws.set_name(name).map_err(AppError::Xlsx)?;
            write_sheet_data(ws, data)?;
        }

        // 3. rust_xlsxwriter 在含有公式的 Workbook 中
        //    默认设置 fullCalcOnLoad="1"，无需额外操作
        Ok(new_data)
    })
}
```

### 效果

| 场景                                    | 行为                                                |
| --------------------------------------- | --------------------------------------------------- |
| Agent 设置公式后立即 `refresh_formulas` | 清除缓存 -> 用户打开 Excel 自动重算                 |
| Agent 连续写多个公式                    | 最后一次调用 `refresh_formulas` 即可                |
| 用户打开文件                            | Excel 检测到 `fullCalcOnLoad` + 空缓存，全量重算    |
| Diff 传递                               | 公式表达式不变，缓存值变化 -> Diff 仅体现缓存值变更 |

### 移除实时计算方案的原因

| 方案                  | 问题                                          |
| --------------------- | --------------------------------------------- |
| 集成公式计算引擎      | 需要实现 400+ 函数，结果不一致，维护成本高    |
| 集成 LibreOffice 内核 | 依赖重、跨平台问题、启动慢                    |
| 调用 Excel COM        | 需要安装 Office，且仅限 Windows               |
| **重算标记（选中）**  | 零依赖、零计算、逻辑简单，利用 Excel 原生能力 |

---

## 二、DuckDB SQL 查询集成方案

### 2.1 为什么选择 DuckDB

| 对比项          | DuckDB            | Polars        | DataFusion    | 纯 Rust 实现 |
| --------------- | ----------------- | ------------- | ------------- | ------------ |
| SQL 完整性      | 完整 SQL:2023     | 无原生 SQL    | 完整 SQL      | 仅基础过滤   |
| Excel 直接读取  | ✅ `excel_scan()` | ❌ 需额外转换 | ❌ 需额外转换 | N/A          |
| 嵌入式运行      | ✅                | ❌            | ✅            | ✅           |
| AI Agent 友好度 | ⭐⭐⭐⭐⭐        | ⭐⭐⭐        | ⭐⭐⭐⭐      | ⭐⭐         |

DuckDB 官方提供专用 **Excel 扩展**（非 spatial/st_read）：

```sql
INSTALL excel;
LOAD excel;
CREATE VIEW data AS SELECT * FROM excel_scan('file.xlsx');
```

优势：

- 官方维护，专属 xlsx 读取，无需空间扩展依赖
- 读取更快、更轻量
- 字段名/类型推导更精确，与 calamine 读取结果一致

### 2.2 统一查询引擎：DuckDB 覆盖全部数据处理

当前 `excel_data` 中的 filter/sort/dedup 是基于 calamine 读取全量数据后在 Rust 内存中实现的。启用 `sql` feature 后：

| 功能          | Rust 实现（默认）          | DuckDB 实现（启用 sql）                            |
| ------------- | -------------------------- | -------------------------------------------------- |
| `filter_rows` | Rust 内存遍历 + 字符串比较 | `SELECT * FROM df WHERE ...`                       |
| `sort_sheet`  | Rust Vec sort_by           | `SELECT * FROM df ORDER BY ...`                    |
| `dedup_sheet` | Rust HashSet 去重          | `SELECT DISTINCT ON (...) * FROM df`               |
| `sql_query`   | ❌ 不支持                  | `SELECT ... FROM df [WHERE/GROUP BY/ORDER BY ...]` |
| 聚合查询      | ❌ 不支持                  | `SELECT col, SUM(col2) ... GROUP BY col`           |
| 多表 JOIN     | ❌ 不支持                  | 跨 Sheet JOIN 查询                                 |

### 2.3 写入类操作强制复用 excel-core 修改流程

`excel-sql` 中的 **写入类操作**（排序 `sort_sheet`、去重 `dedup_sheet`）**必须**调用 `excel-core` 的 `modify_file` 流程，而非直接操作文件：

```
用户请求 -> excel-core::modify_file（统一入口）
  ├── 自动：计算原文件指纹
  ├── 自动：创建备份快照
  ├── 自动：校验 dry_run 标记
  ├── 调用 excel-core::excel_read 读取原数据
  ├── 调用 excel-sql 执行排序/去重逻辑（内存中）
  ├── 调用 rust_xlsxwriter 写入新文件
  ├── 自动：计算新文件指纹
  ├── 自动：执行 diff 比对（通过 excel-diff）
  ├── 自动：backup_info / diff 写入结果
  └── 返回统一 WriteResult
```

这样写入类操作自动继承完整安全体系：备份、dry-run、指纹校验、Diff 回传。

### 2.4 类型复用：所有 DTO 来自 excel-core

`excel-sql` **不定义**任何业务类型。所有数据结构直接导入 `excel-core`：

```rust
// excel-core 已定义的类型，excel-sql 直接复用：
use excel_core::types::{
    CellValue,  // 写操作的入参值
    CellData,   // 读操作的出参值
    FilterCondition,
    SortColumn,
    SecurityParams,
    WriteResult,
    AppError,
    Result,
    CellDataType,
    SheetData,
};

// excel-sql 仅定义内部使用的 DuckDB 映射逻辑
// 不对外暴露任何新类型
```

`QueryResult` 对应 `excel-core` 中已有的 `Vec<Vec<CellData>>`，无需新增类型：

```rust
// sql_query / filter_rows 的返回类型
pub fn sql_query(path: &str, sql: &str) -> Result<Vec<Vec<CellData>>>;
pub fn filter_rows(path: &str, sheet: &str, conditions: &[FilterCondition]) -> Result<Vec<Vec<CellData>>>;
```

### 2.5 错误处理：统一复用 excel-core 的 AppError

`excel-sql` 封装 DuckDB 错误，统一转换为 `excel-core::AppError`：

```rust
use excel_core::AppError;

// DuckDB 错误 -> AppError 转换
impl From<duckdb::Error> for AppError {
    fn from(e: duckdb::Error) -> Self {
        AppError::Custom(format!("DuckDB error: {}", e))
    }
}

// 所有函数返回 excel_core::Result<T>
pub fn sql_query(path: &str, sql: &str) -> excel_core::Result<Vec<Vec<CellData>>> {
    // ...
    Ok(result)
}
```

---

## 三、独立 `excel-sql` Crate

### crate 定位

DuckDB 依赖较重（`duckdb-rs` 绑定 C++ SDK，构建耗时显著增加），不适合放入 `excel-core`。独立 crate 确保：

1. `excel-core` 保持轻量，无 DuckDB 依赖
2. 依赖关系明确、可追踪
3. Feature gate 控制编译，无需运行时分支

### 目录结构

```
crates/
├── excel-core/          # 核心读写（无 DuckDB 依赖，始终编译）
├── excel-sql/           # DuckDB SQL 查询 + 数据操作（可选编译）
├── excel-diff/          # Diff 引擎
├── excel-cli/           # CLI 入口
└── excel-http/          # HTTP 入口
```

### crate 依赖关系

```
excel-cli ──┬── excel-core (always)
            ├── excel-diff (always)
            └── excel-sql (optional, feature "sql")

excel-http ─┬── excel-core (always)
            ├── excel-diff (always)
            └── excel-sql (optional, feature "sql")

excel-sql ──┬── excel-core (always，仅复用类型+安全+读能力)
            └── duckdb (bundled C++ SDK)
```

### Cargo.toml

```toml
# crates/excel-sql/Cargo.toml
[package]
name = "excel-sql"
version.workspace = true
edition.workspace = true

[dependencies]
excel-core = { path = "../excel-core" }
duckdb = { version = "1.2", features = ["bundled"] }
```

### Feature gate 机制（编译时，无运行时分支）

```toml
# 顶层 workspace Cargo.toml
[workspace]
members = [
    "crates/excel-core",
    "crates/excel-sql",
    "crates/excel-diff",
    "crates/excel-cli",
    "crates/excel-http",
]

[workspace.features]
sql = ["excel-sql"]
default = ["sql"]
```

```toml
# crates/excel-cli/Cargo.toml
[dependencies]
excel-core = { path = "../excel-core" }
excel-sql = { path = "../excel-sql", optional = true }
excel-diff = { path = "../excel-diff" }

[features]
sql = ["excel-sql"]
```

入口层使用 `#[cfg]` 编译时条件分支：

```rust
// 编译时条件编译，无运行时判断

#[cfg(feature = "sql")]
fn handle_filter(path, sheet, conditions) -> excel_core::Result<Vec<Vec<CellData>>> {
    // DuckDB 实现
    excel_sql::filter_rows(path, sheet, conditions)
}

#[cfg(not(feature = "sql"))]
fn handle_filter(path, sheet, conditions) -> excel_core::Result<Vec<Vec<CellData>>> {
    // Rust 纯内存回退实现
    excel_data_internal::filter_rows(path, sheet, conditions)
}
```

### excel-sql 对外 API

```rust
// ====== 查询类（只读，不修改文件） ======

/// SQL 查询：完全自由的 SQL 语句
/// sheet 可自动推断，SQL 中通过 'sheet_name' 引用工作表
pub fn sql_query(path: &str, sql: &str) -> excel_core::Result<Vec<Vec<CellData>>>;

/// 条件过滤：语义化封装，底层转 SQL
pub fn filter_rows(
    path: &str,
    sheet: &str,
    conditions: &[FilterCondition],
) -> excel_core::Result<Vec<Vec<CellData>>>;

// ====== 写入类（必须通过 excel-core 的 modify_file） ======

/// 排序数据
/// 内部调用 excel-core::modify_file 统一流程
pub fn sort_sheet(
    path: &str,
    params: &SecurityParams,
    sheet: &str,
    sort_columns: &[SortColumn],
) -> excel_core::Result<WriteResult>;

/// 去重数据
/// 内部调用 excel-core::modify_file 统一流程
pub fn dedup_sheet(
    path: &str,
    params: &SecurityParams,
    sheet: &str,
    columns: &[u16],
) -> excel_core::Result<WriteResult>;
```

### DuckDB Excel 扩展集成

```rust
fn execute_sql(path: &str, sql: &str) -> excel_core::Result<Vec<Vec<CellData>>> {
    let db = duckdb::Connection::open_in_memory()
        .map_err(|e| AppError::Custom(format!("DuckDB init error: {}", e)))?;

    // 1. 加载并安装 Excel 扩展（DuckDB 官方 Excel 扩展）
    db.execute_batch("INSTALL excel; LOAD excel;")
        .map_err(|e| AppError::Custom(format!("DuckDB excel extension load error: {}", e)))?;

    // 2. 通过 excel_scan 注册文件
    db.execute_batch(&format!(
        "CREATE VIEW xlsx_data AS SELECT * FROM excel_scan('{}');",
        path.replace('\'', "''")
    )).map_err(|e| AppError::Custom(format!("DuckDB excel_scan error: {}", e)))?;

    // 3. 执行用户 SQL
    let mut stmt = db.prepare(sql)
        .map_err(|e| AppError::Custom(format!("DuckDB prepare error: {}", e)))?;

    // 4. 映射结果到 Vec<Vec<CellData>>
    let rows = stmt.query_map([], |row| {
        // DuckDB 列 -> CellData 映射
    })?;

    Ok(result)
}
```

---

## 四、`excel-core` 现有代码调整计划

### 完全不变的部分

| 模块                                        | 理由                                |
| ------------------------------------------- | ----------------------------------- |
| `excel_read`                                | calamine 读取，不依赖 DuckDB        |
| `excel_write`                               | rust_xlsxwriter 写入，不依赖 DuckDB |
| `excel_data` 中的 append/insert/delete rows | 纯行操作，无需 SQL 引擎             |
| `security` / `file_util`                    | 基础组件，不依赖 DuckDB             |
| `cell_ref` / `vba_util`                     | 专项功能，不依赖 DuckDB             |

### 调整的部分

| 文件            | 调整内容                                                                                                         |
| --------------- | ---------------------------------------------------------------------------------------------------------------- |
| `excel_data.rs` | 移除 `sql_query` 桩函数，移入 `excel-sql`；保留 filter/sort/dedup 的 Rust 实现作为 `not(feature = "sql")` 的回退 |
| `lib.rs`        | 对外 re-export 保持不变，所有类型不变                                                                            |

---

## 五、总结

| 问题               | 方案                                           | 理由                                      |
| ------------------ | ---------------------------------------------- | ----------------------------------------- |
| `refresh_formulas` | 设置 fullCalcOnLoad 标记 + 清除缓存值          | 零依赖、利用 Excel 原生计算能力           |
| `sql_query`        | DuckDB 官方 Excel 扩展，新建 `excel-sql` crate | 完整 SQL；独立 crate 避免 excel-core 膨胀 |
| filter/sort/dedup  | 编译时 feature 切换 DuckDB/Rust 实现           | 统一查询引擎；条件编译无运行时开销        |
| 写入类操作         | 强制复用 `excel-core::modify_file`             | 统一安全体系：备份/dry-run/指纹/Diff      |
| 类型/错误          | 完全复用 `excel-core` 定义                     | 全局统一，避免冗余                        |
| Feature 控制       | 纯编译时 `#[cfg]`                              | Rust 惯用风格，零运行时开销               |
