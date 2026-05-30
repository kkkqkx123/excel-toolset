# Calamine 架构设计

## 总体架构

Calamine 采用分层架构设计，从底层的文件格式解析到上层的统一 API 抽象，形成一个清晰的层次结构。

```
┌─────────────────────────────────────────────────────────────┐
│                        用户 API 层                            │
│  - Reader Trait                                              │
│  - 公共类型定义（Data, Range, Cell 等）                        │
│  - 便利函数（open_workbook, open_workbook_auto）              │
└─────────────────────────────────────────────────────────────┘
                              ▲
                              │
┌─────────────────────────────────────────────────────────────┐
│                         自动检测层                            │
│  - Sheets 枚举（统一封装所有格式）                             │
│  - 文件格式自动检测                                            │
│  - 运行时格式选择                                              │
└─────────────────────────────────────────────────────────────┘
                              ▲
                              │
┌─────────────────────────────────────────────────────────────┐
│                     反序列化支持层                             │
│  - Serde 集成                                                 │
│  - RangeDeserializer                                         │
│  - 数据类型转换                                                │
└─────────────────────────────────────────────────────────────┘
                              ▲
                              │
┌─────────────────────────────────────────────────────────────┐
│                      格式解析器层                              │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐     │
│  │  XLSX    │  │   XLS    │  │   XLSB   │  │   ODS    │     │
│  │ Reader   │  │ Reader   │  │ Reader   │  │ Reader   │     │
│  └──────────┘  └──────────┘  └──────────┘  └──────────┘     │
└─────────────────────────────────────────────────────────────┘
                              ▲
                              │
┌─────────────────────────────────────────────────────────────┐
│                      底层支持层                               │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐                    │
│  │   VBA    │  │   CFB    │  │ Formats  │                    │
│  │ Parser   │  │  Parser  │  │ Handler  │                    │
│  └──────────┘  └──────────┘  └──────────┘                    │
│  ┌──────────┐  ┌──────────┐                                   │
│  │ DataType │  │  Utils   │                                   │
│  │ Handling │  │ Library  │                                   │
│  └──────────┘  └──────────┘                                   │
└─────────────────────────────────────────────────────────────┘
```

## 核心设计模式

### 1. Trait 抽象（Reader Trait）

`Reader` trait 是整个库的核心抽象，定义了所有格式解析器的统一接口：

```rust
pub trait Reader<RS>: Sized
where
    RS: Read + Seek,
{
    type Error: std::fmt::Debug + From<std::io::Error>;

    fn new(reader: RS) -> Result<Self, Self::Error>;
    fn with_header_row(&mut self, header_row: HeaderRow) -> &mut Self;
    fn vba_project(&mut self) -> Result<Option<VbaProject>, Self::Error>;
    fn metadata(&self) -> &Metadata;
    fn worksheet_range(&mut self, name: &str) -> Result<Range<Data>, Self::Error>;
    fn worksheet_formula(&mut self, _: &str) -> Result<Range<String>, Self::Error>;
    // ... 更多方法
}
```

**设计优势**:
- 统一的 API 接口，用户无需关心底层格式
- 支持泛型，可以处理不同类型的输入源
- 易于扩展新的文件格式

### 2. 枚举多态（Sheets 枚举）

`Sheets` 枚举提供运行时格式检测和统一的访问接口：

```rust
pub enum Sheets<RS> {
    Xls(Xls<RS>),
    Xlsx(Xlsx<RS>),
    Xlsb(Xlsb<RS>),
    Ods(Ods<RS>),
}
```

**设计优势**:
- 运行时动态选择格式
- 实现了 `Reader` trait，可以作为通用类型使用
- 避免了复杂的类型擦除机制

### 3. 零拷贝优化（DataRef）

对于支持懒加载的格式（XLSX、XLSB），提供了 `DataRef` 类型实现零拷贝：

```rust
pub enum DataRef<'a> {
    Int(i64),
    Float(f64),
    StringRef(&'a str),  // 借用字符串，避免拷贝
    // ...
}
```

**设计优势**:
- 减少内存分配
- 提高大文件处理性能
- 保持 API 的一致性

### 4. 错误处理层次

采用分层错误处理，每个模块有特定的错误类型，顶层提供统一错误：

```rust
pub enum Error {
    Io(std::io::Error),
    Ods(OdsError),
    Xls(XlsError),
    Xlsb(XlsbError),
    Xlsx(XlsxError),
    Vba(VbaError),
    De(DeError),
    Msg(&'static str),
}
```

**设计优势**:
- 详细的错误信息
- 保持错误上下文
- 便于用户调试

## 关键组件设计

### 1. 数据表示层

#### Data 枚举
表示单元格的所有可能数据类型：

```rust
pub enum Data {
    Int(i64),
    Float(f64),
    String(String),
    Bool(bool),
    DateTime(ExcelDateTime),
    DateTimeIso(String),
    DurationIso(String),
    Error(CellErrorType),
    Empty,
}
```

#### Range 结构
表示工作表中的单元格范围：

```rust
pub struct Range<T> {
    start: (u32, u32),
    end: (u32, u32),
    inner: Vec<Vec<T>>,
}
```

**设计特点**:
- 泛型设计，支持 `Data` 和 `DataRef`
- 提供迭代器接口
- 支持索引访问

### 2. 格式解析器设计

#### XLSX 解析器
- 基于 ZIP 容器和 XML 解析
- 支持懒加载
- 使用共享字符串表优化内存

#### XLS 解析器
- 基于 CFB（复合文件二进制）格式
- 需要完整的文件读取
- 包含 VBA 项目解析

#### XLSB 解析器
- 二进制 XML 格式
- 支持懒加载
- 性能优于 XLSX

#### ODS 解析器
- 基于 OpenDocument 标准
- ZIP 容器 + XML 格式
- 与 XLSX 架构类似

### 3. VBA 项目解析

VBA 解析模块独立设计，可被 XLS、XLSX、XLSB 等格式共享：

```rust
pub struct VbaProject {
    references: Vec<Reference>,
    modules: BTreeMap<String, Vec<u8>>,
    encoding: XlsEncoding,
}
```

**设计优势**:
- 代码复用
- 独立的错误处理
- 统一的接口

## 性能优化设计

### 1. 懒加载机制

XLSX 和 XLSB 支持懒加载：

- 按需读取单元格数据
- 共享字符串表的智能缓存
- 减少内存占用

实现细节：
- `cells_reader` 模块提供迭代器接口
- 使用 `Range<DataRef>` 避免字符串拷贝
- ZIP 路径缓存优化

### 2. 缓存策略

- ZIP 路径缓存：避免重复解析关系文件
- 共享字符串表缓存：减少字符串查找开销
- 格式信息缓存：避免重复解析格式

### 3. 零拷贝设计

对于大文件，通过引用传递减少数据拷贝：

```rust
fn worksheet_range_ref<'a>(
    &'a mut self,
    name: &str
) -> Result<Range<DataRef<'a>>, Self::Error>
```

### 4. SIMD 优化

使用 `atoi_simd` 库加速整数解析，提高大数据量处理性能。

## 扩展性设计

### 1. 新增格式支持

添加新的文件格式只需：

1. 实现 `Reader` trait
2. 定义特定的错误类型
3. 在 `Sheets` 枚举中添加变体
4. 在 `auto` 模块中添加检测逻辑

### 2. 自定义数据类型

通过 `DataType` trait 支持自定义数据类型：

```rust
pub trait DataType {
    fn is_empty(&self) -> bool;
    fn is_int(&self) -> bool;
    // ... 更多判断方法
}
```

### 3. Serde 集成

通过 `RangeDeserializer` 实现灵活的反序列化：

```rust
pub struct RangeDeserializer<'a, 'de, R> {
    range: &'a Range<Data>,
    headers: Headers<'a, String>,
    row_index: u32,
    marker: PhantomData<R>,
}
```

## 内存管理

### 1. 所有者语义

严格遵循 Rust 的所有权规则，确保：
- 无内存泄漏
- 无数据竞争
- 自动内存释放

### 2. 借用检查

通过生命周期参数确保引用的有效性：
```rust
pub struct DataRef<'a> {
    // 借用数据的生命周期为 'a
}
```

### 3. 大文件处理

对于超大文件：
- 支持流式读取
- 懒加载机制
- 及时释放已读取的数据

## 并发安全

当前设计主要针对单线程场景，但为并发安全提供了基础：

- 数据结构设计上避免内部可变性
- 使用不可变引用传递数据
- 为未来并发扩展预留空间

## 测试策略

### 1. 单元测试
- 每个模块都有对应的测试
- 覆盖主要代码路径

### 2. 集成测试
- `tests/` 目录包含真实文件测试
- 覆盖各种格式的边界情况

### 3. 性能测试
- 使用 `criterion` 进行基准测试
- 监控性能回归

## 文档和示例

### 1. 代码文档
- 完整的 Rustdoc 注释
- 示例代码
- 类型说明

### 2. README 文档
- 快速开始指南
- 常见用法示例
- 特性说明

### 3. 测试文件
- `examples/` 目录包含实用示例
- `tests/` 目录包含功能测试