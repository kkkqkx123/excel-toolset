# CLI 缺陷修复方案：C1 / C2 / C3 / C4 / C5

**模块**：`excel-cli`（crate）+ `excel-core` 的 `features/` 与 `excel_write/batch.rs`。

---

## C1 — 出错永不返回非零退出码（CI/脚本失效）

### 现象
`main()` 从不 `process::exit`，所有错误 `rc=0`。CI/脚本无法判定失败（T1.26/T1.27/T7.18）。

### 根因
```rust
// excel-cli/src/main.rs:3
fn main() {
    let cli = cli::Cli::parse();
    cli::execute(&cli);     // 返回值被丢弃
}
// runners.rs execute() 在 Err(e) 时仅打印 JSON，正常返回 ()，退出码恒为 0
```

### 修改方案
让 `execute` 传播错误，由 `main` 决定退出码：
```rust
// runners.rs
pub fn execute(cli: &Cli) -> Result<()> {     // ← 返回 Result
    match run_command(cli) {
        Ok(json) => { /* 原打印逻辑 */ Ok(()) }
        Err(e) => {
            // 原 err_json 打印逻辑保留
            Err(e)                            // ← 向上传播
        }
    }
}
```
```rust
// main.rs
fn main() {
    use clap::Parser;
    let cli = cli::Cli::parse();
    if let Err(_e) = cli::execute(&cli) {
        std::process::exit(1);   // ← 失败返回非零
    }
}
```
> `run_command` 已返回 `Result<serde_json::Value, AppError>`，类型对齐，无需改动其签名。

### 验证
- 对不存在的文件执行写操作 → `echo $?` 为 `1`（T1.26/T1.27 复测）。
- 正常操作 → `rc=0` 不变。

---

## C2 — `comments add` 不落盘到 xlsx

### 现象
`comments add` 返回 `success:true`，但 xlsx 内**没有 comment 部件**；`comments get` 返回的文本是孤证（T5.10/T5.11）。根因：注释被写入侧车文件 `*.comments.json`，从未进入 xlsx 包。

### 根因
```rust
// excel-core/src/features/comments.rs:16
fn comments_sidecar_path(xlsx_path: &str) -> String {
    format!("{}.comments.json", xlsx_path)   // ← 侧车文件，非 xlsx 部件
}
```
`add_comment` / `update_comment` 只 `save_comments`（写侧车），不触碰 xlsx。

### 修改方案（写入真实 xlsx 注释部件）
rust_xlsxwriter 0.95.0 **已支持** `Worksheet::insert_note()`（note.rs:78 / worksheet.rs:5858）。改为在 xlsx 内写入 note：
```rust
// add_comment 内（dry_run 分支保留）
let (r, c) = crate::utils::cell_ref::parse_cell_ref(cell)?;
excel_write::modify_file_with_wb(path, &params, |_, wb| {
    let ws = wb
        .worksheet_from_name(sheet)
        .map_err(|_| AppError::SheetNotFound(sheet.to_string()))?;
    let mut note = rust_xlsxwriter::Note::new(comment_text);
    if let Some(a) = author { note = note.set_author(a); }
    ws.insert_note(r, c, &note).map_err(AppError::Xlsx)?;
    Ok(())
})?;
```
**读取回显**：calamine 不暴露注释，rust_xlsxwriter 也不读注释。建议：
- **主修复**：写入真实 xlsx note（ClosedXML/openpyxl 可见，解决"虚假成功"）。
- **读取**：保留侧车 JSON 作为 `comments get` 的回显来源（与 xlsx note 双写），并在文档说明侧车仅用于读回、真实注释在 xlsx 内。或未来接入能读注释的库。

### 验证
- `comments add` 后用 **ClosedXML（C#）** 打开 xlsx，`Worksheet.Cell("C1").Comment` 非空（T5.10/T5.11 复测）。

---

## C3 — `pivot-table create` 虚假成功

### 现象
返回 `success:true`，但**不生成任何 pivotTable XML 部件**，只在目标位置写普通格式化单元格（T5.12/T9.14）。rust_xlsxwriter 原生不支持数据透视表（模块头注释已自承）。

### 根因
```rust
// excel-core/src/features/pivot_table.rs:63
crate::excel_write::modify_file_with_wb(path, &params, |_, wb| {
    ...
    write_cell_value_to_worksheet(worksheet, row, col, cell_value)?;  // ← 只写扁平聚合结果
    ...
});
```

### 修改方案（诚实化，避免虚假成功）
实现真实 pivot XML 部件工作量很大（需手写 `pivotTable*.xml` 并注册到 workbook），列为**后续大项（待解决）**。当前务实修复：

1. **不再谎称生成 pivot**：`create_pivot_table` 返回时 `message` 明确说明"已写入聚合汇总（flattened summary），非 Excel 原生数据透视表"，或新增 `is_summary: true` 标志。
2. 同步更新 `skills` 文档（见 [07-文档校订](./07-文档校订-P3.md)），将 `pivot-table` 描述为"聚合汇总写入"，字段以真实 schema（`name/source_range/target_sheet/target_cell/row_fields/data_fields`）为准。
3. （可选增强）若确需真 pivot，future：用 `zip` 直接注入 pivotTable 部件并修改 `[Content_Types].xml` / `workbook.xml.rels` —— 超出本次范围。

### 验证
- 调用后返回信息明确标注"summary，非原生 pivot"，不再误导（T5.12/T9.14 复测）。
- 文档中 `pivot-table` 字段与实现一致。

---

## C4 — 公式求值不持久化

### 现象
`formula eval` / `formula set --eval` 不写回计算值，缓存值仍是 0（T2.12）。在 D2 已复用 `cell.value` 作为 `set_result` 的前提下，只要把求值结果写入 `CellData.value`，公式缓存值即可正确落盘。

### 根因
`features/formula_eval.rs` 的 `evaluate_formula`（第 21 行，返回 `CellValue`）与第 249 行 `let value = result.to_cell_value();` 仅用于**展示**，未回写到写路径的 `CellData.value`。

### 修改方案
- **`formula set --eval`**：求值后，把 `CellValue` 转成字符串写入目标单元格的 `CellData.value`，再经 `write_cell_data` 写出（D2 修复后自动 `set_result`）。
  ```rust
  // 伪代码：在 set --eval 的处理分支
  let cell_value = evaluate_formula(path, sheet, cell, formula)?;
  let cached = cell_value_to_string(&cell_value);   // 复用 formula_eval 既有辅助
  // 构造 CellData { value: Some(cached), formula: Some(formula), data_type: ... }
  // 经 excel_write 写回
  ```
- **`formula eval`（只读）**：保持展示求值结果，但提示"未写回"或提供 `--persist` 开关；与 `--eval` 写回路径共用同一求值逻辑。

### 验证
- `formula set --eval` 后，openpyxl `data_only=True` 读该单元格 ≠ 0（T2.12 复测，依赖 D2 一并修复）。

---

## C5 — 批处理 `write_cell` 行列是 0-based（坐标错位）

### 现象
批处理 `write_cell` 的 `row:1, col:1` 文档示例暗示 A1，实际落在 **B2**（T4.16）。而单格 `cell write` 用 A1（1-based），两套 API 坐标约定不一致。

### 根因
```rust
// excel-core/src/excel_write/batch.rs:331
BatchOperation::WriteCell { sheet, row, col, .. } => match super::data_mut::write(data, sheet, *row, *col, value) {
    // data 是 0-based SheetData，*row/*col 被当作 0-based 索引 → row:1,col:1 = B2
```
`data_mut::write` 按 0-based 索引写 `sheet_data.rows[*row][*col]`。

### 修改方案（对齐 1-based，与单格 API 及文档一致）
在 `WriteCell` 处理处把 1-based 入参转成 0-based 内部索引（一次转换，校验与写入共用）：
```rust
BatchOperation::WriteCell { sheet, row, col, .. } => {
    let r = (*row).saturating_sub(1) as usize;   // 1-based → 0-based
    let c = (*col).saturating_sub(1) as usize;
    // 校验（原 lines 36-48）改用 r/c
    // ...
    match super::data_mut::write(data, sheet, r, c, value) { ... }
}
```
- 同步更新 `BatchOperation::WriteCell` 的 JSON schema 描述（`row`/`col` 注明为 1-based，与 `cell write` 一致）。
- 更新所有内部构造 `WriteCell { row, col }` 的测试/调用点（如有 0-based 用法需 +1）。

> 若团队更倾向"批处理保持 0-based、仅改文档"，取舍见 [07-文档校订](./07-文档校订-P3.md)；但本方案按"代码对齐文档+单格 API"处理，避免用户踩坑。

### 验证
- 批处理 `write_cell` `row:1,col:1` → 落在 **A1**（T4.16 复测）。

---

## 工作量

| 项 | 改动量 | 风险 |
|----|--------|------|
| C1 | `main.rs` 2 行 + `runners.rs` 返回类型 | 低 |
| C2 | `comments.rs` 改用 `insert_note` + 读取策略 | 中（读回依赖侧车） |
| C3 | 诚实化 message + 文档 | 低 |
| C4 | `formula_eval` 写回 `CellData.value` | 中（依赖 D2） |
| C5 | `batch.rs` 1-based 转换 + schema 文档 | 低 |

**合计约 3–4 小时**。C4 与 D2 强耦合，须与 [02-数据损坏](./02-数据损坏-D1D2D3.md) 一同落地；C2 的真实 note 写入可独立验证（用 ClosedXML）。
