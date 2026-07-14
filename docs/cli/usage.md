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

## file — 文件操作

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

## sheet — 工作表操作

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

---

## cell — 单元格操作

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

## range — 区域操作

### 读取区域
```bash
excel-cli range read <path> <sheet> <range>
```
- `range`：区域引用，如 `A1:C5`

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

## data — 数据处理

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
excel-cli data sql <path> <sheet> <query>
```
- `query`：SQL 语句，如 `SELECT * FROM t WHERE A > 10`
- 注意：SQL 功能需要通过 `--features sql` 构建才可用

---

## formula — 公式操作

### 设置公式
```bash
excel-cli formula set <path> <sheet> <cell> <formula> [--dry-run]
```
- `formula`：公式字符串，如 `=SUM(A1:A10)`

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

---

## format — 格式操作

### 设置格式
```bash
excel-cli format set <path> <sheet> <range> <style> [--dry-run]
```
- `style`：JSON 格式样式，如 `{"bold": true, "font_size": 14, "font_color": "#FF0000"}`

### 合并单元格
```bash
excel-cli format merge <path> <sheet> <range> [--dry-run]
```

---

## chart — 图表操作

### 创建图表
```bash
excel-cli chart create <path> <sheet> <range> <chart-type> [--title <title>] [--position <cell>] [--dry-run]
```
- `chart-type`：`column`、`bar`、`line`、`pie`、`area`、`scatter`
- `--title`：可选，图表标题
- `--position`：可选，图表放置位置的单元格引用（如 `E5`），默认放在数据区域下方

---

## vba — VBA 宏操作

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

## diff — 文件对比

### 文件对比
```bash
excel-cli diff file <old-path> <new-path> [--sheet <name>]
```
对比两个 Excel 文件的全部差异，或指定单个工作表。

### 区域对比
```bash
excel-cli diff range <old-path> <new-path> <sheet> <range>
```

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

## batch — 批量操作

### 批量修改
```bash
excel-cli batch modify <path> --operations '<json>' [--dry-run]
```
- `operations`：JSON 数组，每个元素包含 `op`（操作类型）和对应参数字段

示例：
```bash
excel-cli batch modify data.xlsx --operations '[
  {"op": "write_cell", "sheet": "Sheet1", "row": 1, "col": 1, "value": "Hello"},
  {"op": "add_sheet", "name": "NewSheet"},
  {"op": "set_format", "sheet": "Sheet1", "range": "A1:A5", "style": {"bold": true}}
]'
```

---

## rollback — 回滚

```bash
excel-cli rollback <path> <backup-path>
```
从备份文件恢复原始文件。

---

## comments — 批注操作

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

## named-range — 命名范围

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

## search — 搜索

### 全文搜索
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

### 单表搜索
```bash
excel-cli search sheet <path> <sheet> <pattern> \
  [--match-type <type>] \
  [--search-type <type>] \
  [--case-sensitive]
```

---

## conditional-format — 条件格式

### 添加条件格式
```bash
excel-cli conditional-format add <path> <sheet> <range> <rule-type> <condition> \
  [--style <json>] [--dry-run]
```
- `rule-type`：`cell_value`、`formula`、`above_average`、`top10`、`duplicate`、`text_contains`、`date_occurring`
- `condition`：条件表达式，如 `>10`
- `--style`：可选，JSON 格式样式

### 移除条件格式
```bash
excel-cli conditional-format remove <path> <sheet> <range> [--dry-run]
```
