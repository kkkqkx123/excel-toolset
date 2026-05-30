# Calamine 项目概览

## 项目简介

Calamine 是一个用纯 Rust 编写的 Excel/OpenDocument 电子表格读取和反序列化库。

### 基本信息

- **项目名称**: calamine
- **版本**: 0.35.0
- **作者**: Johann Tuffe
- **许可证**: MIT
- **Rust 版本要求**: 1.83+
- **仓库地址**: https://github.com/tafia/calamine
- **文档地址**: https://docs.rs/calamine

## 支持的文件格式

Calamine 支持多种电子表格文件格式：

### Excel 格式
- **XLS**: 传统 Excel 二进制格式 (Excel 97-2003)
- **XLSX**: 基于 XML 的 Excel 格式 (Excel 2007+)
- **XLSB**: Excel 二进制格式 (Excel 2007+)
- **XLSM**: 带宏的 Excel 工作簿
- **XLA**: Excel 加载项 (旧版)
- **XLAM**: Excel 加载项 (新版)

### OpenDocument 格式
- **ODS**: OpenDocument 电子表格

## 核心功能

### 1. 读取功能
- 读取单元格数据（值、公式）
- 读取工作表元数据（名称、类型、可见性）
- 读取 VBA 项目代码
- 读取定义的名称（Defined Names）
- 读取单元格格式信息
- 读取图片数据（可选功能）

### 2. 数据类型支持
- 整数 (Int)
- 浮点数 (Float)
- 字符串 (String)
- 布尔值 (Bool)
- 日期时间 (DateTime)
- ISO 8601 格式的日期时间和持续时间
- 错误类型 (Error)
- 空单元格 (Empty)

### 3. 高级特性
- **Serde 反序列化**: 支持直接将表格数据反序列化为 Rust 结构体
- **懒加载**: XLSX 和 XLSB 格式支持懒加载，提高大文件读取性能
- **表头行配置**: 支持自定义表头行位置
- **格式检测**: 支持自动检测文件格式
- **VBA 项目支持**: 解析和读取 VBA 宏代码

## 可选特性

### chrono
添加对 Chrono 日期时间类型的支持，提供更好的日期时间处理能力。

### picture
添加对读取电子表格中图片原始数据的支持。

### dates
`chrono` 特性的向后兼容别名（已废弃）。

## 主要依赖

### 核心依赖
- **log**: 日志记录
- **serde**: 序列化和反序列化框架
- **codepage**: 代码页转换支持
- **atoi_simd**: 高性能整数解析
- **byteorder**: 字节序处理
- **encoding_rs**: 字符编码处理
- **fast-float2**: 快速浮点数解析
- **zip**: ZIP 文件解压（用于 XLSX、XLSB、ODS）
- **quick-xml**: XML 解析（用于 XLSX、ODS）

### 开发依赖
- **glob**: 文件模式匹配
- **sha2**: SHA-2 哈希算法
- **env_logger**: 环境变量日志记录器
- **serde_derive**: Serde 派生宏
- **rstest**: 测试框架
- **criterion**: 性能基准测试

## 性能特点

- **纯 Rust 实现**: 无需外部 C 库依赖
- **懒加载**: 对于大文件，按需加载数据
- **高性能**: 使用 SIMD 和其他优化技术
- **内存安全**: Rust 的所有权系统保证内存安全
- **零拷贝**: 部分操作支持零拷贝引用

## 使用场景

- 数据分析和处理
- 报表生成和读取
- 数据导入导出
- 自动化测试数据读取
- 数据迁移和转换
- 配置文件读取（以 Excel 格式存储）

## 项目结构

项目采用模块化设计，主要模块包括：

- **lib.rs**: 库入口，定义公共 API
- **xlsx**: XLSX 格式处理模块
- **xls**: XLS 格式处理模块
- **xlsb**: XLSB 格式处理模块
- **ods**: ODS 格式处理模块
- **vba**: VBA 项目解析模块
- **cfb**: 复合文件二进制格式解析（用于 XLS 和 VBA）
- **datatype**: 数据类型定义和处理
- **de**: Serde 反序列化支持
- **errors**: 错误类型定义
- **formats**: 单元格格式处理
- **utils**: 工具函数集合
- **auto**: 自动格式检测和读取

## 设计原则

1. **类型安全**: 使用 Rust 的类型系统确保数据安全
2. **零成本抽象**: 提供易用的 API，同时保持高性能
3. **模块化**: 清晰的模块边界，便于维护和扩展
4. **向后兼容**: 保持 API 的稳定性
5. **错误处理**: 完善的错误处理机制