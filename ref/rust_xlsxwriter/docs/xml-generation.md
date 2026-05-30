# XML 生成模块文档

本文档介绍 XML 文件生成和打包相关的模块，这些模块负责将内部数据结构转换为符合 Office Open XML 标准的文件。

## XML 生成模块列表

- [Packager](#packager) - 文件打包器
- [XMLWriter](#xmlwriter) - XML 写入工具
- [SharedStrings](#sharedstrings) - 共享字符串
- [Styles](#styles) - 样式管理
- [Theme](#theme) - 主题管理
- [App](#app) - 应用属性
- [Core](#core) - 核心属性
- [ContentTypes](#contenttypes) - 内容类型
- [Relationship](#relationship) - 关系管理

## Packager

### 功能

`Packager` 是文件打包的核心组件，负责将所有 XML 文件组装成 xlsx 格式的 ZIP 容器。

### Excel xlsx 文件结构

```
[Content_Types].xml
├── _rels/
│   └── .rels
├── docProps/
│   ├── app.xml
│   └── core.xml
└── xl/
    ├── workbook.xml
    ├── _rels/
    │   └── workbook.xml.rels
    ├── worksheets/
    │   ├── sheet1.xml
    │   ├── sheet2.xml
    │   └── ...
    ├── styles.xml
    ├── theme/
    │   └── theme1.xml
    └── sharedStrings.xml
```

### Packager 结构

```rust
pub struct Packager<W: Write + Seek> {
    zip: ZipWriter<W>,
    zip_options: SimpleFileOptions,
    zip_options_for_binary_files: SimpleFileOptions,
}
```

### 主要功能

1. **创建 ZIP 容器**
2. **添加 XML 文件**
3. **管理文件关系**
4. **处理二进制文件（图片等）**
5. **支持常量内存模式**

### 打包流程

```
1. 创建 ZIP 写入器
2. 写入 [Content_Types].xml
3. 写入关系文件
4. 写入工作簿文件
5. 写入工作表文件
6. 写入共享字符串
7. 写入样式文件
8. 写入主题文件
9. 写入应用属性
10. 写入核心属性
11. 添加图片等二进制文件
12. 完成 ZIP 文件
```

### 常量内存模式

当启用 `constant_memory` feature 时：

```rust
// 使用临时文件存储工作表数据
worksheet.set_tempdir("/tmp");

// Packager 从临时文件读取数据
```

## XMLWriter

### 功能

`XMLWriter` 提供了高效的 XML 写入功能，确保生成的 XML 与 Excel 格式完全一致。

### 核心特性

- 遵循 Excel 的 XML 格式规范
- 正确的字符转义
- 高效的内存使用
- 支持属性写入

### 基本函数

```rust
// XML 声明
xml_declaration(&mut writer);

// 开始标签（无属性）
xml_start_tag_only(&mut writer, "tag");

// 开始标签（带属性）
xml_start_tag(&mut writer, "tag", &[("attr1", "value1"), ("attr2", "value2")]);

// 结束标签
xml_end_tag(&mut writer, "tag");

// 空标签（无属性）
xml_empty_tag_only(&mut writer, "tag");

// 空标签（带属性）
xml_empty_tag(&mut writer, "tag", &[("attr", "value")]);

// 数据元素
xml_data_element_only(&mut writer, "tag", "data");
xml_data_element(&mut writer, "tag", "data", &[("attr", "value")]);
```

### 特殊元素

```rust
// 共享字符串元素（优化）
xml_si_element(&mut writer, "string", preserve_whitespace);

// 富文本元素
xml_rich_si_element(&mut writer, "rich_string");

// 主题元素
xml_theme(&mut writer, theme_xml);
```

### 字符转义

正确处理特殊字符：

- `<` → `&lt;`
- `>` → `&gt;`
- `&` → `&amp;`
- `'` → `&apos;`
- `"` → `&quot;`

### Unicode 逃逸

处理控制字符：

```
_xHHHH_ 格式
```

## SharedStrings

### 功能

`SharedStrings` 管理共享字符串表，优化重复字符串的存储。

### 共享字符串表结构

```xml
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<sst xmlns="..." count="100" uniqueCount="50">
    <si><t>String 1</t></si>
    <si><t>String 2</t></si>
    <si><t><r>Rich <rPr><b/><rPr><t>Text</t></r></t></si>
    ...
</sst>
```

### 内部结构

```rust
pub struct SharedStrings {
    writer: Cursor<Vec<u8>>,
}
```

### 共享字符串表

```rust
pub struct SharedStringsTable {
    strings: HashMap<String, usize>,
    insertion_order: Vec<String>,
    count: usize,
    unique_count: usize,
}
```

### 字符串去重

```rust
// 添加字符串到共享表
let index = shared_strings_table.add_string("Hello");
let index = shared_strings_table.add_string("World");

// 重复字符串返回相同索引
let index1 = shared_strings_table.add_string("Hello"); // 返回 0
let index2 = shared_strings_table.add_string("Hello"); // 返回 0
```

### 富文本支持

```rust
// 富文本字符串
let rich_string = r"<r><rPr><b/></rPr><t>Bold</t></r><r><t>Normal</t></r>";
```

### 空白处理

```rust
// 保留空白字符
xml_si_element(&mut writer, "  spaces  ", true);
```

### 性能优化

- 使用 HashMap 快速查找
- 维护插入顺序
- 批量生成 XML

## Styles

### 功能

`Styles` 管理所有单元格格式和样式，生成 styles.xml 文件。

### 样式文件结构

```xml
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<styleSheet xmlns="...">
    <fonts count="5">
        <font><sz val="11"/><name val="Calibri"/></font>
        <font><b/><sz val="11"/><name val="Calibri"/></font>
        ...
    </fonts>
    <fills count="3">
        <fill><patternFill patternType="none"/></fill>
        ...
    </fills>
    <borders count="2">
        <border>...</border>
        ...
    </borders>
    <cellStyleXfs count="1">...</cellStyleXfs>
    <cellXfs count="10">...</cellXfs>
    <cellStyles count="1">...</cellStyles>
    <dxfs count="5">...</dxfs>
</styleSheet>
```

### 样式层级

```
Styles
├── Fonts（字体）
├── Fills（填充）
├── Borders（边框）
├── NumFmts（数字格式）
├── CellStyleXfs（单元格样式格式）
├── CellXfs（单元格格式）
├── CellStyles（单元格样式）
└── DXFs（条件格式）
```

### 内部结构

```rust
pub struct Styles<'a> {
    writer: Cursor<Vec<u8>>,
    xf_formats: &'a Vec<Format>,
    dxf_formats: &'a Vec<Format>,
    font_count: u16,
    fill_count: u16,
    border_count: u16,
    num_formats: Vec<String>,
    has_hyperlink_style: bool,
    has_comments: bool,
    is_rich_string_style: bool,
    hyperlink_font_id: u16,
}
```

### 样式去重

- 相同格式只存储一次
- 通过引用重用样式
- 减少文件大小

### 格式 ID

```rust
// 每个格式分配唯一 ID
let xf_id = format.xf_id();
```

### 条件格式样式

```rust
// DXF 样式用于条件格式
let dxf_id = format.dxf_id();
```

## Theme

### 功能

`Theme` 管理 Excel 主题，定义颜色方案、字体方案等。

### 主题结构

```xml
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<a:theme xmlns:a="..." name="Office Theme">
    <a:themeElements>
        <a:clrScheme name="Office">
            <a:dk1><a:sysClr val="windowText" lastClr="000000"/></a:dk1>
            <a:lt1><a:sysClr val="window" lastClr="FFFFFF"/></a:lt1>
            ...
        </a:clrScheme>
        <a:fontScheme name="Office">
            <a:majorFont>...</a:majorFont>
            <a:minorFont>...</a:minorFont>
        </a:fontScheme>
        <a:fmtScheme>...</a:fmtScheme>
    </a:themeElements>
</a:theme>
```

### 颜色方案

- `dk1`/`lt1` - 深色/浅色（主要）
- `dk2`/`lt2` - 深色/浅色（次要）
- `accent1` 到 `accent6` - 强调色
- `hlink`/`folHlink` - 超链接颜色

### 字体方案

- `majorFont` - 标题字体
- `minorFont` - 正文字体

### 支持的主题版本

- `THEME_XML_2007` - Excel 2007 主题
- `THEME_XML_2010` - Excel 2010 主题（更新）

## App

### 功能

`App` 生成应用程序属性文件，包含文档元数据。

### 应用属性结构

```xml
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Properties xmlns="..." xmlns:vt="...">
    <Application>Microsoft Excel</Application>
    <DocSecurity>0</DocSecurity>
    <ScaleCrop>false</ScaleCrop>
    <HeadingPairs>
        <vt:vector size="2" baseType="variant">
            <vt:variant><vt:lpstr>Worksheets</vt:lpstr></vt:variant>
            <vt:variant><vt:i4>3</vt:i4></vt:variant>
        </vt:vector>
    </HeadingPairs>
    <TitlesOfParts>
        <vt:vector size="3" baseType="lpstr">
            <vt:lpstr>Sheet1</vt:lpstr>
            <vt:lpstr>Sheet2</vt:lpstr>
            <vt:lpstr>Sheet3</vt:lpstr>
        </vt:vector>
    </TitlesOfParts>
    <Company/>
    <LinksUpToDate>false</LinksUpToDate>
    <SharedDoc>false</SharedDoc>
    <HyperlinksChanged>false</HyperlinksChanged>
    <AppVersion>16.0300</AppVersion>
</Properties>
```

### 主要属性

- `Application` - 应用程序名称
- `DocSecurity` - 文档安全级别
- `ScaleCrop` - 缩放裁剪
- `Company` - 公司名称
- `AppVersion` - 应用程序版本

## Core

### 功能

`Core` 生成核心属性文件，包含 DC 元数据。

### 核心属性结构

```xml
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<cp:coreProperties xmlns:cp="..." xmlns:dc="..." xmlns:dcterms="..." xmlns:dcmitype="...">
    <dc:title>My Document</dc:title>
    <dc:subject>Subject</dc:subject>
    <dc:creator>Author</dc:creator>
    <cp:keywords>keywords</cp:keywords>
    <dc:description>Description</dc:description>
    <cp:lastModifiedBy>Last Modifier</cp:lastModifiedBy>
    <dcterms:created xsi:type="dcterms:W3CDTF">2023-01-01T00:00:00Z</dcterms:created>
    <dcterms:modified xsi:type="dcterms:W3CDTF">2023-01-01T00:00:00Z</dcterms:modified>
</cp:coreProperties>
```

### 属性类型

- `dc:title` - 标题
- `dc:subject` - 主题
- `dc:creator` - 创建者
- `cp:keywords` - 关键词
- `dc:description` - 描述
- `cp:lastModifiedBy` - 最后修改者
- `dcterms:created` - 创建日期
- `dcterms:modified` - 修改日期

## ContentTypes

### 功能

`ContentTypes` 管理内容类型，定义包中各部分的 MIME 类型。

### 内容类型结构

```xml
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="...">
    <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
    <Default Extension="xml" ContentType="application/xml"/>
    <Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/>
    <Override PartName="/xl/worksheets/sheet1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/>
    ...
</Types>
```

### 内容类型映射

| 扩展名/PartName | 内容类型 |
|----------------|----------|
| `.rels` | relationship+xml |
| `.xml` | application/xml |
| `workbook.xml` | spreadsheetml.sheet.main+xml |
| `worksheet` | spreadsheetml.worksheet+xml |
| `sharedStrings.xml` | sharedStrings+xml |
| `styles.xml` | stylesheet+xml |
| `theme` | theme+xml |
| `.png` | image/png |
| `.jpeg` | image/jpeg |

### 动态添加内容类型

```rust
content_types.add_override("/xl/workbook.xml", ContentType::Workbook);
content_types.add_override("/xl/worksheets/sheet1.xml", ContentType::Worksheet);
content_types.add_default("png", ContentType::Png);
```

## Relationship

### 功能

`Relationship` 管理文件之间的关系，定义部件之间的连接。

### 关系类型

```xml
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="...">
    <Relationship Id="rId1" Type="..." Target="worksheets/sheet1.xml"/>
    <Relationship Id="rId2" Type="..." Target="theme/theme1.xml"/>
    ...
</Relationships>
```

### 关系类型常量

```rust
// 工作表关系
RELATIONSHIP_WORKSHEET

// 样式关系
RELATIONSHIP_STYLES

// 主题关系
RELATIONSHIP_THEME

// 超链接关系
RELATIONSHIP_HYPERLINK

// 图片关系
RELATIONSHIP_IMAGE
```

### 关系管理

```rust
// 创建关系
let rel_id = relationships.add_relationship(
    "worksheets/sheet1.xml",
    RELATIONSHIP_WORKSHEET
);
```

## 图片和绘图

### Image

处理图片插入和嵌入：

```rust
let image = Image::new("logo.png")?;
worksheet.insert_image(0, 0, &image)?;
```

### Drawing

处理绘图对象：

```rust
// 管理绘图关系
// 处理图表位置
// 管理形状和文本框
```

### VML

处理 VML（Vector Markup Language）：

```rust
// 用于批注
// 用于按钮
// 用于其他旧式绘图对象
```

## 总结

XML 生成模块是 rust_xlsxwriter 的核心，负责：

1. **文件打包**：组装 xlsx 文件结构
2. **XML 生成**：创建符合标准的 XML 文件
3. **样式管理**：处理格式和样式
4. **关系管理**：维护文件之间的连接
5. **性能优化**：高效的内存使用和处理

这些模块确保生成的 Excel 文件与 Excel 本身生成的文件格式完全一致。