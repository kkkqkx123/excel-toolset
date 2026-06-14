# Excel 工具网关写入功能实现分析

## 概述

本文档基于 `rust_xlsxwriter` 库的现有 API，详细分析如何实现 Excel 工具网关中所需的各种写入功能。`rust_xlsxwriter` 是一个纯写入库，专注于创建和修改 Excel 文件，与 `calamine`（只读库）配合可实现完整的读写能力。

**关键技术特性：**
- 纯 Rust 实现，高性能
- 仅支持写入操作（创建新文件或重新生成文件）
- 支持常量内存模式处理大文件
- 完善的格式支持和公式支持
- 与 Excel 生成的文件格式高度一致

---

## 一、文件/工作簿操作

### 1.1 创建新文件

**功能描述：** 创建新的 Excel 工作簿文件

**实现 API：**
- `Workbook::new()` - 创建新的工作簿实例
- `Workbook::save(path)` - 保存文件到指定路径
- `Workbook::save_to_buffer()` - 保存到内存缓冲区

**代码位置：** `src/workbook.rs:514-654`

**实现示例：**
```rust
use rust_xlsxwriter::{Workbook, XlsxError};

fn create_new_file(filepath: &str) -> Result<(), XlsxError> {
    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();
    worksheet.write_string(0, 0, "Hello")?;
    workbook.save(filepath)?;
    Ok(())
}
```

**安全考虑：**
- 支持 `dry_run` 模式，使用 `save_to_buffer()` 预览结果
- 自动文件备份在调用 `save()` 前执行
- 计算文件哈希作为指纹

---

### 1.2 读取文件信息（底层实现）

**说明：** 由于 `rust_xlsxwriter` 是只写库，文件信息读取需使用 `calamine` 库

**实现建议：**
- 使用 `calamine::Reader` 读取现有文件
- 获取工作表列表、单元格范围等信息
- 结合 `rust_xlsxwriter` 重新生成文件

---

### 1.3 保存文件

**功能描述：** 保存工作簿到文件系统

**实现 API：**
- `Workbook::save(path: P)` - 保存到文件路径
- `Workbook::save_to_buffer() -> Result<Vec<u8>>` - 保存到内存

**代码位置：** `src/workbook.rs:88-107`

**实现示例：**
```rust
use rust_xlsxwriter::{Workbook, XlsxError};

fn save_workbook(workbook: &mut Workbook, filepath: &str) -> Result<(), XlsxError> {
    workbook.save(filepath)?;
    Ok(())
}

// 预执行模式
fn dry_run_save(workbook: &mut Workbook) -> Result<Vec<u8>, XlsxError> {
    workbook.save_to_buffer()
}
```

**安全集成：**
- 保存前调用备份模块创建文件快照
- 计算保存后的文件哈希
- 支持回滚到备份文件

---

## 二、工作表操作

### 2.1 新增工作表

**功能描述：** 向工作簿添加新工作表

**实现 API：**
- `Workbook::add_worksheet() -> &mut Worksheet` - 添加标准工作表
- `Workbook::add_worksheet_with_constant_memory() -> &mut Worksheet` - 添加常量内存工作表
- `Workbook::push_worksheet(worksheet)` - 推入已创建的工作表

**代码位置：** `src/workbook.rs:514-654`

**实现示例：**
```rust
use rust_xlsxwriter::{Workbook, XlsxError};

fn add_worksheet(workbook: &mut Workbook, name: &str) -> Result<(), XlsxError> {
    let worksheet = workbook.add_worksheet();
    worksheet.set_name(name)?;
    Ok(())
}
```

---

### 2.2 删除工作表（需要特殊实现）

**功能描述：** 删除指定工作表

**实现挑战：** `rust_xlsxwriter` 不支持直接删除工作表，需要重新生成文件

**实现方案：**
1. 使用 `calamine` 读取原文件
2. 遍历所有工作表，跳过要删除的工作表
3. 使用 `rust_xlsxwriter` 重新生成文件

**实现示例：**
```rust
use calamine::{Reader, Xlsx, open_workbook};
use rust_xlsxwriter::{Workbook, Worksheet, XlsxError};

fn delete_worksheet(input_path: &str, output_path: &str, sheet_name: &str) -> Result<(), XlsxError> {
    // 1. 读取原文件
    let mut workbook: Xlsx<_> = open_workbook(input_path).unwrap();

    // 2. 创建新工作簿
    let mut new_workbook = Workbook::new();

    // 3. 复制除目标工作表外的所有工作表
    for sheet in workbook.sheet_names() {
        if sheet != sheet_name {
            // 使用 calamine 读取工作表数据
            let range = workbook.worksheet_range(&sheet).unwrap();

            // 使用 rust_xlsxwriter 创建新工作表并写入数据
            let worksheet = new_workbook.add_worksheet().set_name(&sheet)?;
            for (row_idx, row) in range.rows().enumerate() {
                for (col_idx, cell) in row.iter().enumerate() {
                    match cell {
                        calamine::Cell::String(s) => {
                            worksheet.write_string(row_idx as u32, col_idx as u16, s)?;
                        },
                        calamine::Cell::Float(f) => {
                            worksheet.write_number(row_idx as u32, col_idx as u16, *f)?;
                        },
                        // 处理其他数据类型...
                        _ => {}
                    }
                }
            }
        }
    }

    // 4. 保存新文件
    new_workbook.save(output_path)?;
    Ok(())
}
```

---

### 2.3 重命名工作表

**功能描述：** 修改工作表名称

**实现 API：**
- `Worksheet::set_name(name) -> &mut Worksheet` - 设置工作表名称

**代码位置：** `src/worksheet.rs`

**实现示例：**
```rust
use rust_xlsxwriter::{Workbook, XlsxError};

fn rename_worksheet(workbook: &mut Workbook, old_name: &str, new_name: &str) -> Result<(), XlsxError> {
    let worksheet = workbook.worksheet_from_name(old_name)?;
    worksheet.set_name(new_name)?;
    Ok(())
}
```

---

## 三、单元格/区域操作（核心读写）

### 3.1 写入单个单元格

**功能描述：** 向指定单元格写入数据

**实现 API：**
- `Worksheet::write(row, col, data)` - 通用写入方法
- `Worksheet::write_with_format(row, col, data, format)` - 带格式写入
- `Worksheet::write_string(row, col, string)` - 写入字符串
- `Worksheet::write_number(row, col, number)` - 写入数字
- `Worksheet::write_datetime(row, col, datetime)` - 写入日期时间

**代码位置：** `src/worksheet.rs:2149-4889`

**实现示例：**
```rust
use rust_xlsxwriter::{Workbook, XlsxError};

fn write_cell(worksheet: &mut Worksheet, row: u32, col: u16, value: &str) -> Result<(), XlsxError> {
    worksheet.write_string(row, col, value)?;
    Ok(())
}

fn write_cell_with_format(worksheet: &mut Worksheet, row: u32, col: u16, value: f64, format: &Format) -> Result<(), XlsxError> {
    worksheet.write_with_format(row, col, value, format)?;
    Ok(())
}
```

---

### 3.2 写入区域单元格

**功能描述：** 批量写入单元格区域数据

**实现 API：**
- `Worksheet::write_row(row, col, iterator)` - 写入一行数据
- `Worksheet::write_column(row, col, iterator)` - 写入一列数据
- `Worksheet::write_row_matrix(row, col, iterator)` - 写入行矩阵
- `Worksheet::write_column_matrix(row, col, iterator)` - 写入列矩阵

**代码位置：** `src/worksheet.rs:2340-2667`

**实现示例：**
```rust
use rust_xlsxwriter::{Workbook, XlsxError};

fn write_range(worksheet: &mut Worksheet, start_row: u32, start_col: u16, data: &[Vec<String>]) -> Result<(), XlsxError> {
    for (row_idx, row_data) in data.iter().enumerate() {
        for (col_idx, cell_value) in row_data.iter().enumerate() {
            worksheet.write_string(
                start_row + row_idx as u32,
                start_col + col_idx as u16,
                cell_value
            )?;
        }
    }
    Ok(())
}

fn write_row_data(worksheet: &mut Worksheet, row: u32, col: u16, data: &[&str]) -> Result<(), XlsxError> {
    worksheet.write_row(row, col, data.iter())?;
    Ok(())
}
```

---

### 3.3 清空单元格区域

**功能描述：** 清除指定区域的内容

**实现 API：**
- `Worksheet::write_blank(row, col)` - 写入空白单元格
- 批量写入空白单元格覆盖区域

**代码位置：** `src/worksheet.rs:3910`

**实现示例：**
```rust
use rust_xlsxwriter::{Worksheet, XlsxError};

fn clear_range(worksheet: &mut Worksheet, start_row: u32, start_col: u16, end_row: u32, end_col: u16) -> Result<(), XlsxError> {
    for row in start_row..=end_row {
        for col in start_col..=end_col {
            worksheet.write_blank(row, col)?;
        }
    }
    Ok(())
}
```

---

## 四、数据处理操作

### 4.1 追加行

**功能描述：** 在工作表末尾追加新行

**实现方案：**
1. 使用 `calamine` 读取现有数据
2. 确定最后一行的位置
3. 使用 `rust_xlsxwriter` 写入新行

**实现示例：**
```rust
use calamine::{Reader, Xlsx, open_workbook};
use rust_xlsxwriter::{Workbook, XlsxError};

fn append_row(input_path: &str, output_path: &str, sheet_name: &str, new_row: &[&str]) -> Result<(), XlsxError> {
    // 1. 读取原文件
    let mut workbook: Xlsx<_> = open_workbook(input_path).unwrap();

    // 2. 创建新工作簿
    let mut new_workbook = Workbook::new();

    // 3. 复制所有工作表
    for sheet in workbook.sheet_names() {
        let range = workbook.worksheet_range(&sheet).unwrap();
        let worksheet = new_workbook.add_worksheet().set_name(&sheet)?;

        // 复制原数据
        for (row_idx, row) in range.rows().enumerate() {
            for (col_idx, cell) in row.iter().enumerate() {
                match cell {
                    calamine::Cell::String(s) => {
                        worksheet.write_string(row_idx as u32, col_idx as u16, s)?;
                    },
                    calamine::Cell::Float(f) => {
                        worksheet.write_number(row_idx as u32, col_idx as u16, *f)?;
                    },
                    _ => {}
                }
            }
        }

        // 追加新行（如果是指定工作表）
        if sheet == sheet_name {
            let last_row = range.height();
            for (col_idx, value) in new_row.iter().enumerate() {
                worksheet.write_string(last_row as u32, col_idx as u16, value)?;
            }
        }
    }

    // 4. 保存新文件
    new_workbook.save(output_path)?;
    Ok(())
}
```

---

### 4.2 插入行

**功能描述：** 在指定位置插入新行，下方数据下移

**实现方案：**
1. 读取原文件数据
2. 在内存中重新组织数据
3. 重新生成文件

**实现示例：**
```rust
use calamine::{Reader, Xlsx, open_workbook};
use rust_xlsxwriter::{Workbook, XlsxError};

fn insert_row(input_path: &str, output_path: &str, sheet_name: &str, insert_row_num: usize, new_row: &[&str]) -> Result<(), XlsxError> {
    let mut workbook: Xlsx<_> = open_workbook(input_path).unwrap();
    let mut new_workbook = Workbook::new();

    for sheet in workbook.sheet_names() {
        let range = workbook.worksheet_range(&sheet).unwrap();
        let worksheet = new_workbook.add_worksheet().set_name(&sheet)?;

        for (row_idx, row) in range.rows().enumerate() {
            // 在插入位置写入新行
            if sheet == sheet_name && row_idx == insert_row_num {
                for (col_idx, value) in new_row.iter().enumerate() {
                    worksheet.write_string(row_idx as u32, col_idx as u16, value)?;
                }
            }

            // 写入原数据（如果插入行后）
            let write_row_idx = if sheet == sheet_name && row_idx >= insert_row_num {
                row_idx + 1
            } else {
                row_idx
            };

            for (col_idx, cell) in row.iter().enumerate() {
                match cell {
                    calamine::Cell::String(s) => {
                        worksheet.write_string(write_row_idx as u32, col_idx as u16, s)?;
                    },
                    calamine::Cell::Float(f) => {
                        worksheet.write_number(write_row_idx as u32, col_idx as u16, *f)?;
                    },
                    _ => {}
                }
            }
        }
    }

    new_workbook.save(output_path)?;
    Ok(())
}
```

---

### 4.3 删除行

**功能描述：** 删除指定行，下方数据上移

**实现方案：**
类似插入行，但在内存中跳过要删除的行

**实现示例：**
```rust
use calamine::{Reader, Xlsx, open_workbook};
use rust_xlsxwriter::{Workbook, XlsxError};

fn delete_row(input_path: &str, output_path: &str, sheet_name: &str, delete_row_num: usize) -> Result<(), XlsxError> {
    let mut workbook: Xlsx<_> = open_workbook(input_path).unwrap();
    let mut new_workbook = Workbook::new();

    for sheet in workbook.sheet_names() {
        let range = workbook.worksheet_range(&sheet).unwrap();
        let worksheet = new_workbook.add_worksheet().set_name(&sheet)?;

        for (row_idx, row) in range.rows().enumerate() {
            // 跳过要删除的行
            if sheet == sheet_name && row_idx == delete_row_num {
                continue;
            }

            let write_row_idx = if sheet == sheet_name && row_idx > delete_row_num {
                row_idx - 1
            } else {
                row_idx
            };

            for (col_idx, cell) in row.iter().enumerate() {
                match cell {
                    calamine::Cell::String(s) => {
                        worksheet.write_string(write_row_idx as u32, col_idx as u16, s)?;
                    },
                    calamine::Cell::Float(f) => {
                        worksheet.write_number(write_row_idx as u32, col_idx as u16, *f)?;
                    },
                    _ => {}
                }
            }
        }
    }

    new_workbook.save(output_path)?;
    Ok(())
}
```

---

### 4.4 数据筛选（纯查询，不修改文件）

**功能描述：** 根据条件筛选数据，返回结果

**实现方案：**
- 使用 `calamine` 读取数据
- 在内存中进行筛选
- 返回筛选结果

---

### 4.5 数据排序

**功能描述：** 对指定列或区域进行排序

**实现方案：**
1. 使用 `calamine` 读取数据
2. 在内存中排序
3. 使用 `rust_xlsxwriter` 重新生成文件

**实现示例：**
```rust
use calamine::{Reader, Xlsx, open_workbook};
use rust_xlsxwriter::{Workbook, XlsxError};

fn sort_data(input_path: &str, output_path: &str, sheet_name: &str, sort_col: usize) -> Result<(), XlsxError> {
    let mut workbook: Xlsx<_> = open_workbook(input_path).unwrap();
    let mut new_workbook = Workbook::new();

    for sheet in workbook.sheet_names() {
        let range = workbook.worksheet_range(&sheet).unwrap();
        let worksheet = new_workbook.add_worksheet().set_name(&sheet)?;

        if sheet == sheet_name {
            // 转换为可排序的数据结构
            let mut data: Vec<Vec<String>> = range.rows()
                .map(|row| row.iter().map(|cell| match cell {
                    calamine::Cell::String(s) => s.clone(),
                    calamine::Cell::Float(f) => f.to_string(),
                    _ => String::new()
                }).collect())
                .collect();

            // 排序（跳过标题行）
            if data.len() > 1 {
                data[1..].sort_by(|a, b| a[sort_col].cmp(&b[sort_col]));
            }

            // 写入排序后的数据
            for (row_idx, row) in data.iter().enumerate() {
                for (col_idx, value) in row.iter().enumerate() {
                    worksheet.write_string(row_idx as u32, col_idx as u16, value)?;
                }
            }
        } else {
            // 复制其他工作表
            for (row_idx, row) in range.rows().enumerate() {
                for (col_idx, cell) in row.iter().enumerate() {
                    match cell {
                        calamine::Cell::String(s) => {
                            worksheet.write_string(row_idx as u32, col_idx as u16, s)?;
                        },
                        calamine::Cell::Float(f) => {
                            worksheet.write_number(row_idx as u32, col_idx as u16, *f)?;
                        },
                        _ => {}
                    }
                }
            }
        }
    }

    new_workbook.save(output_path)?;
    Ok(())
}
```

---

### 4.6 数据去重

**功能描述：** 删除重复行，保留唯一值

**实现方案：**
类似排序，在内存中使用 `HashSet` 去重

---

## 五、公式与计算

### 5.1 设置单元格公式

**功能描述：** 向单元格写入 Excel 公式

**实现 API：**
- `Worksheet::write_formula(row, col, formula)` - 写入公式
- `Worksheet::write_formula_with_format(row, col, formula, format)` - 带格式写入公式
- `Worksheet::write_array_formula(row, col, formula)` - 写入数组公式
- `Worksheet::write_dynamic_array_formula(row, col, formula)` - 写入动态数组公式

**代码位置：** `src/worksheet.rs:3298-3838`

**实现示例：**
```rust
use rust_xlsxwriter::{Formula, Workbook, XlsxError};

fn set_formula(worksheet: &mut Worksheet, row: u32, col: u16, formula: &str) -> Result<(), XlsxError> {
    worksheet.write_formula(row, col, formula)?;
    Ok(())
}

fn set_complex_formula(worksheet: &mut Worksheet) -> Result<(), XlsxError> {
    worksheet.write_formula(0, 0, "=SUM(A1:A10)")?;
    worksheet.write_formula(1, 0, "=AVERAGE(B1:B10)")?;
    worksheet.write_formula(2, 0, "=IF(C1>100,\"High\",\"Low\")")?;
    worksheet.write_formula(3, 0, "=VLOOKUP(A1,Sheet2!A:B,2,FALSE)")?;
    Ok(())
}
```

---

### 5.2 刷新计算值

**功能描述：** 重新计算公式值

**说明：** `rust_xlsxwriter` 生成的公式在 Excel 打开时会自动计算，无需特殊处理

**实现建议：**
- 公式在保存时已正确写入
- Excel 打开文件时会自动重新计算所有公式
- 如需预计算值，需在外部计算后写入结果

---

## 六、格式样式操作

### 6.1 设置样式（字体/颜色/边框）

**功能描述：** 设置单元格格式

**实现 API：**
- `Format::new()` - 创建格式对象
- `Format::set_font_name(name)` - 设置字体
- `Format::set_font_size(size)` - 设置字号
- `Format::set_bold()` - 设置粗体
- `Format::set_italic()` - 设置斜体
- `Format::set_font_color(color)` - 设置字体颜色
- `Format::set_background_color(color)` - 设置背景颜色
- `Format::set_border(border_type)` - 设置边框
- `Format::set_align(align_type)` - 设置对齐方式

**代码位置：** `src/format.rs`

**实现示例：**
```rust
use rust_xlsxwriter::{Color, Format, FormatAlign, FormatBorder, Workbook, XlsxError};

fn set_cell_format(worksheet: &mut Worksheet, row: u32, col: u16, value: &str) -> Result<(), XlsxError> {
    let format = Format::new()
        .set_font_name("Arial")
        .set_font_size(12)
        .set_bold()
        .set_font_color(Color::Red)
        .set_background_color(Color::Yellow)
        .set_border(FormatBorder::Thin)
        .set_align(FormatAlign::Center);

    worksheet.write_with_format(row, col, value, &format)?;
    Ok(())
}

fn set_multiple_formats(worksheet: &mut Worksheet) -> Result<(), XlsxError> {
    let header_format = Format::new()
        .set_bold()
        .set_background_color(Color::RGB(0x4472C4))
        .set_font_color(Color::White);

    let number_format = Format::new()
        .set_num_format("#,##0.00")
        .set_align(FormatAlign::Right);

    worksheet.write_with_format(0, 0, "Total", &header_format)?;
    worksheet.write_with_format(1, 0, 12345.67, &number_format)?;

    Ok(())
}
```

---

### 6.2 合并单元格

**功能描述：** 合并指定区域的单元格

**实现 API：**
- `Worksheet::merge_range(first_row, first_col, last_row, last_col, data, format)` - 合并区域

**代码位置：** `src/worksheet.rs:4985`

**实现示例：**
```rust
use rust_xlsxwriter::{Format, FormatAlign, Workbook, XlsxError};

fn merge_cells(worksheet: &mut Worksheet, first_row: u32, first_col: u16, last_row: u32, last_col: u16, text: &str) -> Result<(), XlsxError> {
    let format = Format::new()
        .set_align(FormatAlign::Center)
        .set_align(FormatAlign::VerticalCenter);

    worksheet.merge_range(first_row, first_col, last_row, last_col, text, &format)?;
    Ok(())
}

fn merge_title_cells(worksheet: &mut Worksheet) -> Result<(), XlsxError> {
    let title_format = Format::new()
        .set_bold()
        .set_font_size(16)
        .set_align(FormatAlign::Center)
        .set_align(FormatAlign::VerticalCenter);

    // 合并 A1:E1 作为标题
    worksheet.merge_range(0, 0, 0, 4, "销售报表", &title_format)?;

    // 合并 A2:C2 作为副标题
    let subtitle_format = Format::new()
        .set_italic()
        .set_align(FormatAlign::Center);

    worksheet.merge_range(1, 0, 1, 2, "2024年度统计", &subtitle_format)?;

    Ok(())
}
```

---

## 七、高级操作

### 7.1 生成图表

**功能描述：** 在工作表中创建图表

**实现 API：**
- `Chart::new(chart_type)` - 创建图表
- `Chart::add_series()` - 添加数据系列
- `Worksheet::insert_chart(row, col, chart)` - 插入图表

**代码位置：** `src/chart.rs`

**实现示例：**
```rust
use rust_xlsxwriter::{Chart, ChartType, Workbook, XlsxError};

fn create_chart(workbook: &mut Workbook, worksheet: &mut Worksheet) -> Result<(), XlsxError> {
    // 创建柱状图
    let mut chart = Chart::new(ChartType::Column);

    // 添加数据系列
    chart
        .add_series()
        .set_categories("Sheet1!$A$2:$A$5")
        .set_values("Sheet1!$B$2:$B$5")
        .set_name("销售额");

    // 设置图表标题
    chart.title().set_name("月度销售统计");

    // 插入图表到工作表
    worksheet.insert_chart(0, 2, &chart)?;

    Ok(())
}
```

---

### 7.2 生成数据透视表

**说明：** `rust_xlsxwriter` 当前版本不直接支持数据透视表，需要通过公式或其他方式实现

**实现建议：**
- 使用 `SUMIF`、`COUNTIF` 等公式实现类似功能
- 或使用外部库处理后写入结果

---

### 7.3 SQL查询

**说明：** 此功能需要专门的 SQL 引擎集成

**实现建议：**
- 使用 `polars` 或类似库处理数据
- 在内存中执行 SQL 查询
- 将结果写入 Excel

---

## 八、VBA操作

### 8.1 导入VBA二进制流

**功能描述：** 向工作簿导入VBA宏代码

**实现 API：**
- `Workbook::add_vba_project(path)` - 添加VBA项目文件

**代码位置：** `src/macros.rs`

**实现示例：**
```rust
use rust_xlsxwriter::{Workbook, XlsxError};

fn import_vba(workbook: &mut Workbook, vba_file_path: &str) -> Result<(), XlsxError> {
    workbook.add_vba_project(vba_file_path)?;
    Ok(())
}
```

---

### 8.2 导出VBA二进制流

**说明：** 需要使用专门的工具提取VBA代码

**实现建议：**
- 使用 `ole` 等库解析xlsx文件
- 提取 `vbaProject.bin` 文件

---

## 九、Diff计算与变更追踪

### 9.1 操作前后的差异对比

**功能描述：** 记录写入操作前后的单元格变更

**实现方案：**
1. 操作前使用 `calamine` 读取原始数据
2. 执行写入操作
3. 操作后读取修改后的数据
4. 对比生成差异报告

**实现示例：**
```rust
use calamine::{Reader, Xlsx, open_workbook};
use rust_xlsxwriter::{Workbook, XlsxError};

struct CellDiff {
    row: u32,
    col: u16,
    old_value: String,
    new_value: String,
}

fn write_with_diff(
    input_path: &str,
    output_path: &str,
    sheet_name: &str,
    row: u32,
    col: u16,
    new_value: &str
) -> Result<Vec<CellDiff>, XlsxError> {
    // 1. 读取原始数据
    let mut workbook: Xlsx<_> = open_workbook(input_path).unwrap();
    let range = workbook.worksheet_range(sheet_name).unwrap();

    // 2. 记录原始值
    let old_value = match range.get_value((row as usize, col as usize)) {
        Some(calamine::Cell::String(s)) => s.clone(),
        Some(calamine::Cell::Float(f)) => f.to_string(),
        _ => String::new()
    };

    // 3. 创建新工作簿并修改
    let mut new_workbook = Workbook::new();
    let worksheet = new_workbook.add_worksheet().set_name(sheet_name)?;

    // 复制原数据
    for (row_idx, row_data) in range.rows().enumerate() {
        for (col_idx, cell) in row_data.iter().enumerate() {
            match cell {
                calamine::Cell::String(s) => {
                    worksheet.write_string(row_idx as u32, col_idx as u16, s)?;
                },
                calamine::Cell::Float(f) => {
                    worksheet.write_number(row_idx as u32, col_idx as u16, *f)?;
                },
                _ => {}
            }
        }
    }

    // 写入新值
    worksheet.write_string(row, col, new_value)?;

    // 4. 保存新文件
    new_workbook.save(output_path)?;

    // 5. 返回差异
    Ok(vec![CellDiff {
        row,
        col,
        old_value,
        new_value: new_value.to_string()
    }])
}
```

---

## 十、安全机制集成

### 10.1 文件备份

**实现示例：**
```rust
use std::fs;
use std::path::Path;

fn create_backup(filepath: &str) -> Result<String, std::io::Error> {
    let path = Path::new(filepath);
    let stem = path.file_stem().unwrap().to_str().unwrap();
    let ext = path.extension().unwrap().to_str().unwrap();
    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
    let backup_path = format!("{}_backup_{}.{}", stem, timestamp, ext);

    fs::copy(filepath, &backup_path)?;
    Ok(backup_path)
}
```

---

### 10.2 文件指纹计算

**实现示例：**
```rust
use std::fs::File;
use std::io::Read;
use std::path::Path;
use sha2::{Sha256, Digest};

fn calculate_file_hash(filepath: &str) -> Result<String, std::io::Error> {
    let mut file = File::open(filepath)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 8192];

    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }

    Ok(format!("{:x}", hasher.finalize()))
}
```

---

### 10.3 Dry-run 预执行

**实现示例：**
```rust
use rust_xlsxwriter::{Workbook, XlsxError};

fn dry_run_operation(workbook: &mut Workbook) -> Result<Vec<u8>, XlsxError> {
    // 预执行模式，不保存到文件
    workbook.save_to_buffer()
}

fn validate_operation(buffer: &[u8]) -> bool {
    // 验证生成的文件是否符合预期
    buffer.len() > 0
}
```

---

## 十一、综合实现示例

### 完整的单元格写入流程（带安全机制）

```rust
use calamine::{Reader, Xlsx, open_workbook};
use rust_xlsxwriter::{Workbook, XlsxError};
use std::fs;

struct WriteResult {
    success: bool,
    message: String,
    backup_path: Option<String>,
    old_hash: Option<String>,
    new_hash: Option<String>,
    diff: Vec<CellDiff>,
}

fn safe_write_cell(
    input_path: &str,
    output_path: &str,
    sheet_name: &str,
    row: u32,
    col: u16,
    new_value: &str,
    dry_run: bool
) -> Result<WriteResult, XlsxError> {
    // 1. 计算原始文件哈希
    let old_hash = calculate_file_hash(input_path)?;

    // 2. 读取原始数据
    let mut workbook: Xlsx<_> = open_workbook(input_path).unwrap();
    let range = workbook.worksheet_range(sheet_name)?;

    // 3. 记录原始值
    let old_value = match range.get_value((row as usize, col as usize)) {
        Some(calamine::Cell::String(s)) => s.clone(),
        Some(calamine::Cell::Float(f)) => f.to_string(),
        _ => String::new()
    };

    // 4. 创建新工作簿
    let mut new_workbook = Workbook::new();
    let worksheet = new_workbook.add_worksheet().set_name(sheet_name)?;

    // 复制原数据
    for (row_idx, row_data) in range.rows().enumerate() {
        for (col_idx, cell) in row_data.iter().enumerate() {
            match cell {
                calamine::Cell::String(s) => {
                    worksheet.write_string(row_idx as u32, col_idx as u16, s)?;
                },
                calamine::Cell::Float(f) => {
                    worksheet.write_number(row_idx as u32, col_idx as u16, *f)?;
                },
                _ => {}
            }
        }
    }

    // 写入新值
    worksheet.write_string(row, col, new_value)?;

    // 5. Dry-run 模式
    if dry_run {
        let buffer = new_workbook.save_to_buffer()?;
        return Ok(WriteResult {
            success: true,
            message: "Dry-run completed successfully".to_string(),
            backup_path: None,
            old_hash: Some(old_hash),
            new_hash: None,
            diff: vec![CellDiff {
                row,
                col,
                old_value,
                new_value: new_value.to_string()
            }]
        });
    }

    // 6. 创建备份
    let backup_path = create_backup(input_path)?;

    // 7. 保存新文件
    new_workbook.save(output_path)?;

    // 8. 计算新文件哈希
    let new_hash = calculate_file_hash(output_path)?;

    // 9. 返回结果
    Ok(WriteResult {
        success: true,
        message: "Cell written successfully".to_string(),
        backup_path: Some(backup_path),
        old_hash: Some(old_hash),
        new_hash: Some(new_hash),
        diff: vec![CellDiff {
            row,
            col,
            old_value,
            new_value: new_value.to_string()
        }]
    })
}
```

---

## 十二、总结与建议

### 核心API映射总结

| 功能类别 | 核心API | 文件位置 |
|---------|---------|----------|
| 创建工作簿 | `Workbook::new()` | `src/workbook.rs` |
| 保存文件 | `Workbook::save()` | `src/workbook.rs` |
| 添加工作表 | `Workbook::add_worksheet()` | `src/workbook.rs` |
| 写入单元格 | `Worksheet::write()` | `src/worksheet.rs` |
| 写入公式 | `Worksheet::write_formula()` | `src/worksheet.rs` |
| 设置格式 | `Format::new()` + 各种set方法 | `src/format.rs` |
| 合并单元格 | `Worksheet::merge_range()` | `src/worksheet.rs` |
| 添加图表 | `Chart::new()` + `Worksheet::insert_chart()` | `src/chart.rs` |
| 导入VBA | `Workbook::add_vba_project()` | `src/macros.rs` |

### 实现建议

1. **分层架构：** 严格按照设计文档的两层架构，核心能力层做薄封装
2. **原子化设计：** 每个函数对应一个具体的Excel操作
3. **安全优先：** 所有写操作都应集成备份、哈希校验、dry-run机制
4. **错误处理：** 统一使用 `Result<T, XlsxError>` 处理错误
5. **性能优化：** 对于大文件使用常量内存模式
6. **类型安全：** 充分利用Rust的类型系统确保API安全

### 技术限制说明

1. **无法直接修改：** `rust_xlsxwriter` 是纯写入库，修改操作需要读取后重新生成
2. **删除工作表：** 需要特殊处理，通过重新生成文件实现
3. **数据透视表：** 当前版本不支持，需要变通方案
4. **VBA执行：** 仅支持二进制导入导出，不执行VBA代码

### 后续扩展方向

1. 集成更多数据处理库（polars、datafusion）
2. 支持更多Excel高级功能
3. 优化大文件处理性能
4. 增强Diff计算能力
5. 支持更多文件格式

---

## 参考文档

- rust_xlsxwriter 官方文档：https://rustxlsxwriter.github.io
- rust_xlsxwriter GitHub：https://github.com/jmcnamara/rust_xlsxwriter
- calamine 库：https://github.com/tafia/calamine