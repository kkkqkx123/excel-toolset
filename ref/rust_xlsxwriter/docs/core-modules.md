# 核心模块文档

本文档介绍 rust_xlsxwriter 的核心模块，包括主要的数据结构、类型系统和基础组件。

## 核心模块列表

- [Workbook](#workbook) - 工作簿管理
- [Worksheet](#worksheet) - 工作表操作
- [Format](#format) - 格式系统
- [ExcelDateTime](#exceldatetime) - 日期时间处理
- [Formula](#formula) - 公式处理
- [Url](#url) - 超链接处理
- [Error](#error) - 错误处理
- [Color](#color) - 颜色系统
- [Properties](#properties) - 属性定义
- [Protection](#protection) - 保护设置

## Workbook

### 功能

`Workbook` 是 rust_xlsxwriter 的入口点，代表一个完整的 Excel 文档。

### 主要职责

- 管理工作簿级别的属性
- 创建和管理工作表
- 协调文档保存过程
- 管理格式和样式

### 核心方法

```rust
// 创建新工作簿
let mut workbook = Workbook::new();

// 添加工作表
let worksheet = workbook.add_worksheet();

// 保存到文件
workbook.save("output.xlsx")?;

// 保存到内存缓冲区
let buffer = workbook.save_to_buffer()?;

// 设置文档属性
workbook.set_properties(properties);

// 添加自定义属性
workbook.add_custom_property("Author", "John Doe");

// 设置默认格式
workbook.set_default_format(&format);
```

### 内部结构

- `worksheets` - 工作表集合
- `formats` - 格式集合
- `defined_names` - 定义名称
- `custom_properties` - 自定义属性
- `doc_properties` - 文档属性
- `charts` - 图表集合

### 常量内存模式

当启用 `constant_memory` feature 时，支持临时文件模式：

```rust
workbook.set_tempdir("/tmp");
```

## Worksheet

### 功能

`Worksheet` 代表 Excel 中的单个工作表，是数据写入的主要接口。

### 主要职责

- 管理单元格数据
- 应用格式和样式
- 处理公式和函数
- 管理工作表级别设置

### 核心写入方法

```rust
// 基本写入
worksheet.write(0, 0, "Hello")?;
worksheet.write(1, 0, 42)?;
worksheet.write(2, 0, 3.14)?;

// 带格式写入
worksheet.write_with_format(0, 0, "Bold", &bold_format)?;

// 字符串快捷方法
worksheet.write_string(0, 0, "Text")?;
worksheet.write_number(1, 0, 123.456)?;
worksheet.write_formula(2, 0, Formula::new("=SUM(A1:A10)"))?;
worksheet.write_blank(3, 0, &format)?;
worksheet.write_boolean(4, 0, true)?;
worksheet.write_datetime(5, 0, &datetime, &format)?;

// URL 和超链接
worksheet.write(6, 0, Url::new("https://example.com"))?;
```

### 格式化方法

```rust
// 设置列宽
worksheet.set_column_width(0, 20)?;

// 设置行高
worksheet.set_row_height(0, 30)?;

// 合并单元格
worksheet.merge_range(0, 0, 0, 3, "Title", &format)?;

// 自动筛选
worksheet.autofilter(0, 0, 10, 5)?;

// 冻结窗格
worksheet.freeze_panes(1, 0)?;

// 隐藏/显示
worksheet.hide();
worksheet.hide_columns(0, 2)?;
worksheet.hide_rows(0, 5)?;

// 设置标签名称
worksheet.set_name("My Sheet")?;
```

### 数据操作

```rust
// 区域写入
worksheet.write_row(0, 0, &[1, 2, 3, 4])?;
worksheet.write_column(0, 0, &[1, 2, 3, 4])?;

// 添加表格
worksheet.add_table(0, 0, 10, 5, &table)?;

// 添加条件格式
worksheet.conditional_format(0, 0, 10, 5, &cond_format)?;

// 添加数据验证
worksheet.data_validation(0, 0, 10, 0, &data_validation)?;

// 添加图表
worksheet.insert_chart(10, 0, &chart)?;
```

### 内部数据结构

- `cells` - 单元格数据存储
- `merges` - 合并单元格信息
- `options` - 工作表选项
- `protection` - 保护设置
- `autofilter` - 自动筛选信息
- `page_setup` - 页面设置

## Format

### 功能

`Format` 定义单元格的格式和样式。

### 格式类型

```rust
// 字体格式
let format = Format::new()
    .set_font_name("Arial")
    .set_font_size(12)
    .set_bold()
    .set_italic()
    .set_underline(FormatUnderline::Single)
    .set_font_color(Color::RGB(0xFF0000))
    .set_font_strikeout()
    .set_font_script(FormatScript::Superscript);

// 对齐格式
let format = Format::new()
    .set_align(FormatAlign::Center)
    .set_vertical_align(FormatAlign::Center)
    .set_text_wrap()
    .set_indent(2)
    .set_rotation(45);

// 边框格式
let format = Format::new()
    .set_border(FormatBorder::Thin)
    .set_border_color(Color::RGB(0x000000))
    .set_bottom(FormatBorder::Medium)
    .set_top(FormatBorder::Double);

// 填充格式
let format = Format::new()
    .set_background_color(Color::RGB(0xFFFF00))
    .set_pattern(FormatPattern::Solid)
    .set_foreground_color(Color::RGB(0x0000FF));

// 数字格式
let format = Format::new()
    .set_num_format("0.00")
    .set_num_format("yyyy-mm-dd")
    .set_num_format("$#,##0.00");

// 锁定和保护
let format = Format::new()
    .set_locked(false)
    .set_hidden(false);
```

### 格式继承

格式可以链式组合：

```rust
let base_format = Format::new()
    .set_font_name("Arial")
    .set_font_size(10);

let header_format = Format::new()
    .set_bold()
    .set_background_color(Color::RGB(0xCCFFCC));

// 格式可以组合使用
```

### 内部结构

- `font` - 字体属性
- `fill` - 填充属性
- `border` - 边框属性
- `alignment` - 对齐属性
- `number_format` - 数字格式
- `protection` - 保护属性
- `xf_id` - 格式 ID

## ExcelDateTime

### 功能

处理 Excel 中的日期和时间值。

### 创建方法

```rust
// 从年月日创建
let date = ExcelDateTime::from_ymd(2023, 1, 25)?;

// 从年月日时分秒创建
let datetime = ExcelDateTime::from_ymd_hms(2023, 1, 25, 10, 30, 0)?;

// 从 Excel 序列号创建
let excel_date = ExcelDateTime::from_excel_serial(44962)?;

// 从系统时间创建
let now = ExcelDateTime::from_system_time()?;

// 转换为 Excel 序列号
let serial = date.to_excel_serial();
```

### 与其他库集成

当启用相应 features 时：

```rust
// Chrono 集成
#[cfg(feature = "chrono")]
use chrono::NaiveDate;
let chrono_date = NaiveDate::from_ymd(2023, 1, 25);
let excel_date: ExcelDateTime = chrono_date.into();

// Jiff 集成
#[cfg(feature = "jiff")]
use jiff::civil::Date;
let jiff_date = Date::new(2023, 1, 25)?;
let excel_date: ExcelDateTime = jiff_date.into();
```

## Formula

### 功能

表示 Excel 公式和函数。

### 基本使用

```rust
// 简单公式
worksheet.write(0, 0, Formula::new("=SUM(A1:A10)"))?;

// 动态数组公式 (Excel 365)
worksheet.write_formula_dynamic(1, 0, "=UNIQUE(A1:A10)")?;

// 数组公式
worksheet.write_array_formula(2, 0, 2, 2, "{=SUM(A1:C1*A2:C2)}")?;

// 带格式的公式
worksheet.write_with_format(3, 0, Formula::new("=AVERAGE(B1:B10)"), &format)?;
```

### 公式优化

- 自动处理动态数组
- 支持 spilled ranges
- 自动更新公式引用

## Url

### 功能

处理超链接和 URL。

### 基本使用

```rust
// 简单 URL
worksheet.write(0, 0, Url::new("https://www.example.com"))?;

// 自定义显示文本
worksheet.write(1, 0, Url::new("https://www.example.com").set_text("Click Here"))?;

// 内部链接
worksheet.write(2, 0, Url::new("Sheet2!A1").set_text("Go to Sheet2"))?;

// 邮件链接
worksheet.write(3, 0, Url::new("mailto:user@example.com").set_text("Send Email"))?;
```

### 工具提示

```rust
let url = Url::new("https://www.example.com")
    .set_text("Visit Site")
    .set_tooltip("Click to visit our website");
```

## Error

### 功能

统一的错误类型 `XlsxError`。

### 错误类型

```rust
pub enum XlsxError {
    // 参数错误
    ParameterError(String),

    // 工作表名称错误
    SheetnameError(String),

    // 行列限制错误
    ColumnRowLimitError,

    // 工作表未找到
    WorksheetNotFound,

    // 图片错误
    ImageError(String),

    // IO 错误
    IoError(String),

    // ZIP 错误
    ZipError(String),
}
```

### 错误处理

```rust
use rust_xlsxwriter::*;

fn create_excel() -> Result<(), XlsxError> {
    let mut workbook = Workbook::new();
    // ... 操作 ...
    workbook.save("output.xlsx")?;
    Ok(())
}
```

## Color

### 功能

定义 Excel 中的颜色。

### 颜色类型

```rust
// 标准颜色
let color = Color::RED;
let color = Color::BLUE;
let color = Color::BLACK;
let color = Color::WHITE;

// RGB 颜色
let color = Color::RGB(0xFF0000); // 红色
let color = Color::RGB(0x00FF00); // 绿色
let color = Color::RGB(0x0000FF); // 蓝色

// 主题颜色
let color = Color::Theme(1, 0); // 主题颜色 1，亮度 0

// 自定义颜色索引
let color = Color::Indexed(10);
```

### 内置颜色

提供了一组标准颜色常量：

- `BLACK`, `WHITE`, `RED`, `GREEN`, `BLUE`
- `YELLOW`, `MAGENTA`, `CYAN`
- 等等...

## Properties

### 功能

管理文档和工作表的属性。

### DocProperties

```rust
let properties = DocProperties::new()
    .set_title("My Report")
    .set_author("John Doe")
    .set_comments("Quarterly report")
    .set_subject("Sales Data")
    .set_keywords("sales, report, 2023");

workbook.set_properties(&properties);
```

## Protection

### 功能

设置单元格和工作表保护。

### 单元格保护

```rust
let locked_format = Format::new().set_locked(true);
let unlocked_format = Format::new().set_locked(false);

let hidden_format = Format::new().set_hidden(true);
```

### 工作表保护

```rust
worksheet.protect()
    .set_password("password123")
    .set_object_locked(true)
    .set_scenario_locked(true)
    .set_format_cells_locked(false);
```

## 工具类型

### 其他核心类型

- `Alignment` - 对齐设置
- `Border` - 边框设置
- `Fill` - 填充设置
- `Font` - 字体设置
- `NumberFormat` - 数字格式

## 总结

核心模块提供了：

1. **数据结构**：表示 Excel 文档的基本元素
2. **类型系统**：类型安全的 API
3. **错误处理**：统一的错误类型
4. **基础功能**：数据写入和格式化

这些模块是整个库的基础，其他所有功能都构建在这些核心模块之上。