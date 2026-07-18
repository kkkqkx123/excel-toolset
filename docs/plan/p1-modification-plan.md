# P1 功能补充 -- 分阶段修改方案

基于 `excel-toolset-vs-officecli-analysis.md` 分析结论，P1 优先级包含六个任务：自动筛选、图像与形状插入、工作表可见性控制、冻结窗格、工作表保护、打印设置。本方案将其拆分为三个独立阶段，每阶段内部按 `类型定义 -> 核心逻辑 -> CLI/HTTP/MCP 入口 -> 测试验证` 的顺序推进。

---

## 总览

| 阶段 | 任务 | 预估工作量 | 依赖 |
|------|------|-----------|------|
| P1.1 | 工作表可见性控制 + 冻结窗格 | 25% | 无 |
| P1.2 | 自动筛选 + 工作表保护 | 35% | 无 |
| P1.3 | 打印设置 + 图像与形状插入 | 40% | 无 |

三个阶段之间无强依赖，可按任意顺序实施。

每个阶段内部均覆盖四个层次：excel-types 类型定义 -> excel-core 核心逻辑 -> CLI/HTTP/MCP 三入口 -> 测试验证。

---

## 阶段 P1.1：工作表可见性控制 + 冻结窗格

### 背景

当前工作表操作仅支持基础 CRUD（增删改查），缺少可见性控制（visible / hidden / veryHidden）和冻结窗格能力。rust_xlsxwriter 对这两个特性均有原生 API 支持（`Worksheet::set_hidden()` 和 `Worksheet::set_freeze_panes()`），实现成本低。

### 功能设计

#### 1.1.1 工作表可见性控制

**目标**：支持三级工作表可见性（visible / hidden / veryHidden）。

**类型定义** (`excel-types/src/sheet_visibility.rs`，新建)：

```rust
/// 工作表可见性级别
pub enum SheetVisibility {
    Visible,    // 正常可见
    Hidden,     // 隐藏（用户可通过 Excel UI 取消隐藏）
    VeryHidden, // 深度隐藏（仅能通过 VBA 取消隐藏）
}

/// 设置工作表可见性的请求
pub struct SheetVisibilityRequest {
    pub sheet: String,
    pub visibility: SheetVisibility,
}
```

**核心逻辑** (`excel-core/src/features/sheet_visibility.rs`，新建)：

- `set_sheet_visibility(path, sheet, visibility, params) -> WriteResult`：
  使用 `modify_file_with_wb` 模式，找到目标工作表后调用 `ws.set_hidden(visibility)`。
  `VeryHidden` 需要通过 `ws.set_hidden(2)` 设置（rust_xlsxwriter API 中 0=Visible, 1=Hidden, 2=VeryHidden）。

**入口变更**：

- **CLI**：`Commands` 枚举新增 `SheetVisibility(SheetVisibilityArgs)`，`SheetSub` 新增 `SetVisibility` 子命令
- **HTTP**：新增 `POST /api/sheet/visibility`，复用 `sheet` handler
- **MCP**：新增 `excel_sheet_set_visibility` 工具

#### 1.1.2 冻结窗格

**目标**：支持行冻结、列冻结、行列同时冻结。

**类型定义** (`excel-types/src/freeze_panes.rs`，新建)：

```rust
/// 冻结窗格配置
pub struct FreezePanesConfig {
    pub sheet: String,
    /// 冻结行数（从顶部算起，0 表示不冻结行）
    pub rows: u32,
    /// 冻结列数（从左侧算起，0 表示不冻结列）
    pub cols: u16,
}
```

**核心逻辑** (`excel-core/src/features/freeze_panes.rs`，新建)：

- `set_freeze_panes(path, config, params) -> WriteResult`：
  使用 `modify_file_with_wb` 模式，找到目标工作表后调用 `ws.set_freeze_panes(rows, cols)`。
- `clear_freeze_panes(path, sheet, params) -> WriteResult`：
  清除冻结窗格，调用 `ws.set_freeze_panes(0, 0)`。

**入口变更**：

- **CLI**：新增 `FreezePane(FreezePaneArgs)` 顶级命令，含 `Set` 和 `Clear` 子命令
- **HTTP**：新增 `POST /api/freeze-panes/set` 和 `POST /api/freeze-panes/clear`
- **MCP**：新增 `excel_freeze_panes_set` 和 `excel_freeze_panes_clear` 工具

### P1.1 文件变更清单

| 文件 | 操作 | 说明 |
|------|------|------|
| `crates/excel-types/src/sheet_visibility.rs` | 新建 | SheetVisibility 枚举和请求类型 |
| `crates/excel-types/src/freeze_panes.rs` | 新建 | FreezePanesConfig 类型 |
| `crates/excel-types/src/lib.rs` | 修改 | 新增两个模块声明和 re-export |
| `crates/excel-core/src/features/sheet_visibility.rs` | 新建 | 可见性设置核心逻辑 |
| `crates/excel-core/src/features/freeze_panes.rs` | 新建 | 冻结窗格核心逻辑 |
| `crates/excel-core/src/features/mod.rs` | 修改 | 新增两个模块声明 |
| `crates/excel-core/src/excel_write/mod.rs` | 修改 | 导出两个新函数 |
| `crates/excel-cli/src/cli/args.rs` | 修改 | SheetVisibilityArgs、FreezePaneArgs |
| `crates/excel-cli/src/cli/runners.rs` | 修改 | 新增 runner 匹配分支 |
| `crates/excel-http/src/http/router.rs` | 修改 | 注册新路由 |
| `crates/excel-http/src/http/handlers/sheet.rs` | 修改 | 新增 visibility handler |
| `crates/excel-http/src/http/handlers/freeze_panes.rs` | 新建 | 冻结窗格 handler |
| `crates/excel-http/src/http/handlers/mod.rs` | 修改 | 声明 freeze_panes 模块 |
| `crates/excel-mcp/src/tools/sheet.rs` | 修改 | 新增 visibility 工具 |
| `crates/excel-mcp/src/tools/freeze_panes.rs` | 新建 | 冻结窗格工具 |
| `crates/excel-mcp/src/tools.rs` | 修改 | 声明 freeze_panes 模块并注册 |

### P1.1 验证

- `cargo build --workspace` 编译通过
- `cargo test --workspace` 已有测试不退化
- `cargo clippy --workspace -- -D warnings` 无警告
- 手工验证：创建 Excel 文件，设置某个工作表为 hidden 后确认在 Excel 中不可见；设置冻结窗格后确认滚动效果

---

## 阶段 P1.2：自动筛选 + 工作表保护

### 背景

自动筛选（AutoFilter）是 Excel 的列头下拉筛选功能，当前项目仅实现了数据级内存过滤（FilterCondition/FilterOp），未使用 Excel 原生的 AutoFilter 特性。工作表保护允许设置密码保护工作簿/工作表，防止未授权的结构修改。

rust_xlsxwriter 对这两个特性均有原生支持（`Worksheet::set_autofilter()` 和 `Worksheet::protect()` / `Worksheet::unprotect()`）。

### 功能设计

#### 1.2.1 自动筛选

**目标**：为指定区域添加 Excel 原生自动筛选功能（列头下拉箭头），支持读取已有筛选状态。

**类型定义** (`excel-types/src/auto_filter.rs`，新建)：

```rust
/// 自动筛选配置（Excel 原生 AutoFilter）
pub struct AutoFilterConfig {
    pub sheet: String,
    /// 自动筛选应用范围，如 "A1:D100"（包含表头行）
    pub range: String,
}

/// 自动筛选状态信息（读取用）
pub struct AutoFilterInfo {
    pub sheet: String,
    pub range: Option<String>,
    pub enabled: bool,
}
```

**核心逻辑** (`excel-core/src/features/auto_filter.rs`，新建)：

- `set_auto_filter(path, config, params) -> WriteResult`：
  使用 `modify_file_with_wb` 模式，解析 range 定位起始行/列和结束行/列，调用 `ws.set_autofilter(first_row, first_col, last_row, last_col)`。
- `remove_auto_filter(path, sheet, params) -> WriteResult`：
  清除自动筛选。由于 rust_xlsxwriter 没有直接的 "remove autofilter" API，通过在重新构建 worksheet 时不调用 `set_autofilter()` 来实现（默认无 AutoFilter）。
- `get_auto_filter(path, sheet) -> Result<AutoFilterInfo>`：
  读取工作表的 AutoFilter 状态。使用 calamine 读取工作表结构，检测是否存在 AutoFilter 定义。

**入口变更**：

- **CLI**：新增 `AutoFilter(AutoFilterArgs)`，含 `Set`、`Remove`、`Get` 子命令
- **HTTP**：新增 `POST /api/auto-filter/set`、`POST /api/auto-filter/remove`、`POST /api/auto-filter/get`
- **MCP**：新增 `excel_auto_filter_set`、`excel_auto_filter_remove`、`excel_auto_filter_get` 工具

#### 1.2.2 工作表保护

**目标**：支持工作表级别和工作簿级别的密码保护，支持单元格锁定和公式隐藏选项。

**类型定义** (`excel-types/src/sheet_protection.rs`，新建)：

```rust
/// 工作表保护配置
pub struct SheetProtectionConfig {
    pub sheet: String,
    /// 密码（可选，不设置则为无密码保护）
    pub password: Option<String>,
    /// 保护选项
    pub options: ProtectionOptions,
}

/// 保护选项（细分控制允许的操作）
pub struct ProtectionOptions {
    /// 允许选择锁定的单元格
    pub select_locked_cells: bool,
    /// 允许选择未锁定的单元格
    pub select_unlocked_cells: bool,
    /// 允许格式化单元格
    pub format_cells: bool,
    /// 允许格式化列
    pub format_columns: bool,
    /// 允许格式化行
    pub format_rows: bool,
    /// 允许插入行
    pub insert_rows: bool,
    /// 允许插入列
    pub insert_columns: bool,
    /// 允许删除行
    pub delete_rows: bool,
    /// 允许删除列
    pub delete_columns: bool,
    /// 允许排序
    pub sort: bool,
    /// 允许使用自动筛选
    pub auto_filter: bool,
    /// 允许使用数据透视表
    pub pivot_tables: bool,
}
```

**核心逻辑** (`excel-core/src/features/sheet_protection.rs`，新建)：

- `protect_sheet(path, config, params) -> WriteResult`：
  使用 `modify_file_with_wb` 模式，调用 `ws.protect()` 设置保护。遍历 `ProtectionOptions` 的字段，按位设置保护选项掩码传递给 rust_xlsxwriter。

- `unprotect_sheet(path, sheet, params) -> WriteResult`：
  调用 `ws.unprotect()` 取消保护。

- `is_sheet_protected(path, sheet) -> Result<bool>`：
  使用 calamine 读取工作表结构，检测是否存在 sheetProtection 元素。

**入口变更**：

- **CLI**：新增 `Protection(ProtectionArgs)`，含 `Protect` 和 `Unprotect` 子命令
  - `Protect`：通过 `--config` 参数传入 JSON ProtectionConfig
- **HTTP**：新增 `POST /api/protection/sheet/protect`、`POST /api/protection/sheet/unprotect`、`POST /api/protection/sheet/is-protected`
- **MCP**：新增 `excel_sheet_protect`、`excel_sheet_unprotect`、`excel_sheet_is_protected` 工具

### P1.2 文件变更清单

| 文件 | 操作 | 说明 |
|------|------|------|
| `crates/excel-types/src/auto_filter.rs` | 新建 | AutoFilterConfig、AutoFilterInfo 类型 |
| `crates/excel-types/src/sheet_protection.rs` | 新建 | SheetProtectionConfig、ProtectionOptions 类型 |
| `crates/excel-types/src/lib.rs` | 修改 | 新增两个模块声明和 re-export |
| `crates/excel-core/src/features/auto_filter.rs` | 新建 | 自动筛选核心逻辑 |
| `crates/excel-core/src/features/sheet_protection.rs` | 新建 | 工作表保护核心逻辑 |
| `crates/excel-core/src/features/mod.rs` | 修改 | 新增两个模块声明 |
| `crates/excel-core/src/excel_write/mod.rs` | 修改 | 导出新函数 |
| `crates/excel-cli/src/cli/args.rs` | 修改 | AutoFilterArgs、ProtectionArgs |
| `crates/excel-cli/src/cli/runners.rs` | 修改 | 新增 runner 匹配分支 |
| `crates/excel-http/src/http/router.rs` | 修改 | 注册新路由 |
| `crates/excel-http/src/http/handlers/auto_filter.rs` | 新建 | 自动筛选 handler |
| `crates/excel-http/src/http/handlers/sheet_protection.rs` | 新建 | 工作表保护 handler |
| `crates/excel-http/src/http/handlers/mod.rs` | 修改 | 声明新模块 |
| `crates/excel-mcp/src/tools/auto_filter.rs` | 新建 | 自动筛选工具 |
| `crates/excel-mcp/src/tools/sheet_protection.rs` | 新建 | 工作表保护工具 |
| `crates/excel-mcp/src/tools.rs` | 修改 | 声明并注册新模块 |

### P1.2 验证

- `cargo build --workspace` 编译通过
- `cargo test --workspace` 已有测试不退化
- `cargo clippy --workspace -- -D warnings` 无警告
- 手工验证：为 Excel 文件添加自动筛选后，在 Excel 中确认表头出现下拉箭头且能正常筛选；设置工作表保护后，确认无法修改受保护的内容

---

## 阶段 P1.3：打印设置 + 图像与形状插入

### 背景

打印设置涉及工作表页边距、页面方向、打印区域、打印标题行/列、缩放比例等多个属性。图像与形状插入是 P1 中实现复杂度最高的功能，涉及二进制数据读取、图像格式检测、定位计算等。

rust_xlsxwriter 对打印设置有完整 API 支持（`set_print_area()`、`set_margins()`、`set_page_orientation()` 等），对图像插入也提供 `Worksheet::insert_image()` API。

### 功能设计

#### 1.3.1 打印设置

**目标**：支持页边距、页面方向、纸张大小、打印区域、打印标题行/列、缩放比例、分页符等配置。

**类型定义** (`excel-types/src/page_setup.rs`，新建)：

```rust
/// 页面设置配置
pub struct PageSetupConfig {
    pub sheet: String,
    /// 纸张大小，如 A4、Letter、Legal（默认 A4）
    pub paper_size: Option<PaperSize>,
    /// 页面方向
    pub orientation: Option<PageOrientation>,
    /// 页边距（英寸）
    pub margins: Option<PageMargins>,
    /// 打印区域，如 "A1:G50"
    pub print_area: Option<String>,
    /// 打印标题行（每页顶部重复的行范围，如 "1:3"）
    pub print_title_rows: Option<String>,
    /// 打印标题列（每页左侧重复的列范围，如 "A:B"）
    pub print_title_cols: Option<String>,
    /// 缩放：适配页面数 (width_pages, height_pages)
    pub fit_to_pages: Option<(u16, u16)>,
    /// 缩放百分比（100 = 100%）
    pub scale: Option<u16>,
    /// 网格线打印
    pub print_gridlines: bool,
    /// 行号列标打印
    pub print_headings: bool,
    /// 水平居中
    pub center_horizontally: bool,
    /// 垂直居中
    pub center_vertically: bool,
}

pub enum PaperSize {
    A4,
    A3,
    Letter,
    Legal,
    // ... 其他常用纸张
}

pub enum PageOrientation {
    Portrait,
    Landscape,
}

pub struct PageMargins {
    pub left: f64,
    pub right: f64,
    pub top: f64,
    pub bottom: f64,
    pub header: f64,
    pub footer: f64,
}

/// 分页符设置
pub struct PageBreakConfig {
    pub sheet: String,
    /// 水平分页符所在的行号列表
    pub horizontal_breaks: Vec<u32>,
    /// 垂直分页符所在的列号列表
    pub vertical_breaks: Vec<u16>,
}
```

**核心逻辑** (`excel-core/src/features/page_setup.rs`，新建)：

- `configure_page_setup(path, config, params) -> WriteResult`：
  使用 `modify_file_with_wb` 模式，在重建 worksheet 后，依次应用 `config` 中的各项设置：

  1. 页面方向：`ws.set_portrait()` / `ws.set_landscape()`
  2. 纸张大小：`ws.set_paper_size(PaperSize::A4 as u8)`
  3. 页边距：`ws.set_margins(Margins { ... })`
  4. 打印区域：`ws.set_print_area(range)`
  5. 打印标题行/列：`ws.set_repeat_rows(first_row, last_row)` / `ws.set_repeat_columns(first_col, last_col)`
  6. 缩放：`ws.set_print_fit_to_pages(width, height)` 或 `ws.set_print_scale(scale)`
  7. 网格线/行号/居中：通过对应的 `set_print_*` 方法

- `set_page_breaks(path, config, params) -> WriteResult`：
  设置水平/垂直分页符。rust_xlsxwriter 提供 `ws.set_horizontal_page_breaks(&[row])` 和 `ws.set_vertical_page_breaks(&[col])`。

- `clear_page_breaks(path, sheet, params) -> WriteResult`：
  清除分页符。

**入口变更**：

- **CLI**：新增 `PageSetup(PageSetupArgs)`，含 `Set` 子命令（`--config` JSON）
- **HTTP**：新增 `POST /api/page-setup/configure`、`POST /api/page-setup/page-breaks/set`、`POST /api/page-setup/page-breaks/clear`
- **MCP**：新增 `excel_page_setup_configure`、`excel_page_setup_page_breaks_set`、`excel_page_setup_page_breaks_clear` 工具

#### 1.3.2 图像与形状插入

**目标**：支持 PNG / JPG / GIF / SVG 图像嵌入到指定工作表的指定位置。

**类型定义** (`excel-types/src/image.rs`，新建)：

```rust
/// 图像配置
pub struct ImageConfig {
    pub sheet: String,
    /// 图像文件路径
    pub image_path: String,
    /// 插入位置（左上角锚定单元格），如 "B2"
    pub anchor_cell: String,
    /// 图像缩放选项
    pub scale: Option<ImageScale>,
    /// 偏移量（像素），相对于锚定单元格左上角
    pub x_offset: Option<u32>,
    pub y_offset: Option<u32>,
    /// 替代文本
    pub alt_text: Option<String>,
}

pub struct ImageScale {
    /// X 方向缩放比例（1.0 = 原始大小）
    pub x_scale: f64,
    /// Y 方向缩放比例（1.0 = 原始大小）
    pub y_scale: f64,
}

/// 形状配置
pub struct ShapeConfig {
    pub sheet: String,
    /// 形状类型
    pub shape_type: ShapeType,
    /// 锚定单元格，如 "B2"
    pub anchor_cell: String,
    /// 宽度（像素）
    pub width: u32,
    /// 高度（像素）
    pub height: u32,
    /// 填充颜色（十六进制，如 "FF0000"）
    pub fill_color: Option<String>,
    /// 线条颜色
    pub line_color: Option<String>,
    /// 线条宽度
    pub line_width: Option<f64>,
    /// 替代文本
    pub alt_text: Option<String>,
}

pub enum ShapeType {
    Rectangle,
    RoundedRectangle,
    Ellipse,
    Line,
    TextBox, // 含文本的矩形
}

/// 文本框配置（继承自 ShapeConfig，额外包含文本内容）
pub struct TextBoxConfig {
    pub base: ShapeConfig,
    pub text: String,
    pub font_size: Option<f64>,
    pub font_color: Option<String>,
}
```

**核心逻辑** (`excel-core/src/features/image.rs`，新建)：

- `insert_image(path, config, params) -> WriteResult`：
  使用 `modify_file_with_wb` 模式：

  1. 解析 `anchor_cell` 为 (row, col)
  2. 读取图像文件，创建 `rust_xlsxwriter::Image::new(image_path)`
  3. 设置缩放：`image.set_scale_width(x_scale).set_scale_height(y_scale)`
  4. 插入到工作表：`ws.insert_image(row, col, &image)`
  5. 设置偏移和替代文本

- `remove_image(path, sheet, anchor_cell, params) -> WriteResult`：
  由于 rust_xlsxwriter 不支持删除已插入的图像，在 `modify_file_with_wb` 重建工作簿时跳过该位置的 image 插入。

- `insert_shape(path, config, params) -> WriteResult`：
  rust_xlsxwriter 对原生形状支持有限（主要提供 `insert_image` 用于图片）。
  形状插入采用两步策略：
  1. 使用 `rust_xlsxwriter` 的 `insert_image()` 插入预渲染的形状图像（SVG 转 PNG）
  2. 若需要原生 OOXML 形状（支持后续编辑），在 `modify_file_with_wb` 之后做 XML 后处理，注入 drawing 部件

- `insert_textbox(path, config, params) -> WriteResult`：
  实现方式同 `insert_shape`，文本作为形状的附加属性注入。

**入口变更**：

- **CLI**：新增 `Image(ImageArgs)`，含 `Insert` 和 `Remove` 子命令
- **HTTP**：新增 `POST /api/image/insert`、`POST /api/image/remove`
  - 图像上传：HTTP 接口接受 base64 编码的图像数据或文件路径
- **MCP**：新增 `excel_image_insert`、`excel_image_remove`、`excel_shape_insert` 工具

### P1.3 文件变更清单

| 文件 | 操作 | 说明 |
|------|------|------|
| `crates/excel-types/src/page_setup.rs` | 新建 | PageSetupConfig、PaperSize 等类型 |
| `crates/excel-types/src/image.rs` | 新建 | ImageConfig、ShapeConfig 等类型 |
| `crates/excel-types/src/lib.rs` | 修改 | 新增两个模块声明和 re-export |
| `crates/excel-core/src/features/page_setup.rs` | 新建 | 打印设置核心逻辑 |
| `crates/excel-core/src/features/image.rs` | 新建 | 图像与形状插入核心逻辑 |
| `crates/excel-core/src/features/mod.rs` | 修改 | 新增两个模块声明 |
| `crates/excel-core/src/excel_write/mod.rs` | 修改 | 导出新函数 |
| `crates/excel-cli/src/cli/args.rs` | 修改 | PageSetupArgs、ImageArgs |
| `crates/excel-cli/src/cli/runners.rs` | 修改 | 新增 runner 匹配分支 |
| `crates/excel-http/src/http/router.rs` | 修改 | 注册新路由 |
| `crates/excel-http/src/http/handlers/page_setup.rs` | 新建 | 打印设置 handler |
| `crates/excel-http/src/http/handlers/image.rs` | 新建 | 图像插入 handler |
| `crates/excel-http/src/http/handlers/mod.rs` | 修改 | 声明新模块 |
| `crates/excel-mcp/src/tools/page_setup.rs` | 新建 | 打印设置工具 |
| `crates/excel-mcp/src/tools/image.rs` | 新建 | 图像插入工具 |
| `crates/excel-mcp/src/tools.rs` | 修改 | 声明并注册新模块 |

### P1.3 验证

- `cargo build --workspace` 编译通过
- `cargo test --workspace` 已有测试不退化
- `cargo clippy --workspace -- -D warnings` 无警告
- 手工验证：
  - 配置打印设置后，在 Excel 打印预览中确认页边距、方向、标题行生效
  - 插入 PNG 图像后，在 Excel 中确认图像位置和缩放正确
  - 插入形状/文本框后，确认可见且尺寸/颜色正确

---

## 统一设计规则

以下规则适用于三个阶段的所有新功能：

### 类型定义规范

- 所有公开类型定义在 `excel-types/src/` 下独立文件中
- 使用 `#[derive(Debug, Clone, Serialize, Deserialize)]` 标注
- 字段使用 `#[serde(default)]` 和 `#[serde(skip_serializing_if = "Option::is_none")]` 控制序列化
- 所有 enum 支持 `serde` 的 snake_case 反序列化

### 核心逻辑规范

- 遵循 `read -> new Workbook -> write -> overwrite` 模式
- 所有写操作前缀安全检查（fingerprint -> backup -> dry-run check -> execute）
- 返回 `Result<WriteResult>`，错误使用 `AppError` 枚举
- 使用 `info_span!/warn_span!` 进行结构化日志记录
- 不使用 `unwrap()`，测试中用 `expect()` 替代

### 入口注册规范

- **CLI**：在 `args.rs` 的 `Commands` 枚举中新增变体，定义对应的 Args/Sub 结构体，`runners.rs` 中添加命令分发
- **HTTP**：在 `handlers/` 下新建文件，定义 Request struct（derive Deserialize），handler 函数（async fn），在 `router.rs` 中注册路由
- **MCP**：在 `tools/` 下新建文件，实现 `tools()` 和 `register()`，在 `tools.rs` 中注册

### 安全与错误处理

- `SecurityParams` 统一管理 dry_run / create_backup / file_path
- 参数校验放在各 handler 中，核心逻辑函数假设参数已校验
- 使用 `#[serde(default)]` 处理可选字段，避免反序列化时缺少字段报错
- 所有网络/文件 I/O 错误通过 `AppError` 转换后传播

---

## 验证与回归策略

每个阶段完成后执行：

1. `cargo build --workspace` -- 确保编译通过，无 dead code 警告
2. `cargo test --workspace` -- 确保已有测试不退化，新增测试全部通过
3. `cargo clippy --workspace -- -D warnings` -- 代码质量检查
4. `cargo fmt --check` -- 代码格式检查
5. 手工端到端验证（使用 Excel 客户端打开生成的文件）

---

## 风险与注意事项

| 风险 | 影响功能 | 缓解措施 |
|------|---------|---------|
| rust_xlsxwriter 对 VeryHidden 的支持 | 工作表可见性 | 若 rust_xlsxwriter 的 `set_hidden()` 不支持 VeryHidden 值 2，则通过 XML 后处理注入 `state="veryHidden"` 属性 |
| rust_xlsxwriter 不支持删除 AutoFilter | 自动筛选 | 重新构建工作表时不调用 `set_autofilter()` 即可；对于读取，使用 calamine 检测 AutoFilter 状态 |
| 形状插入需 XML 后处理 | 图像与形状 | rust_xlsxwriter 仅原生支持图像插入，形状（矩形/椭圆/文本框）需在 `modify_file_with_wb` 后手工注入 drawing XML；参考 `slicer.rs` 中的 XML 后处理模式 |
| 图像文件不存在或格式不支持 | 图像插入 | 在核心逻辑中进行文件存在性和 MIME 类型校验，提前返回明确的错误信息 |
| 密码哈希与 Excel 兼容性 | 工作表保护 | rust_xlsxwriter 的 `protect()` 方法接受明文密码并自动处理哈希，直接传递用户提供的密码即可 |
| fit_to_pages 和 scale 互斥 | 打印设置 | 在 PageSetupConfig 中通过逻辑校验：若 fit_to_pages 和 scale 同时设置，优先使用 fit_to_pages 并打印警告日志 |
