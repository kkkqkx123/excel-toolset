# Phase 7: 数据类型修复

## 背景

经分析，当前项目对 Excel 数据类型的处理存在若干缺陷，尤其是**错误值类型**在读-写回流中语义丢失。详见 `docs/plan/data-type-analysis-report.md`。

## 修改方案

### Phase 7.1: 错误值显示格式标准化

**问题**：`excel_read.rs` 使用 `{:?}` 输出 CellErrorType（如 `"Div0"`），而非 Excel 标准格式（如 `#DIV/0!`）；`search.rs` 使用 `"ERROR: {:?}"` 格式，两者不一致。

**方案**：利用 calamine 的 `CellErrorType::fmt()`（Display trait）输出标准格式，删除 `search.rs` 中的 `"ERROR: "` 前缀，统一为标准错误字符串。

**涉及文件**：
- `crates/excel-core/src/excel_read.rs`
- `crates/excel-core/src/search.rs`

---

### Phase 7.2: CellValue 增加 Error 变体

**问题**：`CellValue` 枚举没有 `Error` 变体，用户无法通过写入 API 输入错误值；`cell_value_to_data` 无法将 `CellValue::Error` 映射为 `CellData { data_type: CellDataType::Error }`。

**方案**：
1. 在 `CellValue` 增加 `Error(String)` 变体（String 存放 Excel 标准错误码，如 `"#REF!"`）
2. `parse_cell_value` 增加对 `#DIV/0!`、`#N/A` 等错误值的识别
3. `parse_cell_value_grid` 增加对 JSON null/string 形式错误值的处理
4. `cell_value_to_data`（两个副本：`excel_write/util.rs` 和 `excel_data.rs`）增加 `CellValue::Error` → `CellData { data_type: CellDataType::Error }` 的映射

**涉及文件**：
- `crates/excel-types/src/cell.rs`
- `crates/excel-core/src/helpers.rs`
- `crates/excel-core/src/excel_write/util.rs`
- `crates/excel-core/src/excel_data.rs`

---

### Phase 7.3: write_cell_data 处理 Error 类型

**问题**：`write_cell_data` 对 `CellDataType::Error` 走 `_` 通配分支，调用 `write_string` 将错误值降级为文本。

**方案**：增加 `CellDataType::Error` 的显式匹配分支。由于 rust_xlsxwriter 没有公开的错误值写入 API，采用公式变通方案：
- 将错误值以公式形式写入（如 `=1/0` 产生 `#DIV/0!`）
- 对无法映射为公式的错误类型，保持降级为字符串写入，但记录日志

**涉及文件**：
- `crates/excel-core/src/excel_write/write.rs`

---

### Phase 7.4: set_formula 类型标记修复

**问题**：`set_formula` 恒将 `data_type` 设为 `String`，即使公式计算结果可能是数值、布尔或错误。

**方案**：设置公式时将 `data_type` 设为 `Empty` 而不是 `String`，因为公式的实际类型由 Excel 求值后决定，不应由调用方指定。

**涉及文件**：
- `crates/excel-core/src/excel_write/cell.rs`

---

### Phase 7.5: DurationIso 类型映射

**问题**：calamine 的 `DurationIso`（ISO 8601 持续时间，如 `"PT1H30M"`）被映射为 `CellDataType::Float`，丢失类型信息。

**方案**：`CellDataType` 增加 `Duration` 变体，并在读取/写入路径中相应处理。

**涉及文件**：
- `crates/excel-types/src/cell.rs`
- `crates/excel-core/src/excel_read.rs`
- `crates/excel-core/src/excel_write/write.rs`
- `crates/excel-core/src/excel_write/util.rs`
- `crates/excel-core/src/excel_data.rs`
- `crates/excel-sql/src/converter/cell_convert.rs`

---

## 各 Phase 风险与依赖

| Phase | 风险 | 依赖 |
|-------|------|------|
| 7.1 | 无 | 无 |
| 7.2 | `CellValue` 变更影响 `excel-data`、`excel-write` 等下游 | 7.1 |
| 7.3 | rust_xlsxwriter 限制导致部分错误类型无法正确写入 | 7.2 |
| 7.4 | 可能影响依赖公式类型的下游逻辑 | 无 |
| 7.5 | `CellDataType` 枚举新增变体可能影响所有 match 语句 | 无 |