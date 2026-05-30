# 阶段2：读写核心能力

**目标**：实现 Excel 读取和写入的所有原子操作，覆盖文件/工作簿、工作表、单元格/区域三大类别。
**产出**：可独立测试的读写函数库，覆盖 7 大类原子操作中的核心部分。

---

## 2.1 Excel 读取模块（excel_read.rs）

基于 `calamine` 实现只读原子操作。

| 原子操作 | 函数签名 | 说明 |
|----------|---------|------|
| 读取文件信息 | `read_file_info(path) -> Result<FileInfo>` | sheet 列表、行列数、文件哈希 |
| 列出工作表 | `list_sheets(path) -> Result<Vec<String>>` | 所有 sheet 名称 |
| 读取单格 | `read_cell(path, sheet, row, col) -> Result<CellData>` | 值+类型+公式 |
| 读取区域 | `read_range(path, sheet, range) -> Result<Vec<Vec<CellData>>>` | 二维数组 |
| 读取公式 | `read_formula(path, sheet, cell) -> Result<Option<String>>` | 公式文本 |
| 读取样式 | `read_style(path, sheet, cell) -> Result<Style>` | 字体/颜色/对齐（calamine 有限支持） |
| 读取所有数据 | `read_sheet_all(path, sheet) -> Result<SheetData>` | 全量数据（供 diff 使用） |

**关键实现细节**：
- 使用 `calamine::open_workbook` 打开文件
- 通过 `worksheet_range()` 读取数据区域
- 区分 `DataType::String`, `Float`, `Int`, `DateTime`, `Bool`, `Error`
- 公式通过 `worksheet_formula()` 获取
- 文件信息同时计算 SHA-256 指纹

## 2.2 Excel 写入模块（excel_write.rs）

基于 `rust_xlsxwriter` 实现所有写入/修改原子操作。

**核心模式**（`rust_xlsxwriter` 只写不修改，所有写入操作遵循此模式）：

```
calamine 读取原文件 -> 内存构建新 Workbook -> rust_xlsxwriter 写入数据 -> 保存覆盖
```

| 原子操作 | 函数签名 | 说明 |
|----------|---------|------|
| 创建新文件 | `create_file(path, sheet_name) -> Result<()>` | 含默认 sheet |
| 保存文件 | `save_file(workbook, path) -> Result<()>` | 内部调用，配合安全组件 |
| 新增工作表 | `add_sheet(path, sheet) -> Result<()>` | 读→加 sheet→写回 |
| 删除工作表 | `delete_sheet(path, sheet) -> Result<()>` | 读→跳过→写回 |
| 重命名工作表 | `rename_sheet(path, old, new) -> Result<()>` | 读→改名→写回 |
| 写入单格 | `write_cell(path, sheet, row, col, value) -> Result<WriteResult>` | 含 diff 返回 |
| 写入区域 | `write_range(path, sheet, range, data) -> Result<WriteResult>` | 批处理 |
| 清空区域 | `clear_range(path, sheet, range) -> Result<WriteResult>` | 写入空白覆盖 |
| 设置公式 | `set_formula(path, sheet, cell, formula) -> Result<WriteResult>` |  |
| 设置样式 | `set_format(path, sheet, range, format) -> Result<WriteResult>` | 字体/颜色/边框/对齐 |
| 合并单元格 | `merge_cells(path, sheet, range) -> Result<WriteResult>` |  |

**WriteResult 结构**：
```rust
pub struct WriteResult {
    pub success: bool,
    pub message: String,
    pub backup_info: Option<BackupInfo>,
    pub old_hash: String,
    pub new_hash: String,
    pub diff: Option<Vec<CellDiff>>,
}
```

### 2.2.1 写操作的统一内部流程

```
1. compute_file_hash → 记录 old_hash
2. create_backup → 记录 backup_info
3. 判断 dry_run → 若是则进入 dry_run 路径
4. calamine 读取原文件全量数据
5. 创建新 Workbook，逐 sheet 复制原数据
6. 执行目标写入/修改操作
7. excel_diff 对比新旧数据 → 生成 diff
8. 非 dry_run：save 到原路径；dry_run：save_to_buffer 丢弃
9. compute_file_hash → 记录 new_hash
10. 组装 WriteResult 返回
```

## 2.3 单元格类型适配

Rust 数据类型到 Excel 单元格类型的映射：

| Rust 类型 | Excel 写入 API |
|-----------|---------------|
| `String` | `worksheet.write_string()` |
| `f64` | `worksheet.write_number()` |
| `i32/i64` | `worksheet.write_number()` |
| `chrono::NaiveDateTime` | `worksheet.write_datetime()` |
| `bool` | `worksheet.write_boolean()` |
| `&str` (formula) | `worksheet.write_formula()` |

## 2.4 通用工具函数

| 函数 | 说明 |
|------|------|
| `parse_cell_ref(ref: &str) -> (u32, u16)` | "A1" → (0,0) |
| `parse_range(range: &str) -> (u32,u16,u32,u16)` | "A1:C3" 行列转换 |
| `col_to_index(col: &str) -> u16` | "A" → 0, "AA" → 26 |
| `index_to_col(idx: u16) -> String` | 0 → "A" |
| `ensure_sheet_exists(path, sheet) -> Result<()>` | 校验 sheet 存在 |

## 2.5 验证标准

- [ ] `excel_read` 所有读取函数正确解析 `.xlsx` 文件
- [ ] `excel_write` 创建/写入/公式/样式函数正确生成文件
- [ ] 修改操作遵循「读→新建→写回」模式
- [ ] 写操作自动集成安全组件（指纹 → 备份 → dry_run）
- [ ] 单元格引用解析函数正确（A1 ↔ 行列索引）
- [ ] 所有函数单元测试通过
- [ ] 集成测试：创建文件→写入→读取→验证内容一致
