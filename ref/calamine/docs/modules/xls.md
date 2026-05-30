# XLS 模块详解

## 概述

XLS 模块负责解析传统的 Excel 97-2003 二进制格式（`.xls`、`.xla`）。这是一个基于复合文件二进制（CFB）格式的旧版 Excel 格式。

## 模块结构

```
src/xls.rs
```

XLS 模块是一个单一文件模块，包含所有 XLS 解析相关的代码。

## 核心组件

### Xls 结构体

XLS 格式的主要读取器实现。

```rust
pub struct Xls<RS>
where
    RS: Read + Seek,
{
    cfb: Cfb,
    strings: Vec<String>,
    formats: BTreeMap<usize, String>,
    xf_records: Vec<XfRecord>,
    cells: BTreeMap<String, Range<Data>>,
    sheets: Vec<Sheet>,
    defined_names: Vec<(String, String)>,
    // ...
}
```

### XlsError 错误类型

XLS 特定的错误类型：

```rust
pub enum XlsError {
    Io(std::io::Error),
    Cfb(crate::cfb::CfbError),
    Vba(crate::vba::VbaError),
    StackLen,
    Unrecognized {
        typ: &'static str,
        val: u8,
    },
    Password,
    Len {
        expected: usize,
        found: usize,
        typ: &'static str,
    },
    ContinueRecordTooShort,
    EoStream(&'static str),
    InvalidFormula {
        stack_size: usize,
    },
    IfTab(usize),
    Etpg(u8),
    NoVba,
    #[cfg(feature = "picture")]
    Art(&'static str),
    WorksheetNotFound(String),
    InvalidFormat {
        ifmt: u16,
    },
}
```

### XlsOptions 配置选项

```rust
pub struct XlsOptions {
    // XLS 特定的配置选项
}
```

## 文件格式解析

### XLS 文件结构

XLS 文件基于复合文件二进制（CFB）格式，包含以下关键流：

```
workbook.xls (CFB 容器)
├── \x05SummaryInformation    # 文档摘要信息
├── \x05DocumentSummaryInformation
├── Workbook                  # 主工作簿流
├── _VBA_PROJECT_CUR          # VBA 项目（如果有）
└── ...
```

### Workbook 流结构

Workbook 流由一系列记录（Record）组成，每个记录包含类型和长度：

```
BOF (Beginning of File)
  ├─ Workbook Globals
  │   ├─ WINDOW
  │   ├─ FONT
  │   ├─ FORMAT
  │   ├─ XF (Extended Format)
  │   ├─ STYLE
  │   ├─ SST (Shared String Table)
  │   └─ ...
  ├─ Sheet (多个)
  │   ├─ BOUNDSHEET
  │   ├─ ...
  │   ├─ ROW
  │   ├─ CELL
  │   ├─ VALUE
  │   └─ EOF
  └─ EOF
```

### 解析流程

1. **打开 CFB 容器**
   ```rust
   let mut cfb = Cfb::new(&mut reader, len)?;
   ```

2. **解析 Workbook 流**
   - 读取 BOF 记录验证文件类型
   - 解析全局信息（字体、格式、样式）
   - 解析共享字符串表（SST）
   - 解析工作表边界（BOUNDSHEET）

3. **解析工作表数据**
   - 解析行记录（ROW）
   - 解析单元格记录（LABEL、NUMBER、RK 等）
   - 解析公式记录（FORMULA）
   - 应用格式信息

4. **解析 VBA 项目**
   - 从 CFB 中提取 `_VBA_PROJECT_CUR` 流
   - 使用 VBA 模块解析

## 单元格数据类型

### 支持的记录类型

| 记录 ID | 记录名称 | 数据类型 |
|---------|----------|----------|
| 0x0006 | FORMULA | 公式 |
| 0x0007 | STRING | 字符串（公式结果） |
| 0x00BE | SHRFMLA | 共享公式 |
| 0x00FD | LABELSST | 共享字符串标签 |
| 0x0203 | NUMBER | 数字 |
| 0x0205 | BOOLERR | 布尔值或错误 |
| 0x027E | RK | RK 编码的数字 |
| 0x00BD | MULRK | 多个 RK 编码的数字 |
| 0x0204 | LABEL | 标签（字符串） |
| 0x00FC | LABELRST | RString 标签 |

### 数据解析

```rust
// NUMBER 记录：8 字节 IEEE 754 双精度浮点数
let value = read_f64(data);

// BOOLERR 记录：1 字节类型 + 1 字节值
let is_bool = data[0] == 0;
let value = data[1];

// RK 记录：4 字节压缩浮点数
let value = parse_rk(data);
```

## 公式解析

### 公式记录结构

```rust
pub struct Formula {
    row: u32,
    col: u32,
    ixfe: u16,        // 格式索引
    result: Data,     // 计算结果
    options: u16,
    tokens: Vec<u8>,  // 公式令牌
}
```

### 公式令牌解析

XLS 公式使用基于栈的令牌表示法。解析流程：

1. 解析公式令牌字节流
2. 使用栈进行表达式求值
3. 处理各种令牌类型（操作符、函数、引用等）

### 公式结果读取

公式单元格可能有两种结果表示：

1. **缓存结果**：直接存储在 FORMULA 记录中
2. **字符串结果**：存储在后续的 STRING 记录中

## 关键功能

### 格式处理

```rust
pub struct XfRecord {
    font: u16,
    format: u16,
    // ... 其他格式属性
}
```

格式信息用于确定单元格的显示格式（日期、时间、数字等）。

### 共享字符串表

XLS 使用共享字符串表（SST）存储重复的字符串，类似于 XLSX 的共享字符串表。

```rust
pub struct SharedStringTable {
    strings: Vec<String>,
}
```

### 定义名称（Defined Names）

解析工作簿中定义的名称：

```rust
pub struct DefinedName {
    name: String,
    formula: String,
}
```

## 性能考虑

### 内存占用

XLS 格式不支持懒加载，所有工作表数据在打开时加载到内存：

```rust
pub fn new(reader: RS) -> Result<Self, Self::Error> {
    // 加载所有工作表数据
    for sheet in &sheets {
        let data = self.load_sheet(sheet)?;
        self.cells.insert(sheet.name.clone(), data);
    }
}
```

**影响**：
- 大文件可能占用大量内存
- 不适合处理超大工作簿
- 加载时间较长

### 优化策略

1. **共享字符串缓存**：避免重复字符串
2. **按需格式解析**：只在需要时解析格式信息
3. **选择性加载**：只加载用户请求的工作表（在后续版本中可能实现）

## 限制

### 最大行列数

XLS 格式的理论限制：
- 最大行数：65,536
- 最大列数：256

### 不支持的功能

- 写入/修改 XLS 文件（只读库）
- 懒加载（全量加载）
- 超过 65,536 行或 256 列的工作表
- 密码保护的工作簿
- 某些高级公式特性
- 图片、图表等嵌入对象

## 使用示例

### 基本读取

```rust
use calamine::{open_workbook, Xls, Reader};

let mut workbook: Xls<_> = open_workbook("data.xls")?;
let range = workbook.worksheet_range("Sheet1")?;

for row in range.rows() {
    for cell in row {
        println!("{:?}", cell);
    }
}
```

### 读取公式

```rust
let mut workbook: Xls<_> = open_workbook("formulas.xls")?;
let formulas = workbook.worksheet_formula("Sheet1")?;

for row in formulas.rows() {
    println!("Row: {:?}", row);
}
```

### 读取 VBA

```rust
let mut workbook: Xls<_> = open_workbook("macro.xls")?;

if let Ok(Some(vba)) = workbook.vba_project() {
    for (name, module) in vba.get_modules() {
        println!("Module: {}", name);
        println!("Code: {}", String::from_utf8_lossy(module));
    }
}
```

### 获取工作表元数据

```rust
let mut workbook: Xls<_> = open_workbook("data.xls")?;

for sheet in workbook.sheets_metadata() {
    println!("Name: {}", sheet.name);
    println!("Type: {:?}", sheet.typ);
    println!("Visible: {:?}", sheet.visible);
}
```

## 错误处理

```rust
use calamine::open_workbook;

match open_workbook::<Xls<_>, _>("file.xls") {
    Ok(workbook) => {
        // 成功
    }
    Err(e) => {
        match e {
            XlsError::Io(e) => eprintln!("IO 错误: {}", e),
            XlsError::Cfb(e) => eprintln!("CFB 错误: {}", e),
            XlsError::Password => eprintln!("工作簿受密码保护"),
            XlsError::WorksheetNotFound(name) => {
                eprintln!("工作表未找到: {}", name)
            }
            XlsError::Unrecognized { typ, val } => {
                eprintln!("无法识别的 {}: 0x{:02X}", typ, val)
            }
            _ => eprintln!("其他错误: {}", e),
        }
    }
}
```

## 与 XLSX 的对比

| 特性 | XLS | XLSX |
|------|-----|------|
| 格式 | 二进制 + CFB | XML + ZIP |
| 最大行数 | 65,536 | 1,048,576 |
| 最大列数 | 256 | 16,384 |
| 懒加载 | 不支持 | 支持 |
| 文件大小 | 较小 | 较大 |
| 解析速度 | 慢 | 中等 |
| VBA 支持 | 支持 | 支持 |
| 兼容性 | 旧版 Excel | 新版 Excel |

## 总结

XLS 模块提供了完整的 XLS 文件解析功能：

- 完整支持 XLS 97-2003 格式
- 支持 VBA 项目、公式、定义名称
- 清晰的错误处理
- 与 XLSX 模块统一的 API 接口

**注意事项**：
- 不支持懒加载，大文件可能占用大量内存
- 行列数限制较低（65,536 × 256）
- 建议对新项目使用 XLSX 格式

XLS 模块主要用于需要兼容旧版 Excel 文件的场景。对于新项目，建议使用 XLSX 或 XLSB 格式以获得更好的性能和更大的容量。