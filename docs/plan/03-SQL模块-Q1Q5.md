# SQL 模块修复方案：Q1 / Q2 / Q3 / Q4 / Q5

**模块**：`excel-sql`（crate）。验证结果 16 项测 15 项失败，根因集中在 `db/query.rs` 与 `db/loader.rs`。

---

## Q1 — 任何合法查询都 panic（阻断级）

### 现象
```text
$ excel-cli data sql sales.xlsx Sales "SELECT 1"
thread 'main' panicked at duckdb-1.10504.0/src/raw_statement.rs:91:21:
The statement was not executed yet
```
只有**非法** SQL 才返回正常 JSON；一旦 SQL 合法，`column_count()` 必崩（T6.03、T6.14）。

### 根因
```rust
// crates/excel-sql/src/db/query.rs:145
pub fn query(db: &duckdb::Connection, sql: &str) -> Result<QueryResult, AppError> {
    let mut stmt = db.prepare(sql)?;
    let col_count = stmt.column_count();   // ← 执行前调用，duckdb-rs 直接 panic
    ...
}
```
duckdb-rs 要求**先执行**才能读取列元数据。`Statement::column_count()` 在 `result_unwrap()` 处 `.expect("The statement was not executed yet")`。`query_with_params`（第 190 行）有完全相同的错误。

### 修改方案（遵循 duckdb-rs 官方推荐）
先 `query([])` 执行，再通过 `rows.as_ref().unwrap()` 读取列元数据（每次 `as_ref()` 是临时借用，不会与后续 `rows.next()` 冲突）：

```rust
pub fn query(db: &duckdb::Connection, sql: &str) -> Result<QueryResult, AppError> {
    let mut stmt = db.prepare(sql).map_err(|e| AppError::DuckDb(e.to_string()))?;

    // ✅ 先执行
    let mut rows = stmt
        .query([])
        .map_err(|e| AppError::DuckDb(e.to_string()))?;

    // ✅ 执行后读取列元数据（duckdb-rs 要求先执行）
    let col_count = rows.as_ref().expect("statement available after query").column_count();
    let columns: Vec<String> = (0..col_count)
        .map(|i| {
            rows.as_ref()
                .expect("statement available")
                .column_name(i)
                .map_or_else(String::new, |s| s.to_string())
        })
        .collect();

    let mut result_rows = Vec::new();
    while let Some(row) = rows.next().map_err(|e| AppError::DuckDb(e.to_string()))? {
        let mut cells = Vec::with_capacity(col_count);
        for i in 0..col_count {
            let value: duckdb::types::Value =
                row.get(i).unwrap_or(duckdb::types::Value::Null);
            cells.push(duckdb_to_cell(&value));
        }
        result_rows.push(cells);
    }

    Ok(QueryResult {
        columns,
        rows: result_rows,
        row_count: result_rows.len(),
    })
}
```

`query_with_params` 同样改法，仅把 `stmt.query([])` 换成 `stmt.query(duckdb::params_from_iter(params.iter()))`。

### 验证
- `excel-cli data sql sales.xlsx Sales "SELECT 1"` → 返回 JSON 且 `rc=0`（T6.03/T6.14 复测）。
- `SELECT * FROM Sales`、`SELECT Amount FROM Sales WHERE Quantity > 5` 不再 panic。

---

## Q2 / Q3 — 文档说的表名 `t`、列名 `A,B,C` 不存在

### 结论
这是**文档与实现的偏差**，非代码 bug。实现设计上：
- 表名 = **工作表名**（`Sales`/`Config`），且所有工作表都会被加载（T6.01）。
- 列名 = **表头行文本**（如 `Amount`），非 `A,B,C`（T6.04）。

### 处理
- 代码层面**无需修改**（语义合理）。
- 文档层面：将 `skills/excel/references/cli-reference.md` 中"表 `t` / 列 `A,B,C`"更正为"表=工作表名、列=表头文本"。详见 [07-文档校订](./07-文档校订-P3.md)。

---

## Q4 — 数值列被推断为 VARCHAR，聚合全废

### 现象
`SELECT SUM(y) FROM Nums` → `No function matches 'sum(VARCHAR)'`；`WHERE Quantity > 5` → `Cannot compare VARCHAR and INTEGER_LITERAL`（T6.05/06/07/13）。

### 根因
```rust
// crates/excel-sql/src/db/loader.rs:174（load_sheet_to_db）
let type_rows = collect_row_types(&data.rows);   // ← 包含表头行，文本表头把整列拖成 VARCHAR
let col_types = infer_column_types(&type_rows);
```
注意 `load_sheet_with_row_id`（第 211 行）已用 `rows_to_load`（排除表头）做推断，**是正确的**；只有 `load_sheet_to_db` 漏了这一步。

### 修改方案
```rust
// crates/excel-sql/src/db/loader.rs:174（load_sheet_to_db）
// 类型推断排除表头行
let header_excluded: &[Vec<CellData>] =
    if has_header && data.rows.len() > 1 { &data.rows[1..] } else { &data.rows };
let type_rows = collect_row_types(header_excluded);
let col_types = infer_column_types(&type_rows);
```
> 用 `data.rows.len() > 1` 守护：仅表头无数据时不越界；`create_table_with_header` 仍收到 `header` 与（可能为空的）`col_types`，行为与原"无数据"路径一致。

### 验证
- 构造数值列 `Nums.y` 为整数，写库后 `SELECT SUM(y) FROM Nums` 返回正确求和（T6.05 复测）。
- `WHERE Quantity > 5` 正常比较（T6.06 复测）。

---

## Q5 — `<sheet>` 参数被忽略；无表头文件丢首行

### 现象
`sql_query(path, _sheet, query)` 中 `_sheet` **从未使用**，传任意不存在的表名都不报错（T6.10/T6.16）；`has_header` 硬编码 `true`（query.rs:120），无表头工作表**首行数据被当表头吃掉**，列名变 `col_1`（T6.15）。

### 根因
```rust
// crates/excel-core/src/operations/query.rs:117
pub fn sql_query(path: &str, _sheet: &str, query: &str) -> Result<Vec<Vec<CellData>>> {
    let data = excel_read::read_all_sheets_to_map(path)?;
    let sheets: Vec<SheetData> = data.into_values().collect();
    let result = excel_sql::sql_query_on_data(&sheets, query, true)?;  // ← has_header 硬编码
    Ok(result.rows)
}
```

### 修改方案
1. **真正使用 `sheet` 参数**：只读目标表（传入不存在的表名应报错，符合 SQL 语义）。
2. **`has_header` 参数化**：由 CLI 透传（默认 `true` 保持现有行为），避免无表头文件丢首行。

```rust
// crates/excel-core/src/operations/query.rs
pub fn sql_query(
    path: &str,
    sheet: &str,
    query: &str,
    has_header: bool,          // ← 新增，由调用方透传
) -> Result<Vec<Vec<CellData>>> {
    // 真正按 sheet 读取（不存在则报错）
    let data = excel_read::read_sheet_all(path, sheet)
        .map_err(|_| AppError::SheetNotFound(sheet.to_string()))?;
    let sheets = vec![data];
    let result = excel_sql::sql_query_on_data(&sheets, query, has_header)?;
    Ok(result.rows)
}
```
> 若需保留"一次查询多表"能力，可改为 `read_all_sheets_to_map` 后 `filter(|(name, _)| name == sheet)`；但当前 SQL 用法是单表，单表读取更直接且能正确报错。

3. **CLI 调用点**（`excel-cli/src/cli/runners.rs` 中 `data sql` 分支）补充 `has_header` 实参（可由新增 `--no-header` 标志控制，默认 `true`）。

### 验证
- `data sql x.xlsx NonExist "SELECT 1"` → 返回明确 `SheetNotFound` 错误，而非静默成功（T6.10/T6.16 复测）。
- 无表头工作表查询，首行数据保留为数据而非列名（T6.15 复测）。

---

## 工作量

| 项 | 改动量 | 风险 |
|----|--------|------|
| Q1 | `query.rs` 两函数 ~25 行重写 | 中（需复核 `rows` 借用） |
| Q2/Q3 | 0（仅文档） | 无 |
| Q4 | `loader.rs` 2 行 | 低 |
| Q5 | `operations/query.rs` + CLI 调用点 ~15 行 | 低 |

**合计约 2–3 小时**（含 `--features sql` 重新编译 + t6 套件全量复测）。Q1 修复后 t6 应从 15/16 失败降到以 Q4/Q5 逻辑错误为主的可修复区间。
