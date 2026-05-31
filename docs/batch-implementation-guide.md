# 批量操作扩展实现指南

基于 `extend-plan.md` 的决策，结合代码库实际架构的落地实现方案。

---

## 一、当前架构摘要

### 两条独立修改管线

| 管线 | 位置 | 闭包签名 | 涵盖操作 |
|------|------|---------|---------|
| **A** | `excel_write::modify_file` | `(&HashMap<String,SheetData>, &mut Workbook)` | write_cell, write_range, clear_range, set_formula, add/delete/rename_sheet, set_format, merge_cells, add_chart |
| **B** | `excel_data::modify_data_file` | `(&HashMap<String,SheetData>)` | append/insert/delete_rows, sort, dedup |

每条管线独立执行 **备份→hash→读取→修改→重写Workbook→保存→hash**，N次操作 = N倍全流程。

### 关键约束

- `excel-core` 不能依赖 `excel-diff`（否则循环依赖：diff→core→diff）
- `summarize::summarize` 是 `pub(crate)`，对外不可见
- `CellValue` 缺少 `Serialize/Deserialize`，无法 JSON 传输
- 无 CSV 依赖

---

## 二、架构决策

### 决策 1：Batch 执行器放在 `excel_write.rs`

Batch 执行器作为 `excel_write` 模块的新增函数，复用内部所有私有 helper（`ensure_dimensions`、`cell_value_to_data`、`write_sheet_data` 等）。

### 决策 2：数据操作与 Workbook 操作分离

Batch 操作分为两类：

| 类别 | 操作 | 处理方式 |
|------|------|---------|
| **纯数据操作** | WriteCell, WriteRange, WriteRangeFromCsv, ClearRange, SetFormula, InsertRows, DeleteRows, AppendRows, AddSheet, DeleteSheet, RenameSheet, SortSheet, DedupSheet | 直接修改 `HashMap<String, SheetData>` |
| **Workbook 操作** | SetFormat, MergeCells, AddChart | 延后到 Workbook 构建完成后，作为第二遍 pass |

执行流程：

```
[Pass 1] 遍历所有操作
  ├─ 数据操作 → 修改 data map（纯内存）
  └─ Workbook 操作 → 跳过（延后）

[构建] data map → Workbook（一次）

[Pass 2] 遍历所有操作
  └─ Workbook 操作 → 在已构建的 Workbook 上应用

[保存] wb.save(path) — 一次写入
[Diff] 由调用方通过 diff_files(backup, path) 计算
```

### 决策 3：Diff 由调用方计算

`execute_batch_operations` 只负责执行，不计算 diff。调用方（CLI/HTTP handler）在拿到结果后，通过 `excel_diff::diff_files(backup_path, file_path)` 计算最终 Diff。

原因：避免 `excel-core` → `excel-diff` 循环依赖。

### 决策 4：CellValue 启用 Serde

`CellValue` 当前只 derive `Debug, Clone`，需要补 `Serialize, Deserialize` 以支持 BatchOperation 的 JSON 序列化（HTTP 请求/响应）。

---

## 三、实现步骤

### Step 1: `types.rs` — 新增类型定义

**`BatchOperation` 枚举**（`#[serde(tag = "type")]`）：

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum BatchOperation {
    WriteCell { sheet: String, row: u32, col: u16, value: CellValue },
    WriteRange { sheet: String, range: String, data: Vec<Vec<CellValue>> },
    WriteRangeFromCsv { sheet: String, range: String, csv_path: String },
    ClearRange { sheet: String, range: String },
    SetFormula { sheet: String, cell: String, formula: String },
    InsertRows { sheet: String, at_row: u32, data: Vec<Vec<CellValue>> },
    DeleteRows { sheet: String, start_row: u32, end_row: u32 },
    AppendRows { sheet: String, data: Vec<Vec<CellValue>> },
    AddSheet { name: String },
    DeleteSheet { name: String },
    RenameSheet { old_name: String, new_name: String },
    SortSheet { sheet: String, columns: Vec<SortColumn> },
    DedupSheet { sheet: String, columns: Vec<u16> },
    MergeCells { sheet: String, range: String, value: Option<String> },
    SetFormat { sheet: String, range: String, style: Style },
    AddChart { config: ChartConfig },
}
```

**`BatchWriteResult`** 返回结构：

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchWriteResult {
    pub success: bool,
    pub message: String,
    pub backup_info: Option<BackupInfo>,
    pub old_hash: String,
    pub new_hash: String,
    pub diff: Option<FileDiff>,
    pub operations_count: usize,
    pub succeeded_count: usize,
}
```

**`CellValue`** 补 Serde：

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CellValue { ... }
```

### Step 2: `excel-core/Cargo.toml` — 新增依赖

```toml
csv = "1.4"
```

### Step 3: `excel_write.rs` — 新增函数

三个新函数：

1. **`read_csv_to_cell_values(csv_path)`** — 读取 CSV 文件，解析为 `Vec<Vec<CellValue>>`
2. **`write_range_from_csv(path, params, sheet, range, csv_path)`** — 独立 CSV 写入（保留原子接口）
3. **`execute_batch_operations(path, params, operations)`** — 核心批量执行器

内部辅助函数（纯数据变换，操作 `&mut HashMap<String, SheetData>`）：

```rust
fn apply_write_cell(data, sheet, row, col, value) -> Result<()>
fn apply_write_range(data, sheet, range, grid) -> Result<()>
fn apply_clear_range(data, sheet, range) -> Result<()>
fn apply_set_formula(data, sheet, cell, formula) -> Result<()>
fn apply_insert_rows(data, sheet, at_row, grid) -> Result<()>
fn apply_delete_rows(data, sheet, start, end) -> Result<()>
fn apply_append_rows(data, sheet, grid) -> Result<()>
fn apply_add_sheet(data, name) -> Result<()>
fn apply_delete_sheet(data, name) -> Result<()>
fn apply_rename_sheet(data, old, new) -> Result<()>
fn apply_sort_sheet(data, sheet, columns) -> Result<()>
fn apply_dedup_sheet(data, sheet, columns) -> Result<()>
```

内部辅助函数（Workbook 操作）：

```rust
fn apply_set_format(wb, data, sheet, range, style) -> Result<()>
fn apply_merge_cells(wb, data, sheet, range, value) -> Result<()>
fn apply_add_chart(wb, data, config) -> Result<()>
```

### Step 4: CLI 集成 — `commands.rs`

新增子命令：

```
excel batch modify <path> --operations ops.json [--dry-run]
excel write-csv <path> <sheet> <range> <csv> [--dry-run]
```

### Step 5: HTTP 集成 — `router.rs` + `handlers.rs`

新增路由：

```
POST /api/batch/modify
POST /api/range/write-from-csv
```

---

## 四、原子性保证

Batch 执行器的原子性模型：

```
原始文件 ──→ 备份（安全网）
               │
          内存中 clone data map
               │
          顺序应用操作到 clone（纯内存）
               │
          ┌────┴────┐
      成功 └────────┘ 失败 ──→ 丢弃 clone，原始文件不变
          │
      构建 Workbook → 保存 → 计算 hash → 返回
```

- 中途任何操作失败 → 直接返回错误，不写盘
- 备份只做一次，已存在的备份不做清理（保留安全网）
- 无中间状态文件

---

## 五、与现有系统的兼容性

| 现有模块 | 影响 |
|---------|------|
| 所有原子写函数（`write_cell` 等） | 不变，继续可用 |
| `excel_data.rs` 的行操作 | 不变，batch 执行器内联了相同逻辑 |
| `security.rs` 备份/指纹 | 复用，batch 执行器调用一次 |
| `cell_ref.rs` 引用解析 | 复用 |
| `excel-diff` 差异引擎 | 由调用方在 batch 执行后调用 `diff_files` |
| `excel-sql` | 不受影响 |
| VBA 操作 | 不受影响（不在 batch 操作范围内） |

---

## 六、未来可扩展方向

1. **Batch 内 Dry-Run Diff**：当前 dry-run 无备份文件，无法做 diff。后续可通过保存临时文件再删除的方式支持。
2. **Validate 前置校验**：在执行前全量校验所有 sheet 存在性和操作合法性，提前报错。
3. **进度报告**：为 HTTP 长连接提供 SSE 进度事件。
4. **Format 数据化**：将 Style 存入 CellData，统一在 Workbook 构建时渲染（消除双 pass）。
