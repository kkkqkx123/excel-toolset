# Excel CLI 使用手册

## 安装与构建

```bash
# 构建（基础功能）
cargo build --package excel-cli --release

# 构建（包含 SQL 查询功能）
cargo build --package excel-cli --release --features sql

# 生成的可执行文件
./target/release/excel-cli
```

## 全局标志

| 标志 | 默认值 | 说明 |
|------|--------|------|
| `-p, --pretty` | `false` | 格式化输出 JSON |
| `--format` | `json` | 输出格式，可选 `json` 或 `text`（仅 diff 命令支持 text） |

```bash
excel-cli --pretty file info data.xlsx
excel-cli --format text diff file old.xlsx new.xlsx
```

## 输出格式

所有命令默认输出 JSON。成功响应：
```json
{"success": true, ...data}
```
错误响应：
```json
{"success": false, "message": "错误信息"}
```

---

## file -- 文件操作

### 创建文件
```bash
excel-cli file create <path> [--sheet <name>]
```
- `path`：Excel 文件路径
- `--sheet`：初始工作表名，默认 `Sheet1`

### 查看文件信息
```bash
excel-cli file info <path>
```
返回工作表数量、名称、文件哈希等元数据。

### 创建备份
```bash
excel-cli file backup <path> [--output <dest>]
```
- `--output`：可选，将备份复制到指定位置

---

## sheet -- 工作表操作

### 列出工作表
```bash
excel-cli sheet list <path>
```

### 添加工作表
```bash
excel-cli sheet add <path> <name>
```

### 删除工作表
```bash
excel-cli sheet delete <path> <name>
```

### 重命名工作表
```bash
excel-cli sheet rename <path> <old-name> <new-name>
```

### 设置可见性
```bash
excel-cli sheet set-visibility <path> <name> --visibility <mode> [--dry-run]
```
- `--visibility`：`visible`（可见）、`hidden`（隐藏）、`very_hidden`（深度隐藏）

---

## cell -- 单元格操作

### 读取单元格
```bash
excel-cli cell read <path> <sheet> <cell>
```
- `cell`：单元格引用，如 `A1`、`B3`

### 写入单元格
```bash
excel-cli cell write <path> <sheet> <cell> <value> [--dry-run]
```
- `value`：写入的值（自动推断类型：数字、布尔、字符串）
- `--dry-run`：模拟执行，不写入文件

---

## range -- 区域操作

### 读取区域
```bash
excel-cli range read <path> <sheet> <range> [--mode <mode>] [--truncate <n>]
```
- `range`：区域引用，如 `A1:C5`
- `--mode`：`detailed`（默认，含行号列号）、`compact`（纯数据矩阵）、`csv`（CSV 格式）
- `--truncate`：可选，限制返回行数

### 写入区域
```bash
excel-cli range write <path> <sheet> <range> <data> [--dry-run]
```
- `data`：JSON 格式的二维数组，如 `[["a","b"],["1","2"]]`

### 从 CSV 写入区域
```bash
excel-cli range write-csv <path> <sheet> <range> <csv-file> [--dry-run]
```

### 清空区域
```bash
excel-cli range clear <path> <sheet> <range> [--dry-run]
```

---

## data -- 数据处理

### 追加行
```bash
excel-cli data append-row <path> <sheet> <val1> <val2> ... [--dry-run]
```

### 插入行
```bash
excel-cli data insert-row <path> <sheet> <row> <val1> <val2> ... [--dry-run]
```
- `row`：目标行号（从 1 开始）

### 删除行
```bash
excel-cli data delete-row <path> <sheet> <row> [--dry-run]
```

### 过滤
```bash
excel-cli data filter <path> <sheet> <column> <op> <value>
```
- `column`：列号（从 1 开始）
- `op`：操作符，如 `eq`、`ne`、`gt`、`lt`、`gte`、`lte`、`contains`
- `value`：比较值

### 排序
```bash
excel-cli data sort <path> <sheet> <column> [--desc] [--dry-run]
```

### 去重
```bash
excel-cli data dedup <path> <sheet> [--column <col>] [--dry-run]
```
- `--column`：按指定列去重，不传则比较整行

### SQL 查询
```bash
excel-cli data sql <path> <sheet> <query> [--session] [--cache]
```
- `query`：SQL 语句，如 `SELECT * FROM t WHERE A > 10`
- `--session`：启用会话模式，支持多次查询共享上下文
- `--cache`：启用查询缓存
- 注意：SQL 功能需要通过 `--features sql` 构建才可用

---

## formula -- 公式操作

### 设置公式
```bash
excel-cli formula set <path> <sheet> <cell> <formula> [--eval] [--dry-run]
```
- `formula`：公式字符串，如 `=SUM(A1:A10)`
- `--eval`：设置后立即求值

### 刷新公式
```bash
excel-cli formula refresh <path> <sheet> [--dry-run]
```

### 读取公式
```bash
excel-cli formula read <path> <sheet> <cell>
```
返回单元格原始公式字符串。

### 设置计算模式
```bash
excel-cli formula calc-mode <path> [--mode <mode>] [--dry-run]
```
- `mode`：`auto`（默认）或 `manual`

### 追踪依赖
```bash
excel-cli formula trace <path> <sheet> <cell>
```
返回单元格的前驱和后继依赖链。

### 解释公式
```bash
excel-cli formula explain <path> <sheet> <cell> [--language <lang>]
```
- `language`：`en`（默认）或 `zh`

### 解释公式逻辑
```bash
excel-cli formula explain-logic <path> <sheet> <cell> [--language <lang>]
```
返回公式逻辑结构的自然语言说明。

### 公式求值
```bash
excel-cli formula eval <path> <sheet> <cell> <formula> [--no-eval] [--dry-run]
```
- `formula`：公式字符串
- `--no-eval`：只设置公式不立即求值

### 批量求值
```bash
excel-cli formula eval-batch <path> <sheet> <formulas> [--dry-run]
```
- `formulas`：JSON 数组，如 `[["A1","=SUM(B1:B5)"],["A2","=AVERAGE(B1:B5)"]]`

### 自动填充公式
```bash
excel-cli formula fill <path> <sheet> <source> <target-range> [--dry-run]
```
- `source`：源单元格引用，如 `A1`
- `target-range`：目标填充区域，如 `A2:A10`

---

## format -- 格式操作

### 设置格式
```bash
excel-cli format set <path> <sheet> <range> <style> [--dry-run]
```
- `style`：JSON 格式样式，如 `{"bold": true, "font_size": 14, "font_color": "#FF0000"}`

### 合并单元格
```bash
excel-cli format merge <path> <sheet> <range> [--value <v>] [--dry-run]
```
- `--value`：可选，合并后单元格的值

---

## chart -- 图表操作

### 创建图表
```bash
excel-cli chart create <path> <sheet> <range> <chart-type> \
  [--title <title>] \
  [--position <cell>] \
  [--dry-run] \
  [--trendline <json>] \
  [--y-error-bars <json>] \
  [--x-error-bars <json>] \
  [--log-base <n>]
```
- `chart-type`：`column`、`bar`、`line`、`pie`、`area`、`scatter`
- `--title`：可选，图表标题
- `--position`：可选，图表放置位置的单元格引用（如 `E5`），默认放在数据区域下方
- `--trendline`：趋势线配置 JSON，如 `'{"trend_type":"linear","display_equation":true}'`
- `--y-error-bars`：Y 轴误差线配置 JSON，如 `'{"error_type":"standard_error","direction":"both"}'`
- `--x-error-bars`：X 轴误差线配置 JSON
- `--log-base`：Y 轴对数刻度底数

---

## vba -- VBA 宏操作

### 导出 VBA
```bash
excel-cli vba export <path> <output-file>
```

### 导入 VBA
```bash
excel-cli vba import <path> <vba-file> [--dry-run]
```

### 检查是否包含 VBA
```bash
excel-cli vba has <path>
```

---

## diff -- 文件对比

### 文件对比
```bash
excel-cli diff file <old-path> <new-path> [--sheet <name>] [--semantic]
```
对比两个 Excel 文件的全部差异，或指定单个工作表。
- `--semantic`：生成语义级差异报告

### 区域对比
```bash
excel-cli diff range <old-path> <new-path> <sheet> <range> [--semantic]
```
- `--semantic`：生成语义级差异报告

### 语义差异
```bash
excel-cli diff semantic <old-path> <new-path>
```
生成两个文件的语义级差异报告，包含结构化差异摘要。

### 公式依赖对比
```bash
excel-cli diff formula-deps <old-path> <new-path> <sheet>
```
对比两个文件的公式依赖图差异，包含循环检测。

### Git Diff 驱动
```bash
# 在当前仓库安装 Git 驱动（默认覆盖 *.xlsx, *.xls, *.xlsm, *.xlsb）
excel-cli diff install-git-driver

# 全局安装（对所有仓库生效）
excel-cli diff install-git-driver --global

# 自定义文件匹配模式
excel-cli diff install-git-driver --patterns '*.xlsx,*.xlsm'

# 全局安装 + 自定义模式
excel-cli diff install-git-driver --global --patterns '*.xlsx,*.xls,*.xlsm,*.xlsb'

# 卸载当前仓库的驱动
excel-cli diff uninstall-git-driver

# 卸载全局驱动
excel-cli diff uninstall-git-driver --global

# 手动触发（由 git diff 自动调用）
excel-cli diff git-driver
```

安装后 Git 会自动在 `.gitattributes` 中添加：
```
*.xlsx diff=excel-diff
*.xls diff=excel-diff
*.xlsm diff=excel-diff
*.xlsb diff=excel-diff
```

全局安装还会设置 `git config --global core.attributesfile`，使所有仓库共享同一份 gitattributes 配置。

也可以通过脚本一键完成全局安装：
```bash
./scripts/install-global.sh
```

---

## batch -- 批量操作

### 批量修改
```bash
excel-cli batch modify <path> --operations '<json>' \
  [--dry-run] \
  [--strategy <strategy>] \
  [--validate-only]
```
- `--operations`：JSON 数组，每个元素包含 `op`（操作类型）和对应参数字段
- `--strategy`：`best-effort`（默认，失败时继续）、`all-or-nothing`（事务式）、`dry-run`（仿真）
- `--validate-only`：仅验证请求不执行

示例：
```bash
excel-cli batch modify data.xlsx --operations '[
  {"op": "write_cell", "sheet": "Sheet1", "row": 1, "col": 1, "value": "Hello"},
  {"op": "add_sheet", "name": "NewSheet"},
  {"op": "set_format", "sheet": "Sheet1", "range": "A1:A5", "style": {"bold": true}}
]'
```

支持的操作类型 (`op`)：
| 操作 | 字段 |
|------|------|
| `write_cell` | `sheet`, `row`, `col`, `value` |
| `write_range` | `sheet`, `range`, `data` |
| `add_sheet` | `name` |
| `delete_sheet` | `name` |
| `rename_sheet` | `old_name`, `new_name` |
| `set_format` | `sheet`, `range`, `style` |
| `merge_cells` | `sheet`, `range` |
| `append_row` | `sheet`, `values` |
| `insert_row` | `sheet`, `row`, `values` |
| `delete_row` | `sheet`, `start_row`, `end_row` |
| `set_formula` | `sheet`, `cell`, `formula` |

### 验证公式引用
```bash
excel-cli batch validate-refs <path> <sheet> <formula>
```
验证公式中的单元格引用是否有效。

---

## rollback -- 回滚

```bash
excel-cli rollback <path> <backup-path>
```
从备份文件恢复原始文件。

---

## comments -- 批注操作

### 获取批注
```bash
excel-cli comments get <path> <sheet> <cell>
```

### 添加批注
```bash
excel-cli comments add <path> <sheet> <cell> <text> [--dry-run]
```

### 更新批注
```bash
excel-cli comments update <path> <sheet> <cell> <text> [--dry-run]
```

### 删除批注
```bash
excel-cli comments delete <path> <sheet> <cell> [--dry-run]
```

---

## named-range -- 命名范围

### 列出命名范围
```bash
excel-cli named-range list <path>
```

### 获取命名范围的值
```bash
excel-cli named-range get <path> <name>
```

### 创建命名范围
```bash
excel-cli named-range create <path> <name> <range> [--sheet <name>] [--dry-run]
```

### 删除命名范围
```bash
excel-cli named-range delete <path> <name> [--dry-run]
```

---

## search -- 搜索

### 全工作簿搜索
```bash
excel-cli search workbook <path> <pattern> \
  [--match-type <type>] \
  [--search-type <type>] \
  [--case-sensitive] \
  [--sheets <s1>,<s2>]
```
- `match-type`：`contains`（默认）、`exact`、`regex`
- `search-type`：`both`（默认）、`value`、`formula`
- `--case-sensitive`：区分大小写
- `--sheets`：限定搜索的工作表列表

### 单表搜索
```bash
excel-cli search sheet <path> <sheet> <pattern> \
  [--match-type <type>] \
  [--search-type <type>] \
  [--case-sensitive]
```

---

## conditional-format -- 条件格式

### 添加条件格式
```bash
excel-cli conditional-format add <path> <sheet> <range> <rule-type> <condition> \
  [--style <json>] \
  [--config <json>] \
  [--dry-run]
```
- `rule-type`：`cell_value`、`formula`、`above_average`、`top10`、`duplicate`、`text_contains`、`date_occurring`，以及可视化类型 `data_bar`、`color_scale`、`icon_set`
- `condition`：条件表达式，如 `>10`
- `--style`：可选，JSON 格式样式
- `--config`：可选，高级配置 JSON（用于 DataBar、ColorScale、IconSet 等可视化类型），如 `'{"fill_color":"#00FF00"}'` 或 `'{"icon_type":"three_arrows"}'`

### 移除条件格式
```bash
excel-cli conditional-format remove <path> <sheet> <range> [--dry-run]
```

---

## table -- 表格

### 创建表格
```bash
excel-cli table create <path> --config <json> [--dry-run]
```
- `--config`：JSON TableConfig，包含 `sheet`、`range`、`name`、`style` 等字段

### 移除表格
```bash
excel-cli table remove <path> <name> [--dry-run]
```

### 列出表格
```bash
excel-cli table list <path>
```

### 获取表格信息
```bash
excel-cli table get <path> <name>
```

---

## data-validation -- 数据验证

### 添加数据验证
```bash
excel-cli data-validation add <path> <sheet> --config <json> [--dry-run]
```
- `--config`：JSON ValidationConfig，包含 `range`、`validation_type`、`criteria` 等字段

### 移除数据验证
```bash
excel-cli data-validation remove <path> <sheet> <range> [--dry-run]
```

---

## pivot-table -- 数据透视表

### 创建数据透视表
```bash
excel-cli pivot-table create <path> --config <json> [--dry-run]
```
- `--config`：JSON PivotTableConfig，包含 `sheet`、`data_range`、`rows`、`columns`、`values`、`filters` 等字段

---

## slicer -- 切片器

### 创建切片器
```bash
excel-cli slicer create <path> --config <json> [--dry-run]
```
- `--config`：JSON SlicerConfig

---

## sparkline -- 迷你图

### 添加迷你图
```bash
excel-cli sparkline add <path> <sheet> <source-range> <target-cell> \
  [--sparkline-type <type>] \
  [--style <n>] \
  [--dry-run]
```
- `source-range`：数据源区域，如 `'Sheet1'!A1:E1`
- `target-cell`：目标单元格，如 `F1`
- `--sparkline-type`：`line`（默认）、`column`、`winlose`
- `--style`：样式编号 (0-35)

### 移除迷你图
```bash
excel-cli sparkline remove <path> <sheet> <target-cell> [--dry-run]
```

---

## overview -- 工作簿概览

```bash
excel-cli overview <path> [--blueprint]
```
- `--blueprint`：输出工作簿蓝图（详细结构信息，包括表结构、公式、命名范围等）

---

## history -- 操作历史

```bash
excel-cli history <path>
```
列出文件的操作历史记录。

---

## freeze-pane -- 冻结窗格

### 设置冻结
```bash
excel-cli freeze-pane set <path> <sheet> [--rows <n>] [--cols <n>] [--dry-run]
```
- `--rows`：从顶部冻结的行数，默认 0
- `--cols`：从左侧冻结的列数，默认 0

### 清除冻结
```bash
excel-cli freeze-pane clear <path> <sheet> [--dry-run]
```

---

## auto-filter -- 自动筛选

### 设置自动筛选
```bash
excel-cli auto-filter set <path> <sheet> <range> [--dry-run]
```
- `range`：筛选范围（含表头行），如 `A1:D100`

### 移除自动筛选
```bash
excel-cli auto-filter remove <path> <sheet> [--dry-run]
```

### 获取筛选信息
```bash
excel-cli auto-filter get <path> <sheet>
```

---

## protection -- 工作表保护

### 保护工作表
```bash
excel-cli protection protect <path> <sheet> [--password <pwd>] [--options <json>] [--dry-run]
```
- `--password`：可选，保护密码
- `--options`：可选，JSON ProtectionOptions 配置

### 解除保护
```bash
excel-cli protection unprotect <path> <sheet> [--dry-run]
```

### 检查保护状态
```bash
excel-cli protection is-protected <path> <sheet>
```

---

## page-setup -- 页面设置

### 配置页面
```bash
excel-cli page-setup configure <path> <sheet> --config <json> [--dry-run]
```
- `--config`：JSON PageSetupConfig（不含 sheet 字段，由位置参数指定），可配置方向、纸张大小、页边距等

### 设置分页
```bash
excel-cli page-setup page-breaks <path> --config <json> [--dry-run]
```
- `--config`：JSON PageBreakConfig（含 sheet 字段）

### 清除分页
```bash
excel-cli page-setup clear-breaks <path> <sheet> [--dry-run]
```

---

## image -- 图片/形状

### 插入图片
```bash
excel-cli image insert <path> --config <json> [--dry-run]
```
- `--config`：JSON ImageConfig，包含 `sheet`、`image_path`、`anchor_cell`、`width`、`height` 等字段

### 移除图片
```bash
excel-cli image remove <path> <sheet> <anchor-cell> [--dry-run]
```
- `anchor-cell`：图片所在的锚定单元格，如 `B2`

### 插入形状
```bash
excel-cli image shape-insert <path> --config <json> [--dry-run]
```
- `--config`：JSON ShapeConfig，包含 `sheet`、`shape_type`（rectangle/ellipse/line）、`anchor_cell`、尺寸等字段
