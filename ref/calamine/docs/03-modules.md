# Calamine 模块功能详解

## 模块概览

Calamine 项目包含多个模块，每个模块负责特定的功能。本文档详细说明各个模块的职责和功能。

## 核心模块

### lib.rs - 库入口

**文件路径**: `src/lib.rs`

**主要职责**:
- 定义公共 API
- 导出所有公共类型和 trait
- 实现 Reader 和 ReaderRef trait
- 提供便利函数（如 `open_workbook`）

**主要类型和 trait**:
- `Reader<R>`: 统一的电子表格读取接口
- `ReaderRef<R>`: 支持零拷贝的读取接口
- `Data`: 单元格数据类型枚举
- `Range<T>`: 单元格范围数据结构
- `Cell`: 单元格位置和值
- `Metadata`: 工作簿元数据
- `Sheet`: 工作表元数据
- `CellErrorType`: 单元格错误类型
- `SheetType`: 工作表类型
- `SheetVisible`: 工作表可见性
- `Dimensions`: 范围维度信息

**便利函数**:
- `open_workbook<R, P>(path: P) -> Result<R, R::Error>`: 打开特定格式的工作簿
- `open_workbook_from_rs<R, RS>(rs: RS) -> Result<R, R::Error>`: 从 Read + Seek 源打开工作簿

### datatype.rs - 数据类型定义

**文件路径**: `src/datatype.rs`

**主要职责**:
- 定义所有可能的单元格数据类型
- 提供数据类型判断和转换方法
- 处理 Excel 日期时间格式

**主要类型**:

#### Data 枚举
表示单元格的所有可能数据类型：
- `Int(i64)`: 整数
- `Float(f64)`: 浮点数
- `String(String)`: 字符串
- `Bool(bool)`: 布尔值
- `DateTime(ExcelDateTime)`: Excel 日期时间
- `DateTimeIso(String)`: ISO 8601 格式的日期时间
- `DurationIso(String)`: ISO 8601 格式的持续时间
- `Error(CellErrorType)`: 单元格错误
- `Empty`: 空单元格

#### DataRef 枚举
用于零拷贝的引用类型（主要用于 XLSX 和 XLSB）：
- `StringRef(&'a str)`: 字符串引用，避免拷贝

#### ExcelDateTime
表示 Excel 日期时间值，支持：
- 1900 和 1904 日期系统
- 日期、时间、日期时间三种类型

#### DataType Trait
提供数据类型判断方法：
- `is_empty()`: 是否为空
- `is_int()`: 是否为整数
- `is_float()`: 是否为浮点数
- `is_string()`: 是否为字符串
- `is_bool()`: 是否为布尔值
- `is_datetime()`: 是否为日期时间
- `is_error()`: 是否为错误

## 格式解析器模块

### xlsx - XLSX 格式解析器

**文件路径**: `src/xlsx/mod.rs`, `src/xlsx/cells_reader.rs`

**主要职责**:
- 解析 XLSX 格式文件（Office Open XML）
- 实现 Reader 和 ReaderRef trait
- 支持懒加载
- 读取共享字符串表
- 解析样式和格式
- 读取公式

**关键特性**:
- 基于 ZIP 容器（多个 XML 文件）
- 使用共享字符串表减少内存占用
- 支持流式读取（cells_reader）
- 最大行数：1,048,576
- 最大列数：16,384

**主要结构**:
- `Xlsx<RS>`: XLSX 读取器
- `XlsxError`: XLSX 特定错误类型
- `XlsxCellReader`: 单元格流式读取器

**支持的子模块**:
- `cells_reader`: 流式单元格读取，支持懒加载

### xlsb - XLSB 格式解析器

**文件路径**: `src/xlsb/mod.rs`, `src/xlsb/cells_reader.rs`

**主要职责**:
- 解析 XLSB 格式文件（Excel 二进制格式）
- 实现 Reader 和 ReaderRef trait
- 支持懒加载
- 读取二进制格式的单元格数据

**关键特性**:
- 基于 ZIP 容器（二进制文件）
- 使用二进制格式而非 XML
- 性能优于 XLSX
- 支持流式读取
- 最大行数：1,048,576
- 最大列数：16,384

**主要结构**:
- `Xlsb<RS>`: XLSB 读取器
- `XlsbError`: XLSB 特定错误类型
- `XlsbCellsReader`: 单元格流式读取器

### xls - XLS 格式解析器

**文件路径**: `src/xls.rs`

**主要职责**:
- 解析 XLS 格式文件（传统 Excel 97-2003）
- 实现 Reader trait
- 基于 CFB 格式
- 解析二进制记录

**关键特性**:
- 基于 CFB（复合文件二进制）格式
- 不支持懒加载（必须全量加载）
- 支持 VBA 项目
- 解析 BIFF 记录
- 最大行数：65,536
- 最大列数：256

**主要结构**:
- `Xls<RS>`: XLS 读取器
- `XlsError`: XLS 特定错误类型
- `XlsOptions`: XLS 读取选项

**特殊功能**:
- 支持密码保护检测
- 解析单元格公式
- 读取图片数据（可选 feature）

### ods - ODS 格式解析器

**文件路径**: `src/ods.rs`

**主要职责**:
- 解析 ODS 格式文件（OpenDocument Spreadsheet）
- 实现 Reader trait
- 基于 OpenDocument 标准
- 读取 XML 格式的单元格数据

**关键特性**:
- 基于 ZIP 容器（XML 文件）
- MIME 类型验证
- 支持单元格合并
- 最大行数：1,048,576
- 最大列数：16,384
- 最大单元格数：100,000,000（防止内存耗尽）

**主要结构**:
- `Ods<RS>`: ODS 读取器
- `OdsError`: ODS 特定错误类型

**标准参考**:
- OASIS Open Document Format for Office Application 1.2

## 基础设施模块

### cfb - 复合文件二进制解析器

**文件路径**: `src/cfb.rs`

**主要职责**:
- 解析复合文件二进制（CFB）格式
- 为 XLS 和 VBA 提供底层支持
- 管理扇区、FAT、目录结构

**主要功能**:
- 解析 CFB 头部
- 管理文件分配表（FAT）
- 管理目录结构
- 解码流数据
- 处理编码转换

**主要结构**:
- `Cfb`: CFB 解析器
- `CfbError`: CFB 特定错误类型
- `XlsEncoding`: 编码类型枚举

**应用场景**:
- XLS 文件解析
- VBA 项目解析（vbaProject.bin）

### vba - VBA 项目解析器

**文件路径**: `src/vba.rs`

**主要职责**:
- 解析 VBA 项目文件（vbaProject.bin）
- 提取 VBA 模块代码
- 解析 VBA 引用
- 支持编码转换

**主要功能**:
- 解析 VBA 项目结构
- 读取模块代码
- 读取引用信息
- 处理压缩数据
- 编码转换（支持多种编码）

**主要结构**:
- `VbaProject`: VBA 项目
- `VbaError`: VBA 特定错误类型
- `Reference`: VBA 引用
- `Module`: VBA 模块

**支持的特性**:
- 模块代码提取
- 引用信息获取
- 检查缺失的引用

### formats - 单元格格式处理

**文件路径**: `src/formats.rs`

**主要职责**:
- 识别单元格格式类型
- 格式化 Excel 日期时间值
- 处理内置格式和自定义格式

**主要功能**:
- 检测日期时间格式
- 检测持续时间格式
- 格式化 Excel 数值为日期时间
- 识别内置格式代码

**主要类型**:
- `CellFormat`: 单元格格式类型（Other、DateTime、TimeDelta）

**主要函数**:
- `detect_custom_number_format(format: &str) -> CellFormat`: 检测自定义数字格式
- `builtin_format_by_code(code: u16) -> CellFormat`: 通过代码获取内置格式
- `format_excel_f64(value: f64, ...) -> Data`: 格式化 f64 为 Data
- `format_excel_i64(value: i64, ...) -> Data`: 格式化 i64 为 Data

### utils - 工具函数库

**文件路径**: `src/utils.rs`

**主要职责**:
- 提供通用的二进制读取函数
- XML 处理工具
- 列名转换工具
- ZIP 路径缓存
- 宏定义

**主要功能**:

#### 二进制读取
- `read_u16`, `read_u32`, `read_u64`: 读取无符号整数
- `read_i16`, `read_i32`: 读取有符号整数
- `read_f64`: 读取浮点数
- `read_usize`: 读取 usize

#### 列名转换
- `push_column(col: u32, buf: &mut String)`: 将列号转换为列名（0->A, 1->B...）

#### XML 处理
- `unescape_entity_to_buffer`: 解码 XML 实体
- `unescape_xml`: 解码 XML 字符串
- `build_zip_path_cache`: 构建 ZIP 路径缓存
- `cached_zip_path`: 从缓存获取 ZIP 路径

#### 宏
- `from_err!`: 自动实现 From trait

### de - Serde 反序列化支持

**文件路径**: `src/de.rs`

**主要职责**:
- 实现 Serde 反序列化器
- 支持 Range 到 Rust 结构体的转换
- 提供行级反序列化

**主要功能**:
- 将单元格数据反序列化为 Rust 结构体
- 支持表头行解析
- 处理数据类型转换
- 提供自定义反序列化函数

**主要类型**:
- `RangeDeserializer`: Range 反序列化器
- `RangeDeserializerBuilder`: 反序列化器构建器
- `RowDeserializer`: 行反序列化器
- `DeError`: 反序列化错误类型

**主要函数**:
- `deserialize_as_f64_or_none`: 反序列化为 f64 或 None
- `deserialize_as_f64_or_string`: 反序列化为 f64 或 String

## 便利模块

### auto - 自动格式检测

**文件路径**: `src/auto.rs`

**主要职责**:
- 根据文件扩展名自动选择格式
- 提供运行时格式检测
- 统一封装所有格式

**主要功能**:
- 扩展名识别（.xls, .xlsx, .xlsb, .ods 等）
- 自动尝试各种格式
- 提供统一的 Sheets 枚举

**主要类型**:
- `Sheets<RS>`: 统一的工作簿枚举（Xls、Xlsx、Xlsb、Ods）

**主要函数**:
- `open_workbook_auto<P>(path: P) -> Result<Sheets<BufReader<File>>, Error>`: 自动检测并打开工作簿
- `open_workbook_auto_from_rs<RS>(data: RS) -> Result<Sheets<RS>, Error>`: 从数据源自动检测并打开

### errors - 统一错误处理

**文件路径**: `src/errors.rs`

**主要职责**:
- 提供统一的错误类型
- 封装各模块的特定错误
- 实现错误转换

**主要类型**:
- `Error`: 统一错误枚举
  - `Io(std::io::Error)`: IO 错误
  - `Ods(OdsError)`: ODS 错误
  - `Xls(XlsError)`: XLS 错误
  - `Xlsb(XlsbError)`: XLSB 错误
  - `Xlsx(XlsxError)`: XLSX 错误
  - `Vba(VbaError)`: VBA 错误
  - `De(DeError)`: 反序列化错误
  - `Msg(&'static str)`: 通用错误消息

## 辅助模块

### changelog.rs - 变更日志

**文件路径**: `src/changelog.rs`

**主要职责**:
- 提供版本变更信息
- 记录主要功能更新和 bug 修复

## 模块依赖关系

```
lib.rs (入口)
├── auto (自动检测)
│   ├── xlsx
│   ├── xlsb
│   ├── xls
│   └── ods
├── datatype (数据类型)
├── de (反序列化)
│   └── datatype
├── errors (错误处理)
│   ├── xlsx
│   ├── xlsb
│   ├── xls
│   ├── ods
│   ├── vba
│   └── de
├── formats (格式处理)
│   └── datatype
└── utils (工具函数)

xlsx (XLSX 解析器)
├── cells_reader
├── vba
├── cfb
├── formats
├── datatype
└── utils

xlsb (XLSB 解析器)
├── cells_reader
├── vba
├── formats
├── datatype
└── utils

xls (XLS 解析器)
├── vba
├── cfb
├── formats
├── datatype
└── utils

ods (ODS 解析器)
├── vba
├── formats
├── datatype
└── utils

vba (VBA 解析)
├── cfb
└── utils

cfb (CFB 解析)
└── utils
```

## 扩展点

### 添加新格式

1. 创建新的解析器模块（如 `src/csv.rs`）
2. 实现 `Reader` trait（可选实现 `ReaderRef`）
3. 定义特定错误类型
4. 在 `auto.rs` 中添加格式检测逻辑
5. 在 `errors.rs` 中添加错误封装

### 添加新的数据类型

1. 在 `datatype.rs` 中扩展 `Data` 枚举
2. 更新 `DataType` trait 方法
3. 在 `formats.rs` 中添加格式处理逻辑
4. 更新所有解析器以支持新类型

## 测试文件

- `tests/`: 集成测试目录，包含各种格式的测试文件
- `examples/`: 示例代码目录
- `fuzz/`: 模糊测试目录（针对解析器）

## 总结

Calamine 的模块设计遵循单一职责原则，每个模块专注于特定的功能：
- 格式解析器（xlsx、xlsb、xls、ods）负责具体的文件格式解析
- 基础设施模块（cfb、vba、formats、utils）提供底层支持
- 便利模块（auto、errors）提供统一的接口和错误处理
- 核心模块（lib.rs、datatype）定义公共 API

这种设计使得代码易于维护、扩展和测试。