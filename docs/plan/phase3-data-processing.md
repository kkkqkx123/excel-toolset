# 阶段3：数据处理、公式、样式、VBA、Diff 能力

**目标**：实现数据处理、VBA、Diff 等高级能力模块。
**产出**：完整覆盖架构设计中 9 类原子操作的全部能力。

---

## 3.1 数据加工模块（excel_data.rs）

依赖 `excel_read` 读取数据，内存加工，按需调用 `excel_write` 落地。

| 原子操作 | 实现方式 | 是否写回文件 |
|----------|---------|:---:|
| 追加行 | 读原数据 → 末尾追加 → 写回 | ✅ |
| 插入行 | 读原数据 → 指定位置插入 → 写回 | ✅ |
| 删除行 | 读原数据 → 跳过目标行 → 写回 | ✅ |
| 筛选 | 读原数据 → 内存条件过滤 → 返回结果 | ❌ 纯查询 |
| 排序 | 读原数据 → 内存排序 → 写回 | ✅ |
| 去重 | 读原数据 → HashSet 去重 → 写回 | ✅ |
| SQL 查询 | 读原数据 → DuckDB/polars 内存查询 → 返回结果 | ❌ 纯查询 |

**实现要点**：

- **追加/插入/删除行**：复用 `excel_write` 的「读→改→写」模式
- **筛选**：支持多条件组合（`col > 100 AND col < 200`），返回筛选后的 JSON 数据
- **排序**：支持多列排序、升降序、空值处理
- **去重**：支持指定列去重，默认全列匹配
- **SQL 查询**：集成 `polars` 或 `sqlparser` 在内存中执行 SQL，结果可写回新 sheet

### 3.1.1 SQL 查询方案选型

| 方案 | 优点 | 缺点 |
|------|------|------|
| `polars` + `sqlparser` | 纯 Rust、性能好 | 仅支持 SQL 子集 |
| `duckdb` (rust binding) | SQL 支持完整、成熟 | 额外二进制依赖 |
| 自建简易过滤 DSL | 零依赖、可控 | 功能有限 |

**推荐**：优先 `polars` 做核心引擎，配合 `sqlparser` 解析 SQL WHERE/ORDER BY。

## 3.2 公式刷新（excel_write.rs 扩展）

| 操作 | 说明 |
|------|------|
| `refresh_formulas(path, sheet)` | 重写公式单元格，设置 `calc_mode` 标记 |
| `set_calculation_mode(path, mode)` | 设置工作簿计算模式（auto/manual） |

**注意**：`rust_xlsxwriter` 不执行公式计算，仅写入公式文本。计算值需在 Excel/WPS 中打开时自动刷新。如需预计算，使用外部计算引擎。

## 3.3 VBA 模块（vba_util.rs）

基于 `calamine` + `rust_xlsxwriter` 实现 VBA 二进制流透传。

| 操作 | 实现方式 |
|------|---------|
| 导出 VBA | `calamine` 读取 `vbaProject.bin` → 输出二进制/文件 |
| 导入 VBA | `rust_xlsxwriter` `add_vba_project()` → 嵌入 `.xlsm` |
| 检查 VBA 存在 | 检测 `.xlsm` 文件内 `vbaProject.bin` 是否存在 |

**约束**：
- 仅透传二进制，不解析、不执行 VBA 代码
- 导入时确保文件为 `.xlsm` 格式
- VBA 修改需在外部分析工具中完成

## 3.4 Diff 模块（excel_diff.rs）

实现文件/工作表/单元格多粒度对比，支撑「独立对比」和「写操作附属对比」双形态。

### 3.4.1 独立 Diff API

| 函数 | 说明 |
|------|------|
| `diff_files(old_path, new_path) -> Result<FileDiff>` | 全文件对比 |
| `diff_sheets(old_path, new_path, sheet) -> Result<SheetDiff>` | 指定 sheet 对比 |
| `diff_range(old_path, new_path, sheet, range) -> Result<RangeDiff>` | 指定区域对比 |

### 3.4.2 附属 Diff API

供写入模块内部调用：
```rust
fn compute_diff(old_data: &SheetData, new_data: &SheetData) -> Vec<CellDiff>
```

### 3.4.3 Diff 数据结构

```rust
pub struct FileDiff {
    pub file_hash_match: bool,       // 快速判同
    pub sheet_diffs: Vec<SheetDiff>,
    pub summary: DiffSummary,        // 变更统计
}

pub struct SheetDiff {
    pub sheet_name: String,
    pub row_count_diff: i32,
    pub col_count_diff: i32,
    pub cell_diffs: Vec<CellDiff>,
}

pub struct CellDiff {
    pub row: u32,
    pub col: u16,
    pub diff_type: DiffType,         // Add/Delete/Modify/NoChange
    pub old_value: Option<String>,
    pub new_value: Option<String>,
    pub old_formula: Option<String>,
    pub new_formula: Option<String>,
}

pub struct DiffSummary {
    pub adds: usize,
    pub deletes: usize,
    pub modifies: usize,
    pub total_changes: usize,
}
```

### 3.4.4 性能策略

1. 优先比对文件指纹（SHA-256），一致则跳过全量解析
2. `diff_range` 仅读取和目标区域，减少 IO
3. 大文件常量内存模式读取（`calamine` 支持增量读取）

## 3.5 图表与数据透视表

| 操作 | 依赖 | 说明 |
|------|------|------|
| 生成图表 | `rust_xlsxwriter::Chart` | 柱状图/折线图/饼图 |
| 数据透视表 | ⚠️ 有限支持 | rust_xlsxwriter 暂无原生支持，使用 SUMIF/COUNTIF 变通 |

**图表支持类型**：
- `Column`（柱状图）、`Line`（折线图）、`Pie`（饼图）
- `Bar`（条形图）、`Area`（面积图）、`Scatter`（散点图）
- 支持设置标题、坐标轴、图例、数据标签

## 3.6 验证标准

- [ ] 追加/插入/删除行数据正确、行列号正确
- [ ] 筛选返回正确过滤结果
- [ ] 排序支持多列、升降序
- [ ] 去重正确保留唯一行
- [ ] SQL 查询返回结构化结果
- [ ] VBA 导入导出的二进制内容与原始文件一致
- [ ] Diff 模块：文件级快速判同、单元格级变更标记正确
- [ ] 图表生成正确，可写入 .xlsx 并被 Excel 打开
- [ ] 所有写操作自动携带附属 diff
