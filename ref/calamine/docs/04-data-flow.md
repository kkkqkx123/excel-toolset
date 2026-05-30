# 数据流和内部实现

## 概述

本文档详细介绍 Calamine 库内部的数据流转过程和实现细节，帮助开发者理解从文件读取到数据返回的完整流程。

## 整体数据流

```
文件输入 (File/Bytes)
    ↓
格式检测 (auto.rs)
    ↓
选择解析器 (Xls/Xlsx/Xlsb/Ods)
    ↓
打开工作簿 (Reader::new)
    ↓
解析元数据 (Metadata)
    ↓
用户请求 (worksheet_range)
    ↓
读取工作表数据
    ↓
解析单元格 (Cells Reader)
    ↓
格式化和类型转换 (formats.rs)
    ↓
返回 Range<Data> 或 Range<DataRef>
```

## 格式检测流程

### auto 模块的检测逻辑

```rust
pub fn open_workbook_auto<P>(path: P) -> Result<Sheets<BufReader<File>>, Error>
where
    P: AsRef<Path>,
{
    let path = path.as_ref();
    Ok(match path.extension().and_then(|e| e.to_str()) {
        Some("xls" | "xla") => Sheets::Xls(open_workbook(path).map_err(Error::Xls)?),
        Some("xlsx" | "xlsm" | "xlam") => Sheets::Xlsx(open_workbook(path).map_err(Error::Xlsx)?),
        Some("xlsb") => Sheets::Xlsb(open_workbook(path).map_err(Error::Xlsb)?),
        Some("ods") => Sheets::Ods(open_workbook(path).map_err(Error::Ods)?),
        _ => {
            // 扩展名不明确时，尝试各种格式
            if let Ok(ret) = open_workbook::<Xls<_>, _>(path) {
                return Ok(Sheets::Xls(ret));
            } else if let Ok(ret) = open_workbook::<Xlsx<_>, _>(path) {
                return Ok(Sheets::Xlsx(ret));
            } else if let Ok(ret) = open_workbook::<Xlsb<_>, _>(path) {
                return Ok(Sheets::Xlsb(ret));
            } else if let Ok(ret) = open_workbook::<Ods<_>, _>(path) {
                return Ok(Sheets::Ods(ret));
            } else {
                return Err(Error::Msg("Cannot detect file format"));
            };
        }
    })
}
```

## XLSX 解析流程

### 打开工作簿

```rust
impl<RS> Reader<RS> for Xlsx<RS>
where
    RS: Read + Seek,
{
    fn new(reader: RS) -> Result<Self, Self::Error> {
        // 1. 打开 ZIP 容器
        let mut zip = ZipArchive::new(reader)?;

        // 2. 解析关系文件
        let relationships = parse_relationships(&mut zip, "_rels/.rels")?;

        // 3. 查找工作簿文件位置
        let workbook_path = find_workbook_path(&relationships)?;

        // 4. 解析工作簿文件获取元数据
        let (sheets_metadata, defined_names, shared_strings) =
            parse_workbook(&mut zip, &workbook_path)?;

        // 5. 返回 Xlsx 实例（此时不加载工作表数据）
        Ok(Xlsx {
            zip,
            strings: shared_strings,
            sheets_metadata,
            defined_names,
            // ...
        })
    }
}
```

### 读取工作表数据

```rust
fn worksheet_range(&mut self, name: &str) -> Result<Range<Data>, Self::Error> {
    // 1. 查找工作表路径
    let worksheet_path = self.get_worksheet_path(name)?;

    // 2. 使用 cells_reader 流式读取
    let mut reader = XlsxCellReader::new(
        &mut self.zip,
        &worksheet_path,
        &self.strings,
        &self.formats,
    );

    // 3. 解析所有单元格
    let mut cells = BTreeMap::new();
    while let Some((row, col, data)) = reader.next_cell()? {
        cells.insert((row, col), data);
    }

    // 4. 构建 Range
    let range = Range::from_sparse(cells);
    Ok(range)
}
```

### Cells Reader 实现细节

```rust
pub struct XlsxCellReader<'a, RS> {
    zip: &'a mut ZipArchive<RS>,
    shared_strings: &'a [String],
    zip_path_cache: HashMap<String, usize>,
    formats: &'a BTreeMap<usize, String>,
}

impl<'a, RS> XlsxCellReader<'a, RS>
where
    RS: Read + Seek,
{
    pub fn next_cell(&mut self) -> Result<Option<(u32, u32, Data)>, XlsxError> {
        // 1. 读取 XML 事件
        let event = self.reader.read_event()?;

        // 2. 处理单元格标签
        match event {
            Event::Start(ref e) if e.name() == QName(b"c") => {
                // 解析单元格位置
                let (row, col) = parse_cell_ref(e.attributes())?;

                // 3. 解析单元格值
                let data = self.parse_cell_value(e)?;

                Ok(Some((row, col, data)))
            }
            // ... 其他事件处理
        }
    }

    fn parse_cell_value(&mut self, cell: &BytesStart) -> Result<Data, XlsxError> {
        // 1. 检查单元格类型（s=共享字符串, n=数字, b=布尔等）
        let cell_type = get_cell_type(cell.attributes())?;

        // 2. 根据类型解析值
        match cell_type {
            "s" => {
                // 共享字符串
                let index = self.reader.read_text(b"v")?;
                let index: u32 = index.parse()?;
                let string = self.shared_strings[index as usize].clone();
                Ok(Data::String(string))
            }
            "n" => {
                // 数字
                let value = self.reader.read_text(b"v")?;
                let value: f64 = value.parse()?;
                Ok(Data::Float(value))
            }
            "b" => {
                // 布尔值
                let value = self.reader.read_text(b"v")?;
                let value: u32 = value.parse()?;
                Ok(Data::Bool(value == 1))
            }
            // ... 其他类型处理
        }
    }
}
```

## XLS 解析流程

### 打开工作簿

```rust
impl<RS> Reader<RS> for Xls<RS>
where
    RS: Read + Seek,
{
    fn new(mut reader: RS) -> Result<Self, Self::Error> {
        // 1. 打开 CFB 容器
        let mut cfb = Cfb::new(&mut reader, len)?;

        // 2. 读取 Workbook 流
        let mut workbook_stream = cfb.get_stream("Workbook", &mut reader)?;

        // 3. 解析 BOF 记录
        let bof = parse_bof(&mut workbook_stream)?;

        // 4. 解析全局记录（字体、格式、SST 等）
        while let Some(record) = parse_record(&mut workbook_stream)? {
            match record.id {
                0x00FC => parse_font(&record.data),
                0x041E => parse_format(&record.data),
                0x00FC => parse_xf(&record.data),
                0x00FC => parse_sst(&record.data),
                // ... 其他记录类型
            }
        }

        // 5. 解析工作表边界
        for sheet in &boundsheets {
            self.parse_sheet(&mut cfb, sheet)?;
        }

        Ok(Xls { /* ... */ })
    }
}
```

### 解析工作表

```rust
fn parse_sheet(&mut self, cfb: &mut Cfb, sheet: &Boundsheet) -> Result<(), XlsError> {
    // 1. 读取工作表流
    let mut sheet_stream = cfb.get_stream(&sheet.stream_name, &mut reader)?;

    // 2. 解析 BOF
    let bof = parse_bof(&mut sheet_stream)?;

    // 3. 解析行和单元格记录
    while let Some(record) = parse_record(&mut sheet_stream)? {
        match record.id {
            0x0208 => parse_row(&record.data),      // ROW 记录
            0x0203 => parse_number(&record.data),   // NUMBER 记录
            0x0205 => parse_bool_err(&record.data), // BOOLERR 记录
            0x027E => parse_rk(&record.data),       // RK 记录
            0x00FD => parse_label_sst(&record.data), // LABELSST 记录
            0x0204 => parse_label(&record.data),     // LABEL 记录
            0x0006 => parse_formula(&record.data),  // FORMULA 记录
            // ... 其他记录类型
        }
    }

    Ok(())
}
```

## XLSB 解析流程

XLSB 解析流程与 XLSX 类似，但使用二进制格式而非 XML：

```rust
fn parse_xlsb_cell(&mut self) -> Result<Option<(u32, u32, Data)>, XlsbError> {
    // 1. 读取记录类型
    let record_id = read_u16(&mut self.reader)?;

    // 2. 根据记录类型处理
    match record_id {
        0x0001 => {
            // BrtRowHdr - 行头
            let row = read_u32(&mut self.reader)?;
            // ...
        }
        0x0003 => {
            // BrtCellBlank - 空单元格
            let (row, col) = parse_cell_ref(&mut self.reader)?;
            Ok(Some((row, col, Data::Empty)))
        }
        0x0004 => {
            // BrtCellRk - RK 编码的数字
            let (row, col) = parse_cell_ref(&mut self.reader)?;
            let value = parse_rk(&mut self.reader)?;
            Ok(Some((row, col, Data::Float(value))))
        }
        0x0005 => {
            // BrtCellError - 错误单元格
            let (row, col) = parse_cell_ref(&mut self.reader)?;
            let error = parse_error(&mut self.reader)?;
            Ok(Some((row, col, Data::Error(error))))
        }
        0x0006 => {
            // BrtCellBool - 布尔单元格
            let (row, col) = parse_cell_ref(&mut self.reader)?;
            let value = read_u8(&mut self.reader)? != 0;
            Ok(Some((row, col, Data::Bool(value))))
        }
        0x0007 => {
            // BrtCellReal - 实数单元格
            let (row, col) = parse_cell_ref(&mut self.reader)?;
            let value = read_f64(&mut self.reader)?;
            Ok(Some((row, col, Data::Float(value))))
        }
        0x0008 => {
            // BrtCellSt - 字符串单元格
            let (row, col) = parse_cell_ref(&mut self.reader)?;
            let string = parse_xlWideString(&mut self.reader)?;
            Ok(Some((row, col, Data::String(string))))
        }
        0x0009 => {
            // BrtCellIsst - 共享字符串单元格
            let (row, col) = parse_cell_ref(&mut self.reader)?;
            let index = read_u32(&mut self.reader)?;
            let string = self.shared_strings[index as usize].clone();
            Ok(Some((row, col, Data::String(string))))
        }
        // ... 其他记录类型
    }
}
```

## ODS 解析流程

ODS 解析流程与 XLSX 类似，但使用 OpenDocument XML 格式：

```rust
fn parse_ods_cell(&mut self) -> Result<Option<(u32, u32, Data)>, OdsError> {
    // 1. 读取 XML 事件
    let event = self.reader.read_event()?;

    // 2. 处理 table:table-cell 元素
    match event {
        Event::Start(ref e) if e.name() == QName(b"table:table-cell") => {
            // 1. 解析单元格位置
            let (row, col) = self.current_cell_position;

            // 2. 检查重复次数
            let repeat = get_attribute(e.attributes(), b"table:number-columns-repeated")?;

            // 3. 解析单元格值
            let data = self.parse_ods_cell_value(e)?;

            // 4. 更新位置
            self.current_cell_position.1 += repeat;

            Ok(Some((row, col, data)))
        }
        Event::End(ref e) if e.name() == QName(b"table:table-row") => {
            // 行结束
            self.current_cell_position.0 += 1;
            self.current_cell_position.1 = 0;
            self.next_cell()
        }
        _ => self.next_cell(),
    }
}

fn parse_ods_cell_value(&mut self, cell: &BytesStart) -> Result<Data, OdsError> {
    // 1. 检查单元格类型
    let value_type = get_attribute(cell.attributes(), b"office:value-type")?;

    match value_type {
        "float" => {
            let value = get_attribute(cell.attributes(), b"office:value")?;
            Ok(Data::Float(value.parse()?))
        }
        "string" => {
            let text = read_element_text(&mut self.reader, b"text:p")?;
            Ok(Data::String(text))
        }
        "boolean" => {
            let value = get_attribute(cell.attributes(), b"office:boolean-value")?;
            Ok(Data::Bool(value == "true"))
        }
        "date" => {
            let value = get_attribute(cell.attributes(), b"office:date-value")?;
            Ok(Data::DateTimeIso(value.to_string()))
        }
        // ... 其他类型
    }
}
```

## 数据类型转换

### 格式检测和转换

```rust
// 在 formats.rs 中
pub fn format_excel_f64(
    value: f64,
    format: Option<&CellFormat>,
    is_1904: bool,
) -> Data {
    match format {
        Some(CellFormat::DateTime) => {
            Data::DateTime(ExcelDateTime::new(
                value,
                ExcelDateTimeType::DateTime,
                is_1904,
            ))
        }
        Some(CellFormat::TimeDelta) => {
            Data::DateTime(ExcelDateTime::new(
                value,
                ExcelDateTimeType::TimeDelta,
                is_1904,
            ))
        }
        _ => {
            // 判断是整数还是浮点数
            if value.fract() == 0.0 && value >= i64::MIN as f64 && value <= i64::MAX as f64 {
                Data::Int(value as i64)
            } else {
                Data::Float(value)
            }
        }
    }
}
```

### Excel 日期时间处理

```rust
impl ExcelDateTime {
    pub fn new(value: f64, typ: ExcelDateTimeType, is_1904: bool) -> Self {
        ExcelDateTime {
            value,
            typ,
            is_1904,
        }
    }

    pub fn as_datetime(&self) -> Option<chrono::NaiveDateTime> {
        // 1. 转换为 Excel 基准日期
        let days = self.value.floor();
        let time = self.value.fract();

        // 2. 根据 1900 或 1904 系统调整
        let base_date = if self.is_1904 {
            // 1904 系统基准：1904-01-01
            NaiveDate::from_ymd_opt(1904, 1, 1)?
        } else {
            // 1900 系统基准：1899-12-30（Excel bug: 认为有 1900-02-29）
            NaiveDate::from_ymd_opt(1899, 12, 30)?
        };

        // 3. 计算日期
        let date = base_date + Duration::days(days as i64);

        // 4. 计算时间
        let seconds = (time * 86400.0).round() as i64;
        let time = NaiveTime::from_num_seconds_from_midnight_opt(seconds, 0)?;

        Some(NaiveDateTime::new(date, time))
    }
}
```

## Range 数据结构

### Range 实现

```rust
pub struct Range<T> {
    start: (u32, u32),
    end: (u32, u32),
    inner: Vec<Vec<T>>,
}

impl<T> Range<T> {
    pub fn new(start: (u32, u32), end: (u32, u32), inner: Vec<Vec<T>>) -> Self {
        Range { start, end, inner }
    }

    pub fn from_sparse(cells: BTreeMap<(u32, u32), T>) -> Self {
        if cells.is_empty() {
            return Range::empty();
        }

        // 1. 计算范围
        let (min_row, min_col) = cells.keys().min().unwrap();
        let (max_row, max_col) = cells.keys().max().unwrap();

        // 2. 创建二维数组
        let rows = (max_row - min_row + 1) as usize;
        let cols = (max_col - min_col + 1) as usize;
        let mut inner = vec![vec![T::default(); cols]; rows];

        // 3. 填充数据
        for ((row, col), value) in cells {
            let r = (row - min_row) as usize;
            let c = (col - min_col) as usize;
            inner[r][c] = value;
        }

        Range {
            start: (*min_row, *min_col),
            end: (*max_row, *max_col),
            inner,
        }
    }

    pub fn rows(&self) -> Rows<'_, T> {
        Rows {
            inner: &self.inner,
            index: 0,
        }
    }

    pub fn get(&self, row: u32, col: u32) -> Option<&T> {
        let r = (row - self.start.0) as usize;
        let c = (col - self.start.1) as usize;
        self.inner.get(r)?.get(c)
    }
}
```

## VBA 项目解析流程

### VBA 解析

```rust
impl VbaProject {
    pub fn new<R: Read>(r: &mut R, len: usize) -> Result<VbaProject, VbaError> {
        // 1. 打开 CFB 容器
        let mut cfb = Cfb::new(r, len)?;

        // 2. 解析 VBA 项目结构
        let dir_stream = cfb.get_stream("dir", r)?;
        let dir_stream = decompress_stream(&dir_stream)?;

        // 3. 解析目录
        let (references, modules) = parse_dir(&dir_stream)?;

        // 4. 读取模块内容
        let mut module_data = BTreeMap::new();
        for module in &modules {
            let stream = cfb.get_stream(&module.stream_name, r)?;
            let stream = decompress_stream(&stream)?;
            module_data.insert(module.name.clone(), stream);
        }

        Ok(VbaProject {
            references,
            modules: module_data,
            encoding: XlsEncoding::default(),
        })
    }
}
```

## 性能优化细节

### 懒加载实现

XLSX 和 XLSB 支持懒加载，只在用户请求时读取工作表：

```rust
// 懒加载：不预先加载工作表
impl<RS> Reader<RS> for Xlsx<RS>
where
    RS: Read + Seek,
{
    fn new(reader: RS) -> Result<Self, Self::Error> {
        // 只解析元数据，不加载工作表数据
        Ok(Xlsx {
            zip: ZipArchive::new(reader)?,
            strings: None,  // 延迟加载共享字符串
            cells: HashMap::new(),  // 空的单元格缓存
            // ...
        })
    }

    fn worksheet_range(&mut self, name: &str) -> Result<Range<Data>, Self::Error> {
        // 首次访问时加载共享字符串
        if self.strings.is_none() {
            self.strings = Some(self.load_shared_strings()?);
        }

        // 按需加载工作表
        if !self.cells.contains_key(name) {
            let range = self.load_worksheet(name)?;
            self.cells.insert(name.to_string(), range);
        }

        Ok(self.cells.get(name).unwrap().clone())
    }
}
```

### 零拷贝实现

使用 `DataRef` 避免字符串拷贝：

```rust
pub enum DataRef<'a> {
    Int(i64),
    Float(f64),
    StringRef(&'a str),  // 借用，避免拷贝
    Bool(bool),
    // ...
}

impl<'a> ReaderRef<RS> for Xlsx<RS> {
    fn worksheet_range_ref<'b>(
        &'b mut self,
        name: &str,
    ) -> Result<Range<DataRef<'b>>, Self::Error> {
        // 返回 DataRef，字符串是借用的
        let mut reader = XlsxCellReader::new(&mut self.zip, /* ... */);

        let mut cells = BTreeMap::new();
        while let Some((row, col, data)) = reader.next_cell_ref()? {
            cells.insert((row, col), data);  // data 包含字符串引用
        }

        Ok(Range::from_sparse(cells))
    }
}
```

### ZIP 路径缓存

避免重复解析关系文件：

```rust
fn build_zip_path_cache<RS>(
    zip: &mut ZipArchive<RS>,
    relationships: &[(String, String)],
) -> Result<HashMap<String, usize>, ZipError>
where
    RS: Read + Seek,
{
    let mut cache = HashMap::new();

    for (source, target) in relationships {
        let target_path = get_zip_path(zip, target)?;
        cache.insert(source.clone(), target_path);
    }

    Ok(cache)
}

// 使用缓存
fn get_worksheet_path(&self, name: &str) -> Result<String, XlsxError> {
    if let Some(&path_index) = self.zip_path_cache.get(name) {
        Ok(self.zip_path_by_index(path_index)?)
    } else {
        Err(XlsxError::WorksheetNotFound(name.to_string()))
    }
}
```

## 错误处理流程

### 错误转换

```rust
// 在 errors.rs 中
impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Error {
        Error::Io(e)
    }
}

impl From<XlsxError> for Error {
    fn from(e: XlsxError) -> Error {
        Error::Xlsx(e)
    }
}

// 使用
fn example() -> Result<(), Error> {
    let mut workbook: Xlsx<_> = open_workbook("file.xlsx")?;  // 自动转换错误
    Ok(())
}
```

### 特定错误处理

```rust
match open_workbook::<Xlsx<_>, _>("file.xlsx") {
    Ok(workbook) => {
        // 处理工作簿
    }
    Err(Error::Xlsx(XlsxError::FileNotFound(name))) => {
        eprintln!("文件未找到: {}", name);
    }
    Err(Error::Xlsx(XlsxError::Password)) => {
        eprintln!("文件受密码保护");
    }
    Err(Error::Io(e)) => {
        eprintln!("IO 错误: {}", e);
    }
    Err(e) => {
        eprintln!("其他错误: {}", e);
    }
}
```

## 总结

Calamine 的数据流设计遵循以下原则：

1. **懒加载**：XLSX 和 XLSB 按需加载工作表数据
2. **流式解析**：逐步读取单元格，避免一次性加载全部数据
3. **零拷贝**：使用引用类型减少内存分配
4. **缓存优化**：缓存共享字符串、ZIP 路径等重复使用的数据
5. **错误转换**：统一的错误类型，便于错误处理

这些设计使得 Calamine 能够高效地处理大文件，同时保持 API 的简洁性和易用性。