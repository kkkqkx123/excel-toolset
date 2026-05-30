# 高级特性模块文档

本文档介绍 rust_xlsxwriter 的高级特性模块，包括图表、宏、序列化等功能。

## 高级特性模块列表

- [Chart](#chart) - 图表功能
- [Image](#image) - 图片处理
- [Shape](#shape) - 形状和文本框
- [Button](#button) - 按钮控件
- [Macros](#macros) - 宏支持
- [Serializer](#serializer) - Serde 序列化
- [Drawing](#drawing) - 绘图对象
- [VML](#vml) - VML 绘图
- [RichValue](#richvalue) - 富值支持

## Chart

### 功能

`Chart` 提供了丰富的 Excel 图表功能，支持多种图表类型。

### 图表类型

#### 1. 柱状图

```rust
let mut chart = Chart::new(ChartType::Column);

// 添加数据系列
chart.add_series()
    .set_categories(("Sheet1", 0, 0, 4, 0))
    .set_values(("Sheet1", 0, 1, 4, 1))
    .set_name("Series 1");

chart.add_series()
    .set_categories(("Sheet1", 0, 0, 4, 0))
    .set_values(("Sheet1", 0, 2, 4, 2))
    .set_name("Series 2");
```

#### 2. 条形图

```rust
let mut chart = Chart::new(ChartType::Bar);
```

#### 3. 折线图

```rust
let mut chart = Chart::new(ChartType::Line);

// 设置线条样式
chart.add_series()
    .set_line(ChartLine::new().set_width(2.25))
    .set_marker(ChartMarker::new().set_type(ChartMarkerType::Circle));
```

#### 4. 饼图

```rust
let mut chart = Chart::new(ChartType::Pie);

// 设置饼图数据标签
chart.set_legend(ChartLegend::new()
    .set_position(ChartLegendPosition::Right)
    .set_show_legend_key(true));
```

#### 5. 散点图

```rust
let mut chart = Chart::new(ChartType::Scatter);

// 设置 X 和 Y 值
chart.add_series()
    .set_x_values(("Sheet1", 0, 0, 4, 0))
    .set_y_values(("Sheet1", 0, 1, 4, 1));
```

#### 6. 面积图

```rust
let mut chart = Chart::new(ChartType::Area);
```

#### 7. 环形图

```rust
let mut chart = Chart::new(ChartType::Doughnut);

// 设置内径大小
chart.set_hole_size(50);
```

#### 8. 雷达图

```rust
let mut chart = Chart::new(ChartType::Radar);
```

### 图表系列配置

```rust
chart.add_series()
    .set_name("Sales")
    .set_categories(("Sheet1", 0, 0, 11, 0))
    .set_values(("Sheet1", 0, 1, 11, 1))
    .set_line(ChartLine::new().set_color(Color::RGB(0xFF0000)))
    .set_fill(ChartFill::new().set_color(Color::RGB(0xFF0000)).set_transparency(50))
    .set_marker(ChartMarker::new()
        .set_type(ChartMarkerType::Circle)
        .set_size(8)
        .set_border(ChartLine::new().set_color(Color::RGB(0x000000))))
    .set_smooth(true)
    .set_invert_if_negative(true)
    .set_labels(ChartLabel::new()
        .set_value(true)
        .set_category(true)
        .set_series_name(true));
```

### 图表标题

```rust
chart.set_title(ChartTitle::new("Sales Report")
    .set_name_font(ChartFont::new()
        .set_bold()
        .set_size(14)
        .set_color(Color::RGB(0x0000FF)))
    .set_name_layout(ChartLayout::new()
        .set_x(0.5)
        .set_y(0.9)));
```

### 图例

```rust
chart.set_legend(ChartLegend::new()
    .set_position(ChartLegendPosition::Bottom)
    .set_show_legend_key(false)
    .set_delete_series(0) // 删除指定系列
    .set_font(ChartFont::new().set_size(10)));
```

### 坐标轴

```rust
// X 轴
chart.set_x_axis(ChartAxis::new()
    .set_name("Month")
    .set_num_font(ChartFont::new().set_size(10))
    .set_line(ChartLine::new().set_color(Color::RGB(0x000000)))
    .set_major_gridlines(ChartGridLine::new()
        .set_visible(true)
        .set_line(ChartLine::new().set_color(Color::RGB(0xC0C0C0)))));

// Y 轴
chart.set_y_axis(ChartAxis::new()
    .set_name("Sales")
    .set_num_format("$#,##0")
    .set_min(0)
    .set_max(100)
    .set_log_base(10));
```

### 绘图区

```rust
chart.set_plotarea(ChartPlotArea::new()
    .set_border(ChartLine::new()
        .set_color(Color::RGB(0x000000))
        .set_hidden(false))
    .set_fill(ChartFill::new()
        .set_color(Color::RGB(0xFFFFFF))));
```

### 图表区

```rust
chart.set_chartarea(ChartFill::new()
    .set_color(Color::RGB(0xF2F2F2)));
```

### 数据标签

```rust
chart.add_series()
    .set_labels(ChartLabel::new()
        .set_value(true)
        .set_category(false)
        .set_series_name(true)
        .set_leader_lines(true)
        .set_num_format("0.00")
        .set_position(ChartLabelPosition::Above));
```

### 趋势线

```rust
chart.add_series()
    .set_trendline(ChartTrendline::new()
        .set_type(ChartTrendlineType::Linear)
        .set_name("Trend")
        .set_forward(0.5)
        .set_backward(0.5)
        .set_intercept(0)
        .set_display_equation(true)
        .set_display_r_squared(true));
```

### 误差线

```rust
chart.add_series()
    .set_y_error_bars(ChartErrorBars::new()
        .set_type(ChartErrorBarType::StandardError)
        .set_value(5)
        .set_plus(3)
        .set_minus(2)
        .set_line(ChartLine::new().set_color(Color::RGB(0xFF0000))));
```

### 组合图表

```rust
let mut chart = Chart::new(ChartType::Column);

// 主图表（柱状图）
chart.add_series()
    .set_values(("Sheet1", 0, 1, 4, 1))
    .set_secondary_axis(false);

// 次图表（折线图）
chart.add_series()
    .set_values(("Sheet1", 0, 2, 4, 2))
    .set_chart_type(ChartType::Line)
    .set_secondary_axis(true);
```

### 嵌入图表

```rust
// 在工作表中插入图表
worksheet.insert_chart(0, 0, &chart)?;

// 设置图表大小
worksheet.insert_chart_with_size(0, 0, &chart, 480, 288)?;
```

## Image

### 功能

`Image` 处理图片插入和显示。

### 支持的格式

- PNG
- JPEG
- GIF
- BMP

### 基本使用

```rust
// 创建图片
let image = Image::new("logo.png")?;

// 插入图片
worksheet.insert_image(0, 0, &image)?;
```

### 图片缩放

```rust
let image = Image::new("photo.jpg")?
    .set_scale_x(0.5)  // 水平缩放 50%
    .set_scale_y(0.5); // 垂直缩放 50%
```

### 偏移和位置

```rust
let image = Image::new("logo.png")?
    .set_offset_x(10)  // X 偏移（像素）
    .set_offset_y(5);  // Y 偏移（像素）

worksheet.insert_image(1, 2, &image)?;
```

### 对象定位

```rust
let image = Image::new("logo.png")?
    .set_object_position(ImageObjectPosition::OneCell); // 在单元格中

worksheet.insert_image(0, 0, &image)?;
```

### 描述和替代文本

```rust
let image = Image::new("chart.png")?
    .set_description("Sales Chart")
    .set_decorative(false);
```

## Shape

### 功能

`Shape` 提供形状和文本框功能。

### 文本框

```rust
let shape = Shape::new()
    .set_name("TextBox1")
    .set_width(200)
    .set_height(100)
    .set_text("This is a text box")
    .set_text_rotation(45)
    .set_line(ShapeLine::new().set_color(Color::RGB(0x000000)))
    .set_fill(ShapeFill::new().set_color(Color::RGB(0xFFFF00)));

worksheet.insert_shape(0, 0, &shape)?;
```

### 形状类型

```rust
// 矩形
let shape = Shape::new()
    .set_shape_type(ShapeType::Rect);

// 椭圆
let shape = Shape::new()
    .set_shape_type(ShapeType::Oval);

// 其他形状...
```

### 形状样式

```rust
let shape = Shape::new()
    .set_line(ShapeLine::new()
        .set_color(Color::RGB(0x000000))
        .set_width(1))
    .set_fill(ShapeFill::new()
        .set_color(Color::RGB(0xFF0000))
        .set_transparency(50));
```

## Button

### 功能

`Button` 提供按钮控件功能。

### 创建按钮

```rust
let button = Button::new()
    .set_name("Button1")
    .set_caption("Click Me")
    .set_width(100)
    .set_height(30)
    .set_macro("MyMacro");

worksheet.insert_button(0, 0, &button)?;
```

### 按钮样式

```rust
let button = Button::new()
    .set_font(Font::new()
        .set_bold()
        .set_size(11))
    .set_fill(Fill::new()
        .set_color(Color::RGB(0xCCCCCC)))
    .set_line(Line::new()
        .set_color(Color::RGB(0x000000)));
```

## Macros

### 功能

`Macros` 模块支持在 Excel 文件中嵌入 VBA 宏。

### 启用宏支持

```toml
# Cargo.toml
[dependencies]
rust_xlsxwriter = { version = "0.95", features = ["macros"] }
```

### 基本使用

```rust
let mut workbook = Workbook::new();

// 设置 VBA 项目
let vba_project = VbaProject::new("macros.xlam");
workbook.set_vba_project(&vba_project);

// 创建工作表
let worksheet = workbook.add_worksheet();

// 添加按钮调用宏
let button = Button::new()
    .set_caption("Run Macro")
    .set_macro("MyMacro");
worksheet.insert_button(0, 0, &button);

workbook.save("output.xlsm")?;
```

### 宏文件格式

需要提供包含 VBA 代码的 `.xlam` 文件。

### 注意事项

- 仅支持 `.xlsm` 格式（启用宏的工作簿）
- 需要用户提供 VBA 代码文件
- 不支持从代码生成 VBA

## Serializer

### 功能

`Serializer` 提供 Serde 序列化支持，将 Rust 结构体直接序列化为 Excel 数据。

### 启用序列化

```toml
# Cargo.toml
[dependencies]
rust_xlsxwriter = { version = "0.95", features = ["serde"] }
```

### 基本序列化

```rust
use rust_xlsxwriter::XlsxSerialize;

#[derive(XlsxSerialize)]
struct Employee {
    #[xlsx(rename = "Name")]
    name: String,

    #[xlsx(rename = "Age")]
    age: u32,

    #[xlsx(rename = "Salary")]
    salary: f64,
}

fn main() -> Result<(), XlsxError> {
    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();

    let employees = vec![
        Employee {
            name: "Alice".to_string(),
            age: 30,
            salary: 50000.0,
        },
        Employee {
            name: "Bob".to_string(),
            age: 25,
            salary: 45000.0,
        },
    ];

    worksheet.serialize_headers(&employees)?;
    worksheet.serialize(&employees)?;

    workbook.save("employees.xlsx")?;

    Ok(())
}
```

### 高级序列化选项

```rust
#[derive(XlsxSerialize)]
struct Product {
    #[xlsx(rename = "Product Name", skip = false)]
    name: String,

    #[xlsx(rename = "Price", format = "$#,##0.00")]
    price: f64,

    #[xlsx(rename = "Quantity", header_format = "bold")]
    quantity: u32,

    #[xlsx(rename = "Total", formula = "=[Price]*[Quantity]")]
    total: f64,

    #[xlsx(skip_serializing)]
    internal_id: String,
}
```

### 自定义表头

```rust
worksheet.serialize_headers_with_options(
    &employees,
    &XlsxSerializeHeadersOptions::new()
        .set_rename("name", "Full Name")
        .set_skip("internal_id", true)
)?;
```

### 序列化到表格

```rust
let table = Table::new()
    .set_style(TableStyle::Medium9)
    .set_name("Products");

worksheet.serialize_table(0, 0, 5, 3, &products, &table)?;
```

## Drawing

### 功能

`Drawing` 模块处理绘图对象和图表定位。

### 绘图关系

```rust
// 管理绘图关系
let drawing = Drawing::new();

// 添加图表关系
drawing.add_chart_relationship(chart_id);
```

### 坐标系

```rust
// EMU (English Metric Units) 坐标系
// 1 inch = 914400 EMU
// 1 cm = 360000 EMU
```

## VML

### 功能

`VML` 处理 Vector Markup Language 绘图对象。

### VML 用途

- 批注（旧式）
- 按钮
- 其他旧式绘图对象

### VML 结构

```xml
<v:shapetype id="_x0000_t202" ...>
    <v:path .../>
    <v:textbox .../>
</v:shapetype>
```

## RichValue

### 功能

`RichValue` 支持 Excel 的富值功能，用于复杂的数据类型。

### 富值类型

- `RichValueRel` - 富值关系
- `RichValueStructure` - 富值结构
- `RichValueTypes` - 富值类型

## 总结

高级特性模块提供了丰富的 Excel 功能：

1. **图表**：多种图表类型和自定义选项
2. **图片**：图片插入和格式化
3. **形状**：文本框和形状
4. **按钮**：交互控件
5. **宏**：VBA 宏支持
6. **序列化**：Serde 集成
7. **绘图**：高级绘图功能

这些功能使得 rust_xlsxwriter 能够创建专业级的 Excel 文档。