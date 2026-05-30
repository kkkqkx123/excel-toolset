# 工作表模块文档

本文档介绍与工作表和数据操作相关的模块，包括表格、条件格式、数据验证等。

## 工作表模块列表

- [Table](#table) - 表格功能
- [ConditionalFormat](#conditionalformat) - 条件格式
- [DataValidation](#datavalidation) - 数据验证
- [Filter](#filter) - 筛选功能
- [Sparkline](#sparkline) - 迷你图
- [Note](#note) - 单元格备注
- [Comment](#comment) - 批注

## Table

### 功能

`Table` 提供了 Excel 表格功能，用于将单元格区域组织成具有统一格式的表格。

### 基本使用

```rust
use rust_xlsxwriter::*;

// 创建表格
let table = Table::new()
    .set_style(TableStyle::Medium9)
    .set_name("SalesData")
    .set_first_column(true)
    .set_last_column(true)
    .set_banded_rows(true)
    .set_banded_columns(true);

worksheet.add_table(0, 0, 10, 5, &table)?;
```

### 表格样式

提供多种内置样式：

```rust
// 浅色样式
TableStyle::Light1 到 TableStyle::Light21

// 中等样式
TableStyle::Medium1 到 TableStyle::Medium28

// 深色样式
TableStyle::Dark1 到 TableStyle::Dark11
```

### 表列设置

```rust
let table = Table::new()
    .set_columns(&[
        TableColumn::new("Name").set_header_format(&header_format),
        TableColumn::new("Age").set_total_string("Average"),
        TableColumn::new("Salary").set_total_function(TableFunction::Average),
        TableColumn::new("Department"),
    ])
    .set_show_total_row(true)
    .set_autofilter(true);
```

### 高级功能

```rust
// 定义名称
.set_name("MyTable")

// 显示标题行
.set_show_header_row(true)

// 显示汇总行
.set_show_total_row(true)

// 第一列样式
.set_first_column(true)

// 最后一列样式
.set_last_column(true)

// 隔行样式
.set_banded_rows(true)

// 隔列样式
.set_banded_columns(true)

// 自动筛选
.set_autofilter(true)
```

## ConditionalFormat

### 功能

`ConditionalFormat` 提供条件格式功能，根据单元格值应用不同的格式。

### 条件格式类型

#### 1. 基于单元格值

```rust
// 大于
let cond_format = ConditionalFormatCell::new()
    .set_rule(ConditionalFormatCellRule::GreaterThan(100))
    .set_format(&format);

// 小于
.set_rule(ConditionalFormatCellRule::LessThan(50))

// 等于
.set_rule(ConditionalFormatCellRule::EqualTo(100))

// 介于
.set_rule(ConditionalFormatCellRule::Between(10, 100))

// 不介于
.set_rule(ConditionalFormatCellRule::NotBetween(10, 100))
```

#### 2. 基于公式

```rust
let cond_format = ConditionalFormatFormula::new()
    .set_rule("=A1>100")
    .set_format(&format);
```

#### 3. 数据条

```rust
let cond_format = ConditionalFormatDataBar::new()
    .set_min_type(ConditionalFormatDataBarType::Percent(0))
    .set_max_type(ConditionalFormatDataBarType::Percent(100))
    .set_color(Color::RGB(0x638EC6));
```

#### 4. 色阶

```rust
let cond_format = ConditionalFormatColorScale::new()
    .set_minimum(ConditionalFormatColorScaleCriteria::new(Color::RGB(0x63BE7B)))
    .set_midpoint(ConditionalFormatColorScaleCriteria::new(Color::RGB(0xFFEB84)))
    .set_maximum(ConditionalFormatColorScaleCriteria::new(Color::RGB(0xF8696B)));
```

#### 5. 图标集

```rust
let cond_format = ConditionalFormatIconSet::new()
    .set_icon_style(ConditionalFormatIconSetStyle::ThreeTrafficLights)
    .set_reverse_icons(true)
    .set_show_icons_only(true);
```

#### 6. 重复值

```rust
let cond_format = ConditionalFormatDuplicateValues::new()
    .set_format(&format);
```

#### 7. 前/后 N 项

```rust
// 前 10 项
let cond_format = ConditionalFormatTop10::new()
    .set_rule(ConditionalFormatTop10Rule::Top10)
    .set_format(&format);

// 前 10%
.set_rule(ConditionalFormatTop10Rule::TopPercent(10))
```

#### 8. 高于/低于平均值

```rust
// 高于平均值
let cond_format = ConditionalFormatAverage::new()
    .set_rule(ConditionalFormatAverageRule::AboveAverage)
    .set_format(&format);

// 低于平均值
.set_rule(ConditionalFormatAverageRule::BelowAverage)

// 高于平均值 1 个标准差
.set_rule(ConditionalFormatAverageRule::AboveOrEqual1StdDev)
```

#### 9. 文本包含

```rust
let cond_format = ConditionalFormatText::new()
    .set_rule(ConditionalFormatTextRule::Contains("Important"))
    .set_format(&format);
```

#### 10. 空白/非空白

```rust
// 空白单元格
let cond_format = ConditionalFormatBlanks::new()
    .set_format(&format);

// 非空白单元格
let cond_format = ConditionalFormatNoBlanks::new()
    .set_format(&format);
```

#### 11. 错误/无错误

```rust
// 包含错误
let cond_format = ConditionalFormatErrors::new()
    .set_format(&format);

// 不包含错误
let cond_format = ConditionalFormatNoErrors::new()
    .set_format(&format);
```

#### 12. 时间周期

```rust
// 昨天
let cond_format = ConditionalFormatTimePeriod::new()
    .set_rule(ConditionalFormatTimePeriodRule::Yesterday)
    .set_format(&format);

// 今天
.set_rule(ConditionalFormatTimePeriodRule::Today)

// 明天
.set_rule(ConditionalFormatTimePeriodRule::Tomorrow)

// 最近 7 天
.set_rule(ConditionalFormatTimePeriodRule::Last7Days)

// 本周
.set_rule(ConditionalFormatTimePeriodRule::ThisWeek)

// 上周
.set_rule(ConditionalFormatTimePeriodRule::LastWeek)
```

### 应用条件格式

```rust
// 应用到单元格区域
worksheet.conditional_format(0, 0, 10, 5, &cond_format)?;

// 多个条件格式
worksheet.conditional_format(0, 0, 10, 0, &cond_format1)?;
worksheet.conditional_format(0, 0, 10, 0, &cond_format2)?;
```

### 停止规则

```rust
let cond_format = ConditionalFormatCell::new()
    .set_rule(ConditionalFormatCellRule::GreaterThan(100))
    .set_format(&format)
    .set_stop_if_true(true);
```

## DataValidation

### 功能

`DataValidation` 提供数据验证功能，限制用户可以输入到单元格中的数据。

### 验证类型

#### 1. 整数验证

```rust
let data_validation = DataValidation::new()
    .set_formula(DataValidationFormula::Number)
    .set_allow_blank(true)
    .set_show_input_message(true)
    .set_prompt_title("Enter an integer")
    .set_prompt("Please enter an integer between 1 and 100")
    .set_show_error_message(true)
    .set_error_title("Invalid input")
    .set_error_style(DataValidationErrorStyle::Stop)
    .set_error("Value must be an integer between 1 and 100");

worksheet.data_validation(0, 0, 10, 0, &data_validation)?;
```

#### 2. 小数验证

```rust
let data_validation = DataValidation::new()
    .set_formula(DataValidationFormula::NumberBetween(0.0, 100.0));
```

#### 3. 列表验证

```rust
let data_validation = DataValidation::new()
    .set_formula(DataValidationFormula::List("Red,Green,Blue"))
    .set_in_cell_dropdown(true);
```

#### 4. 日期验证

```rust
let data_validation = DataValidation::new()
    .set_formula(DataValidationFormula::DateBetween(
        ExcelDateTime::from_ymd(2023, 1, 1)?,
        ExcelDateTime::from_ymd(2023, 12, 31)?
    ));
```

#### 5. 时间验证

```rust
let data_validation = DataValidation::new()
    .set_formula(DataValidationFormula::TimeBetween("09:00", "17:00"));
```

#### 6. 文本长度验证

```rust
let data_validation = DataValidation::new()
    .set_formula(DataValidationFormula::LengthBetween(1, 10));
```

#### 7. 自定义公式

```rust
let data_validation = DataValidation::new()
    .set_formula(DataValidationFormula::CustomFormula("=AND(A1>0,A1<100)"));
```

### 验证规则

```rust
// 介于
DataValidationFormula::NumberBetween(1, 100)
DataValidationFormula::DateBetween(start, end)
DataValidationFormula::TimeBetween("09:00", "17:00")
DataValidationFormula::LengthBetween(1, 10)

// 不介于
DataValidationFormula::NumberNotBetween(1, 100)

// 等于
DataValidationFormula::NumberEqualTo(50)

// 不等于
DataValidationFormula::NumberNotEqualTo(50)

// 大于
DataValidationFormula::NumberGreaterThan(0)

// 小于
DataValidationFormula::NumberLessThan(100)

// 大于等于
DataValidationFormula::NumberGreaterThanOrEqual(0)

// 小于等于
DataValidationFormula::NumberLessThanOrEqual(100)
```

### 错误样式

```rust
// 停止
DataValidationErrorStyle::Stop

// 警告
DataValidationErrorStyle::Warning

// 信息
DataValidationErrorStyle::Information
```

### 输入消息和错误消息

```rust
let data_validation = DataValidation::new()
    .set_show_input_message(true)
    .set_prompt_title("提示")
    .set_prompt("请输入 1-100 之间的整数")
    .set_show_error_message(true)
    .set_error_title("错误")
    .set_error("输入值必须在 1-100 之间")
    .set_error_style(DataValidationErrorStyle::Stop);
```

## Filter

### 功能

`Filter` 提供自动筛选功能，允许用户过滤数据。

### 基本使用

```rust
// 设置自动筛选区域
worksheet.autofilter(0, 0, 10, 5)?;
```

### 筛选条件

```rust
// 等于
worksheet.filter_column(0, "x >= 2000")?;

// 不等于
worksheet.filter_column(0, "x != 2000")?;

// 大于
worksheet.filter_column(0, "x > 2000")?;

// 小于
worksheet.filter_column(0, "x < 2000")?;

// 包含文本
worksheet.filter_column(1, "x == East")?;

// 日期筛选
worksheet.filter_column(2, "x > 2017-06-01")?;
```

### 高级筛选

```rust
// 多条件筛选（AND）
worksheet.filter_column_list(0, &["East", "North"])?;
```

## Sparkline

### 功能

`Sparkline` 提供迷你图功能，在单元格中显示小型图表。

### 迷你图类型

```rust
// 折线图
let sparkline = Sparkline::new().set_range(("Sheet1", 0, 0, 0, 10));

// 柱状图
let sparkline = Sparkline::new()
    .set_range(("Sheet1", 0, 0, 0, 10))
    .set_type(SparklineType::Column);

// 盈亏图
let sparkline = Sparkline::new()
    .set_range(("Sheet1", 0, 0, 0, 10))
    .set_type(SparklineType::WinLoss);
```

### 迷你图样式

```rust
let sparkline = Sparkline::new()
    .set_range(("Sheet1", 0, 0, 0, 10))
    .set_location((2, 0))
    .set_high_point_color(Color::RGB(0xFF0000))
    .set_low_point_color(Color::RGB(0x0000FF))
    .set_negative_points_color(Color::RGB(0x00FF00))
    .set_first_point_color(Color::RGB(0xFFFF00))
    .set_last_point_color(Color::RGB(0xFF00FF))
    .set_markers_color(Color::RGB(0x00FFFF))
    .set_line_weight(1.5)
    .set_show_markers(true)
    .set_show_high_point(true)
    .set_show_low_point(true)
    .set_show_first_point(true)
    .set_show_last_point(true)
    .set_show_negative_points(true)
    .set_reverse_axis_colors(true);
```

### 添加迷你图

```rust
worksheet.add_sparkline(0, 0, &sparkline)?;
```

### 群组迷你图

```rust
let sparkline1 = Sparkline::new().set_range(("Sheet1", 0, 0, 0, 10));
let sparkline2 = Sparkline::new().set_range(("Sheet1", 1, 0, 1, 10));

worksheet.add_sparkline_group(&[sparkline1, sparkline2])?;
```

## Note

### 功能

`Note` 提供单元格备注功能（旧式备注）。

### 基本使用

```rust
let note = Note::new("This is a cell note");

worksheet.set_note(0, 0, &note)?;
```

### 备注样式

```rust
let note = Note::new("Important note")
    .set_author("John Doe")
    .set_color(Color::RGB(0xFFFF00))
    .set_height(100)
    .set_width(200)
    .set_visible(false);
```

## Comment

### 功能

`Comment` 提供批注功能（现代样式）。

### 基本使用

```rust
let comment = Comment::new("This is a comment");

worksheet.insert_comment(0, 0, &comment)?;
```

### 批注样式

```rust
let comment = Comment::new("Important comment")
    .set_author("John Doe")
    .set_width(200)
    .set_height(100)
    .set_visible(true)
    .set_color(Color::RGB(0xFFFF00));
```

## 总结

工作表模块提供了丰富的数据操作功能：

1. **表格功能**：组织数据为结构化表格
2. **条件格式**：动态格式化单元格
3. **数据验证**：控制用户输入
4. **筛选功能**：数据过滤
5. **迷你图**：小型图表
6. **备注和批注**：单元格注释

这些功能使得 Excel 工作表不仅能够存储数据，还能提供丰富的数据展示和交互体验。