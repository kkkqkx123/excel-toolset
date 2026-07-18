# P0 功能补充 -- 分阶段修改方案

基于 `excel-toolset-vs-officecli-analysis.md` 分析结论，P0 优先级包含三个任务：公式求值引擎、切片器、计算字段。本方案将三者拆分为独立阶段，每阶段内部按层次递进。

---

## 总览

| 阶段 | 任务 | 预估工作量 | 依赖 |
|------|------|-----------|------|
| A | 公式求值引擎 | 70% | 无 |
| B | 切片器 (Slicer) | 15% | 无 |
| C | 计算字段 (Calculated Field) | 15% | 建议在 B 之后（共用透视表扩展接口） |

每个阶段内部按 `类型定义 -> 核心逻辑 -> CLI/HTTP/MCP 入口 -> 测试验证` 的顺序推进。

---

## 阶段 A：公式求值引擎

### 背景

当前 excel-toolset 仅将公式作为字符串写入，依赖 Excel 客户端打开时求值。在无头服务端场景下（自动报表生成、数据校验），缺少求值能力是根本性限制。

### 目标

实现一个独立的公式求值引擎，写入公式后自动触发求值，支持 350+ Excel 内置函数、动态数组溢出、查找引用、金融、统计函数。

### 架构设计

新增 crate `excel-formula` 作为独立公式求值引擎：

```
crates/
├── excel-formula/         # 新增：公式求值引擎
│   ├── src/
│   │   ├── lib.rs         # crate 入口，导出 Engine
│   │   ├── engine.rs      # 求值引擎核心：parse → evaluate → spill
│   │   ├── parser.rs      # 公式解析器 (A1/R1C1 引用、函数调用、运算符)
│   │   ├── functions/     # 内置函数实现
│   │   │   ├── mod.rs
│   │   │   ├── math.rs    # SUM/AVERAGE/ROUND/ABS 等数学函数
│   │   │   ├── lookup.rs  # VLOOKUP/XLOOKUP/INDEX/MATCH/OFFSET/INDIRECT
│   │   │   ├── dynamic.rs # FILTER/SORT/UNIQUE/SEQUENCE/LET/LAMBDA/MAP
│   │   │   ├── financial.rs # XIRR/PRICE/YIELD/DURATION/COUPNUM
│   │   │   ├── statistical.rs # NORM.DIST/T.TEST/LINEST 等统计函数
│   │   │   ├── text.rs    # CONCATENATE/LEFT/RIGHT/MID 等文本函数
│   │   │   ├── logical.rs # IF/AND/OR/NOT/IFERROR 等逻辑函数
│   │   │   └── datetime.rs # DATE/YEAR/TODAY 等日期函数
│   │   ├── evaluator.rs   # 递归 AST 求值器
│   │   └── spill.rs       # 动态数组溢出计算
│   └── Cargo.toml
```

**依赖关系**：`excel-formula` 仅依赖 `excel-types`，不依赖 `excel-core`（避免循环依赖）。

### A.1：基础设施搭建

**范围**：创建 `excel-formula` crate，实现公式解析器与基础求值框架。

**实现内容**：

1. 创建 `crates/excel-formula/Cargo.toml`，依赖 `excel-types`、`calamine`、`regex`、`chrono`，加入 workspace
2. 实现 `parser.rs`：公式字符串 → AST 节点树
   - 支持单元格引用（A1 风格 + R1C1 风格）、区域引用（A1:B10）
   - 支持运算符（`+` `-` `*` `/` `^` `&` `:` `,` `%`）
   - 支持函数调用 `FUNC(args...)`、括号嵌套
   - 识别动态数组函数标记（用于后续溢出处理）
3. 实现 `engine.rs`：`FormulaEngine` 结构体
   - `new(data_provider: Arc<dyn DataProvider>)` 创建引擎实例
   - `DataProvider` trait：抽象数据源接口，支持从 calamine workbook 读取
   - `evaluate(sheet: &str, cell: &str, formula: &str) -> Result<CellValue>`
   - 函数注册表（HashMap）：函数名 → 实现闭包
4. 实现 `functions/math.rs`：第一批数学/聚合函数
   - SUM、AVERAGE、COUNT、COUNTA、MIN、MAX、PRODUCT
   - ROUND、ROUNDUP、ROUNDDOWN、ABS、SQRT、POWER
   - SUMIF、SUMIFS、COUNTIF、COUNTIFS
   - SUBTOTAL（支持 function_num 参数控制忽略隐藏行）

**验证**：单元测试覆盖解析器正确性 + 基础函数求值。

### A.2：查找引用函数

**范围**：实现 VLOOKUP/XLOOKUP/INDEX/MATCH/OFFSET/INDIRECT。

**实现内容**：

1. `functions/lookup.rs`：
   - VLOOKUP（近似/精确匹配，列偏移）
   - XLOOKUP（双向匹配，支持 not_found 回退值）
   - INDEX（行列索引，区域多区域模式）
   - MATCH（match_type: 0/1/-1）
   - OFFSET（rows/cols 偏移，可选 height/width 返回区域）
   - INDIRECT（跨工作表引用，R1C1 模式）
   - HLOOKUP
   - CHOOSE、ROW、COLUMN、ROWS、COLUMNS、ADDRESS
2. `DataProvider` trait 扩展：支持跨工作表数据访问

**验证**：对照 Excel 参考值编写参数化测试。

### A.3：动态数组溢出

**范围**：实现动态数组函数及溢出机制。

**实现内容**：

1. `spill.rs`：
   - `SpillResult` 类型：包含溢出范围（行列）和二维值数组
   - 溢出冲突检测：若溢出区域被已有数据占据，返回 `#SPILL!` 错误
   - 隐式交叉（`@` 运算符）处理：当公式期望单值但参数是区域时自动应用
2. `functions/dynamic.rs`：
   - FILTER（条件过滤 + 空结果处理）
   - SORT / SORTBY（多键排序）
   - UNIQUE（按行/列去重）
   - SEQUENCE（等差数列生成）
   - LET（变量绑定）
   - LAMBDA / MAP / REDUCE / SCAN / BYROW / BYCOL
3. 引擎修改：`evaluate` 返回 `FormulaResult` 枚举（Single / Spilled）

**验证**：溢出结果的行列数正确，溢出冲突返回错误。

### A.4：金融与统计函数

**范围**：补充金融和统计函数，达到对标 OfficeCLI 的覆盖度。

**实现内容**：

1. `functions/financial.rs`：
   - XIRR / IRR / NPV / XNPV（不等间距现金流折现）
   - PRICE / YIELD / DURATION / MDURATION（债券定价）
   - COUPNUM / COUPDAYS / COUPDAYBS / COUPDAYSNC（付息周期）
   - PMT / IPMT / PPMT / FV / PV / RATE / NPER（贷款计算）
2. `functions/statistical.rs`：
   - NORM.DIST / NORM.INV / NORM.S.DIST / NORM.S.INV
   - T.DIST / T.INV / T.TEST
   - LINEST / LOGEST / TREND / GROWTH（线性回归）
   - CONFIDENCE.NORM / CONFIDENCE.T
   - CHISQ.DIST / CHISQ.TEST
   - 基础统计：STDEV.P / STDEV.S / VAR.P / VAR.S / MEDIAN / MODE / QUARTILE / PERCENTILE

**验证**：使用 `statrs` 或 `argmin` crate 辅助数值验证。

### A.5：与写入流水线集成

**范围**：将公式求值引擎集成到 excel-core 的写入流水线。

**实现内容**：

1. `excel-core` 新增依赖 `excel-formula`
2. 在 `excel-core/src/features/formula_ops.rs` 的 `set_formula` 函数中：
   - 写入公式后自动调用 `FormulaEngine::evaluate`
   - 若求值成功，将结果写入目标单元格
   - 若为溢出结果，扩展写入区域
   - 若求值失败（如循环引用），保留公式字符串并返回错误提示
3. 在 `excel-core/src/features/formula_ops.rs` 的 `refresh_formulas` 函数中：
   - 遍历所有含公式的单元格，重新求值并更新
4. 新增函数 `evaluate_formula` 作为独立公开 API

**验证**：写入公式后读取同一单元格，确认值为求值结果。

### A.6：CLI / HTTP / MCP 入口

**范围**：为公式求值引擎添加对外接口。

**实现内容**：

1. **CLI**：`crates/excel-cli/src/` 新增子命令
   - `excel formula-eval --file a.xlsx --sheet Sheet1 --cell A1 --formula "=SUM(B1:B10)"`
   - `excel formula-eval-batch --file a.xlsx --formulas formulas.json`
   - 可选参数 `--no-eval` 跳过求值，仅写入公式字符串
2. **HTTP**：`crates/excel-http/src/http/handlers/formula_ops.rs` 新增端点
   - `POST /api/formula/evaluate` -- 对单个单元格求值
   - `POST /api/formula/evaluate-batch` -- 批量求值
3. **MCP**：`crates/excel-mcp/src/tools/formula.rs` 新增工具
   - `excel_formula_evaluate` -- 写入公式并求值
   - `excel_formula_evaluate_batch` -- 批量写入公式并求值
   - 修改已有 `excel_formula_set` 添加 `evaluate` 参数（默认 true）

**验证**：各入口端到端测试。

---

## 阶段 B：切片器 (Slicer)

### 背景

当前已完整支持数据透视表创建（12 种聚合、9 种 showAs、3 种布局），但缺少切片器，无法实现透视表之间的交互式联动筛选。

### 目标

支持数据透视表切片器的创建与配置，实现多透视表联动切片。

### B.1：类型定义

**范围**：在 `excel-types` 中定义切片器相关类型。

**文件**：`crates/excel-types/src/slicer.rs`

**实现内容**：

```rust
pub struct SlicerConfig {
    pub name: String,                          // 切片器名称
    pub pivot_table_name: String,              // 关联的透视表
    pub field_column: u16,                     // 切片字段 (0-based column index)
    pub target_sheet: String,                  // 放置切片器的工作表
    pub position: SlicerPosition,              // 切片器位置
    pub style: Option<String>,                 // SlicerStyleLight1-6 / SlicerStyleDark1-6
    pub columns: Option<u32>,                  // 按钮列数 (默认 1)
    pub show_header: bool,                     // 是否显示标题
    pub linked_pivots: Vec<String>,            // 联动的其他透视表名称
}

pub struct SlicerPosition {
    pub col: u32,                              // 列偏移 (像素)
    pub row: u32,                              // 行偏移 (像素)
    pub width: u32,                            // 宽度 (像素)
    pub height: u32,                           // 高度 (像素)
}
```

### B.2：核心实现

**范围**：在 `excel-core` 中实现切片器创建逻辑。

**文件**：`crates/excel-core/src/features/slicer.rs`（新建）

**实现内容**：

1. 切片器基于透视表字段的唯一值创建过滤选择器
2. 使用 `rust_xlsxwriter` 的 drawing/ole 能力写入切片器 XML 部件
   - 写入 `xl/slicers/slicer1.xml`（切片器定义）
   - 写入 `xl/slicerCaches/slicerCache1.xml`（切片器缓存数据）
   - 修改 `xl/workbook.xml` 注册切片器关系
   - 修改 `xl/worksheets/sheet1.xml` 添加 drawing 引用
3. 多透视表联动：`linked_pivots` 参数指定联动透视表
   - 在切片器缓存中声明多个 pivotCache 关联
   - 确保所有关联透视表使用同一数据源
4. 修改 `excel-core/src/features/mod.rs` 添加 `pub mod slicer`
5. 修改 `excel-core/src/excel_write/mod.rs` 导出 `create_slicer`

**注意**：`rust_xlsxwriter` 对切片器的原生支持有限，需在 `modify_file_with_wb` 之后对原始 XML 做后处理注入切片器部件。

### B.3：CLI / HTTP / MCP 入口

**实现内容**：

1. **CLI**：`excel slicer-create --file a.xlsx --config slicer.json`
2. **HTTP**：`POST /api/slicer/create` -- 创建切片器
3. **MCP**：新增 `mcp/src/tools/slicer.rs`
   - `excel_slicer_create` -- 创建切片器

**验证**：端到端测试 -- 创建透视表 + 切片器，在 Excel 客户端验证。

---

## 阶段 C：计算字段 (Calculated Field)

### 背景

当前透视表已完整支持数据字段聚合，但缺少计算字段能力，无法在透视表中基于现有字段定义派生字段。

### 目标

支持在数据透视表中定义计算字段，基于现有字段的算术表达式创建新的聚合列。

### C.1：类型定义

**范围**：在 `excel-types` 的 `pivot_table.rs` 中扩展类型。

**实现内容**：

```rust
/// A calculated field in a pivot table.
pub struct PivotCalculatedField {
    /// Display name for the calculated field
    pub name: String,
    /// Formula expression using existing field names
    /// e.g. "=Revenue - Cost", "=Price * Quantity"
    pub formula: String,
}

// 在 PivotTableConfig 中新增字段：
#[serde(default)]
pub calculated_fields: Vec<PivotCalculatedField>,
```

### C.2：核心实现

**范围**：在 `excel-core` 透视表模块中实现计算字段逻辑。

**文件**：`crates/excel-core/src/features/pivot_table.rs`（修改）

**实现内容**：

1. 在 `create_pivot_table` 的数据预处理阶段：
   - 解析 `calculated_fields` 中的 formula 表达式
   - 识别表达式中引用的源字段名（按 column header 匹配）
   - 为每个原始数据行计算派生值
   - 将计算结果作为新的虚拟列参与聚合
2. 表达式解析（复用 `excel-formula` 的 `parser.rs` 或实现轻量解析器）：
   - 支持四则运算（`+` `-` `*` `/`）
   - 支持括号嵌套
   - 支持字段名作为操作数（按 header 查找列值）
   - 不支持更复杂的函数（KISS 原则，计算字段的核心场景是简单算术）
3. 在 `build_pivot_data` 中：
   - 计算字段作为额外的 `PivotDataField`（aggregation 固定为 Sum）
   - 名称冲突检测：不允许与已有字段同名

**验证**：单元测试 -- 计算 `Revenue - Cost` 结果正确。

### C.3：CLI / HTTP / MCP 入口

**实现内容**：

1. **CLI**：`excel pivot-table-create` 的 config JSON 中直接包含 `calculated_fields`（无需新增子命令）
2. **HTTP**：`POST /api/pivot-table/create` 的 config 中直接包含 `calculated_fields`
3. **MCP**：`excel_pivot_table_create` 工具 config 参数中直接包含 `calculated_fields`

**验证**：端到端测试 -- 创建含计算字段的透视表，验证数据正确。

---

## 验证与回归策略

每个阶段完成后执行：

1. `cargo build --workspace` -- 确保编译通过
2. `cargo test --workspace` -- 确保已有测试不退化
3. `cargo clippy --workspace -- -D warnings` -- 代码质量检查
4. 手工端到端验证（使用 Excel 客户端打开生成的文件）

---

## 风险与注意事项

| 风险 | 缓解措施 |
|------|---------|
| `rust_xlsxwriter` 对切片器 XML 支持有限 | 在 `modify_file_with_wb` 后对原始 zip 做 XML 后处理注入；参考 OOXML 规范手工构建切片器 XML |
| 公式求值引擎精度与 Excel 不一致 | 使用 `f64` 并遵循 IEEE 754；对金融函数使用 BigDecimal 或高精度库；关键函数按 Excel 行为做模糊匹配测试 |
| 循环引用导致求值死循环 | 在 `FormulaEngine` 中维护求值栈，检测到重复访问同一单元格时立即终止并返回 `#CIRCULAR!` 错误 |
| 计算字段与公式求值引擎耦合 | 计算字段使用独立轻量解析器，不依赖 `excel-formula` crate，避免阶段性耦合 |
