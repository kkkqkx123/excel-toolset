# XLSX 模块详解

## 概述

XLSX 模块负责解析基于 XML 的 Excel 2007+ 格式（`.xlsx`、`.xlsm`、`.xlam`）。这是最常用的现代 Excel 格式。

## 模块结构

```
src/xlsx/
├── mod.rs          # 主模块，定义 Xlsx 结构体和错误类型
└── cells_reader.rs # 单元格读取器，支持懒加载
```

## 核心组件

### Xlsx 结构体

XLSX 格式的主要读取器实现。

```rust
pub struct Xlsx<RS>
where
    RS: Read + Seek,
{
    zip: ZipArchive<RS>,
    strings: Option<Vec<String>>,
    relationships: HashMap<String, Vec<(String, String)>>,
    sheets_metadata: Vec<Sheet>,
    workbook: Option<XmlReader<BufReader<ZipFile<'static, RS>>>>,
    workbook_rels: HashMap<String, Vec<(String, String)>>,
    defined_names: Vec<(String, String)>,
    // ...
}
```

### XlsxError 错误类型

XLSX 特定的错误类型：

```rust
pub enum XlsxError {
    Io(std::io::Error),
    Zip(zip::result::ZipError),
    Vba(crate::vba::VbaError),
    Xml(quick_xml::Error),
    XmlAttr(quick_xml::events::attributes::AttrError),
    Parse(std::string::ParseError),
    ParseFloat(std::num::ParseFloatError),
    ParseInt(std::num::ParseIntError),
    XmlEof(&'static str),
    UnexpectedNode(&'static str),
    FileNotFound(String),
    RelationshipNotFound,
    Alphanumeric(u8),
    NumericColumn(u8),
    MissingColumn(&'static str),
    InvalidDimension(String),
    InvalidSheetPath,
    InvalidSharedString(u32),
    WorkbookFileNotFound,
}
```

## 文件格式解析

### XLSX 文件结构

XLSX 文件本质上是一个 ZIP 容器，包含以下关键文件：

```
workbook.xlsx
├── [Content_Types].xml      # 内容类型定义
├── _rels/                    # 关系文件夹
│   └── .rels                 # 包级别关系
├── xl/                       # 主要内容文件夹
│   ├── workbook.xml          # 工作簿定义
│   ├── _rels/
│   │   └── workbook.xml.rels # 工作簿关系
│   ├── worksheets/           # 工作表文件夹
│   │   ├── sheet1.xml
│   │   ├── sheet2.xml
│   │   └── ...
│   ├── sharedStrings.xml     # 共享字符串表
│   ├── styles.xml            # 样式定义
│   └── theme/
└── docProps/                 # 文档属性
    ├── app.xml
    └── core.xml
```

### 解析流程

1. **打开 ZIP 容器**
   ```rust
   let zip = ZipArchive::new(reader)?;
   ```

2. **解析关系文件**
   - 解析 `_rels/.rels` 获取工作簿位置
   - 解析 `xl/_rels/workbook.xml.rels` 获取工作表和共享字符串表位置

3. **解析工作簿文件**
   - 读取工作表元数据（名称、类型、可见性）
   - 读取定义的名称（defined names）

4. **读取共享字符串表**
   - 优化内存：所有字符串只存储一次
   - 单元格引用字符串索引

5. **按需加载工作表**
   - 用户请求时才读取工作表 XML
   - 使用 `cells_reader` 流式解析

## 单元格读取

### cells_reader 模块

提供高效的单元格流式读取器：

```rust
pub struct XlsxCellReader<'a, RS> {
    zip: &'a mut ZipArchive<RS>,
    shared_strings: &'a [String],
    zip_path_cache: HashMap<String, usize>,
    formats: &'a BTreeMap<usize, String>,
}
```

#### 支持的单元格数据类型

- **数字**：`<v>` 标签
- **共享字符串**：通过索引引用 `sharedStrings.xml`
- **内联字符串**：`<is>` 标签
- **布尔值**：`<v>` 标签（1 或 0）
- **错误**：`<v>` 标签（#DIV/0!、#N/A 等）
- **公式**：`<f>` 标签

#### 读取模式

1. **全量读取**：`worksheet_range()` 返回 `Range<Data>`
2. **懒加载读取**：`worksheet_range_ref()` 返回 `Range<DataRef>`，字符串使用借用

## 关键功能

### 懒加载支持

XLSX 模块实现了 `ReaderRef` trait，支持零拷贝读取：

```rust
impl<RS> ReaderRef<RS> for Xlsx<RS>
where
    RS: Read + Seek,
{
    fn worksheet_range_ref<'a>(
        &'a mut self,
        name: &str
    ) -> Result<Range<DataRef<'a>>, Self::Error> {
        // 返回 DataRef，字符串是借用的
    }
}
```

**优势**：
- 减少内存分配
- 提高大文件处理速度
- 避免字符串重复复制

### 公式读取

支持读取单元格公式：

```rust
let formulas = workbook.worksheet_formula("Sheet1")?;
for row in formulas.rows() {
    for formula in row {
        if !formula.is_empty() {
            println!("Formula: {}", formula);
        }
    }
}
```

### VBA 项目支持

读取嵌入的 VBA 宏代码：

```rust
if let Ok(Some(vba)) = workbook.vba_project() {
    let module = vba.get_module("Module1")?;
    println!("VBA Code: {}", module);
}
```

### 定义名称（Defined Names）

读取工作簿中定义的名称：

```rust
for (name, formula) in workbook.defined_names() {
    println!("Name: {}, Formula: {}", name, formula);
}
```

## 性能优化

### 1. ZIP 路径缓存

避免重复解析关系文件：

```rust
let zip_path_cache = build_zip_path_cache(&mut zip, &relationships)?;
```

### 2. 共享字符串表

所有字符串只存储一次，单元格通过索引引用：

```rust
let shared_strings = parse_shared_strings(&mut zip)?;
let cell_value = shared_strings[index as usize].clone();
```

### 3. 流式解析

使用 `cells_reader` 逐步读取单元格，避免一次性加载整个工作表：

```rust
let mut reader = XlsxCellReader::new(&mut self.zip, ...);
while let Some(cell) = reader.next_cell()? {
    // 处理单个单元格
}
```

## 限制

### 最大行列数

```rust
pub const MAX_ROWS: u32 = 1_048_576;    // 最大行数
pub const MAX_COLUMNS: u32 = 16_384;    // 最大列数
```

### 不支持的功能

- 写入/修改 Excel 文件（只读库）
- 图表、图片的完全解析（仅支持读取原始数据）
- 复杂样式和格式（仅支持基本格式识别）
- 数据验证、条件格式等高级特性

## 使用示例

### 基本读取

```rust
use calamine::{open_workbook, Xlsx, Reader};

let mut workbook: Xlsx<_> = open_workbook("data.xlsx")?;
let range = workbook.worksheet_range("Sheet1")?;

for row in range.rows() {
    for cell in row {
        println!("{:?}", cell);
    }
}
```

### 懒加载读取

```rust
use calamine::{open_workbook, Reader, ReaderRef};

let mut workbook: Xlsx<_> = open_workbook("large_file.xlsx")?;
let range = workbook.worksheet_range_ref("Sheet1")?;

for row in range.rows() {
    for cell in row {
        // cell 是 DataRef，字符串是借用的，零拷贝
        println!("{:?}", cell);
    }
}
```

### 读取公式

```rust
let mut workbook: Xlsx<_> = open_workbook("formulas.xlsx")?;
let formulas = workbook.worksheet_formula("Sheet1")?;

for row in formulas.rows() {
    println!("Row: {:?}", row);
}
```

### 读取 VBA

```rust
let mut workbook: Xlsx<_> = open_workbook("macro.xlsm")?;

if let Ok(Some(vba)) = workbook.vba_project() {
    for (name, module) in vba.get_modules() {
        println!("Module: {}", name);
        println!("Code: {}", String::from_utf8_lossy(module));
    }
}
```

## 错误处理

XLSX 模块提供了详细的错误信息，便于调试：

```rust
use calamine::open_workbook;

match open_workbook::<Xlsx<_>, _>("file.xlsx") {
    Ok(workbook) => {
        // 成功
    }
    Err(e) => {
        match e {
            XlsxError::Io(e) => eprintln!("IO 错误: {}", e),
            XlsxError::Zip(e) => eprintln!("ZIP 错误: {}", e),
            XlsxError::Xml(e) => eprintln!("XML 解析错误: {}", e),
            XlsxError::FileNotFound(f) => eprintln!("文件未找到: {}", f),
            _ => eprintln!("其他错误: {}", e),
        }
    }
}
```

## 与其他格式的对比

| 特性 | XLSX | XLS | XLSB | ODS |
|------|------|-----|------|-----|
| 格式 | XML + ZIP | 二进制 + CFB | 二进制 + ZIP | XML + ZIP |
| 懒加载 | 支持 | 不支持 | 支持 | 不支持 |
| 大小 | 较大 | 较小 | 最小 | 较大 |
| 解析速度 | 中等 | 慢 | 快 | 中等 |
| VBA 支持 | 支持 | 支持 | 支持 | 不支持 |

## 总结

XLSX 模块提供了完整且高效的 XLSX 文件解析功能：

- 支持懒加载和零拷贝读取
- 完善的错误处理
- 支持 VBA 项目、公式、定义名称等高级特性
- 性能优化适合处理大文件
- 清晰的 API 设计易于使用