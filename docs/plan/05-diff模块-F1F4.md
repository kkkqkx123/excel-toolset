# diff 模块修复方案：F1 / F2 / F3 / F4

**模块**：`excel-diff`（crate）。整体质量最好，但 F1/F2 是两个严重缺陷，F4 使 git driver 完全失效。

---

## F1 — 非 A1 起点数据 → 坐标全部报错（严重）

### 现象
纯数值、数据在 **C3:E3**（把 E3 由 14 改成 99），diff 报：`Cell C1 in "S": changed from 14 to 99` —— 实际变的是 **E3**，不是 C1（T7.25）。同一 bug 还会把别的单元格值合并进公式单元格：A1 有公式、C1=100 时，diff 报 `A1: formula="1+1", value="100"`（T7.26）。

### 根因（两个叠加）
1. **坐标偏移未加**：`read_sheet_all` 用 `range.rows().enumerate()` 的**相对索引**当绝对坐标，`format_cell_ref` 从不加 calamine `range.start()` 偏移（helpers.rs:82 + engine.rs:33）。数据在 C3 → 实际偏移 `(start_row=2, start_col=2)` 被丢弃，E3 被报成 C1。
2. **公式网格与值网格原点不一致**：`worksheet_formula()` 返回的 `Range` 与 `worksheet_range()` 的原点可能不同。当前用**值网格的相对索引**去查公式网格：
   ```rust
   // excel_read.rs:202
   f.get_value((row_idx as u32, col_idx as u32))   // ← 应为绝对坐标
   ```
   当两者原点不同，公式被错挂到相邻单元格 → "A1 的值变成 100"。

### 修改方案

**(a) 公式按绝对坐标查找**（修 F1-2，同时大概率修 F3）
```rust
// excel_read.rs read_sheet_all
let (start_row, start_col) = range.start();   // calamine 返回 (usize,usize) 绝对原点
...
let formula = ws_formulas.as_ref().and_then(|f| {
    // 用绝对坐标查公式网格，避免与值网格原点不一致
    f.get_value(((start_row + row_idx) as u32, (start_col + col_idx) as u32))
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
});
```

**(b) 在 SheetData 携带原点偏移**，使 diff 输出坐标正确（修 F1-1）
- 在 `excel-types/src/cell.rs` 的 `SheetData` 增加字段：
  ```rust
  pub struct SheetData {
      pub name: String,
      pub rows: Vec<Vec<CellData>>,
      pub start_row: u32,   // ← 新增：range.start() 行偏移
      pub start_col: u32,   // ← 新增：range.start() 列偏移
  }
  ```
- `read_sheet_all`（excel_read.rs:211）构造时填入 `start_row: start_row as u32, start_col: start_col as u32`（其余构造点默认 `0`，属机械更新，见下）。
- `compute_cell_diffs`（engine.rs:33）与 `all_cells_as_diff`（helpers.rs:36）在调用 `format_cell_ref` 时加偏移：
  ```rust
  // engine.rs
  let abs_row = (row_idx + old_data.start_row as usize);
  let abs_col = (col_idx + old_data.start_col as usize);
  let cell_ref = format_cell_ref(abs_row, abs_col);
  // CellDiff.row/col 也设为绝对，保持一致
  ```
  > `format_cell_ref(row, col)` 本身保持 `(row,col)->A1`（0-based→1-based），调用方负责传入绝对索引。

**机械更新范围**：`SheetData { name, rows }` 字面量约 30 处（engine.rs / helpers.rs / formula_tracker.rs 的测试、operations.rs、core.rs、examples）。统一补 `, start_row: 0, start_col: 0`（真实读取路径填真实值）。

### 验证
- 构造 C3:E3 数据，E3 14→99，diff 应报 `Cell E3 ... changed from 14 to 99`（T7.25 复测）。
- A1 公式 + C1=100 表，diff 的 A1 不应带 value="100"（T7.26 复测）。

> **待解决问题（关联写路径）**：当前 `write_sheet_data` 按 `data.rows` 的**相对索引**写，非 A1 起点数据在"读后写"时会被重新定位到 A1 起（与 F1 同源）。F1(b) 的偏移若仅用于 diff 不影响写；但若要彻底一致，写路径也应在 `(start_row+ri, start_col+ci)` 落位（或在 read 时按需 pad 至绝对坐标）。该项测试套件未覆盖非 A1 写，标记为后续跟进。

---

## F2 — `formula-deps` 依赖图恒为空

### 现象
`formula-deps` 永远返回空：`cycles_introduced/modified_dependencies/...` 全空。D1 改 B1→C1、循环引用 A1↔B1 均无输出（T7.20/22）。反证：同文件 `formula trace` 能正确列出 `["S!A1","S!B1"]`（T7.21）—— 数据没问题，是解析器坏了。

### 根因
```rust
// excel-diff/src/formula_tracker.rs:136
pub fn extract_cell_refs(formula: &str) -> HashSet<String> {
    let formula = strip_all_sheet_prefixes(formula);
    if !formula.starts_with('=') {
        return refs;          // ← calamine 读出的公式不带前导 '='，直接返回空集
    }
    ...
}
```

### 修改方案
calamine 存储公式**不含 `=`**（如 `"A1+B1"`）。在去掉表前缀后，统一补 `=`：
```rust
pub fn extract_cell_refs(formula: &str) -> HashSet<String> {
    let stripped = strip_all_sheet_prefixes(formula);
    // calamine 公式不带前导 '='，统一归一化
    let formula = if stripped.starts_with('=') {
        stripped
    } else {
        format!("={}", stripped)
    };
    if formula.is_empty() {
        return refs;
    }
    let formula = &formula[1..];
    ...
}
```
> 同样需检查 `formula_tracker.rs` 内其它以 `starts_with('=')` 为前提的解析入口（如依赖图构建），统一在入口归一化，避免重复判断。

### 验证
- D1 依赖 B1→C1 变更 → `modified_dependencies` 含 `S!B1`/`S!C1`（T7.20 复测）。
- 构造 A1=`=B1`, B1=`=A1` → 检出 `cycles_introduced`（T7.22 复测）。

---

## F3 — 单行工作表的公式变更被漏报

### 现象
`C1: =A1+B1 → =A1*B1` 在**只有一行**的表里报 "No changes"；同样改动**多加一行普通数据后就能检出**（T7.23 vs T7.24）。检出与否取决于表形状而非实际改动。

### 根因（假设，需复测确认）
与 F1-2 同源：单行表的公式网格与值网格原点错位时，公式被错挂，导致 `classify_diff` 比较的 `old.formula`/`new.formula` 与预期单元格不匹配，从而判定 `NoChange`。**F1(a) 的绝对坐标公式查找很可能同时修好 F3**。

### 修改方案
1. 先实施 F1(a)，再用下方回归测试复测 F3；若已通过，本项无需额外改动。
2. 若 F1(a) 后仍漏报，则需深入 `compute_cell_diffs` / `classify_diff`（helpers.rs:6）检查单行（`max_row==1` 或单行单列）分支是否走了不同路径（如 `all_cells_as_diff` 而非 `compute_cell_diffs`，或 `semantic` 层短路）。

**回归测试（必须加入）**：
```rust
// 单行表：C1 公式变更应被检出
let old = sheet_with_one_row("C1", "=A1+B1");
let new = sheet_with_one_row("C1", "=A1*B1");
let diffs = compute_cell_diffs(&old, &new);
assert!(diffs.iter().any(|d| d.cell_ref == "C1" && d.diff_type == DiffType::Modify));
```

### 验证
- 单行表公式变更 → 检出 `C1` Modify（T7.23 复测）；加行后行为保持一致（T7.24 复测）。

---

## F4 — git diff driver 拿不到临时文件路径（完全失效）

### 现象
`install-git-driver` 能正确写入 `.gitattributes` 与 git config（T7.15/T7.17 通过），但真正 `git diff` 时：
```json
{"message":"IO error: No such file or directory (os error 2)","success":false}
```
驱动拿不到 git 传入的临时文件路径（T7.16/T7.27）。

### 根因
git 的 `diff.<name>.command` 协议把驱动叫成：
`<command> <path> <old-file> <old-hex> <old-mode> <new-file> <new-hex> <new-mode>`。
本项目的注册命令是 `excel-cli diff git-driver`，所以 argv = `[excel-cli, diff, git-driver, <path>, <old>, ...]`。

但 `parse_git_driver_args` 只跳过字面 `"git-driver"` 这一个 token：
```rust
// excel-diff/src/git_driver.rs:397
let mut iter = args.iter().skip(1).skip_while(|a| a.as_str() == "git-driver");
// skip(1) 去掉 exe；skip_while 在遇见 "diff"（≠"git-driver"）处停止
// → _path = "diff"（错），old_file = "git-driver"（错！），new_file = <old 临时路径>
```
于是 `old_file` 被赋成字面量 `"git-driver"`，`diff_files("git-driver", "<old临时路径>")` → 文件不存在 → "No such file or directory"。

### 修改方案
跳过**所有**已知子命令 token（不只 `git-driver`）：
```rust
// excel-diff/src/git_driver.rs:397
let mut iter = args
    .iter()
    .skip(1)
    .skip_while(|a| matches!(a.as_str(), "diff" | "git-driver" | "file"));
// 之后：_path = <path>, old_file = <old-file>, new_file = <new-file> —— 对齐 git 7 参协议
let _path = iter.next()?;
let old_file = iter.next()?;
let _ = iter.next();
let _ = iter.next();
let new_file = iter.next()?;
```
> 顺带建议：把 `excel-diff` 二进制自身的 `main.rs`（当前仅当 `args[1]=="git-driver"` 才进入驱动）也改为"argv 长度 ≥ 2 且含已知子命令即进入"，与 `parse_git_driver_args` 的跳过集合保持一致，避免二进制直接调用时再次错位。

### 验证
- 在 git 仓库中对 xlsx 做真实修改后 `git diff` → 输出自然语言 diff（T7.16/T7.27 复测）。
- `excel-cli diff git-driver <path> <old> <old-hex> <old-mode> <new> <new-hex> <new-mode>` 直接调用 → 正确读取两个临时文件。

---

## 工作量

| 项 | 改动量 | 风险 |
|----|--------|------|
| F1 | 公式绝对坐标 ~4 行 + SheetData 加字段 + ~30 处字面量机械更新 + diff 引擎加偏移 | 中 |
| F2 | `formula_tracker.rs` ~6 行 | 低 |
| F3 | 依赖 F1(a)；加回归测试 | 低（若 F1 已覆盖） |
| F4 | `git_driver.rs` ~3 行 + main.rs 对齐 | 低 |

**合计约 3–4 小时**（含编译 + t7 套件复测 + 新增回归测试）。F1 完成后建议先跑 t7 确认 F1/F2/F3/F4 全部回归通过，再处理 [02-数据损坏](./02-数据损坏-D1D2D3.md) 中关联的非 A1 写路径待解决问题。
