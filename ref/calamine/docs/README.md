# Calamine 项目文档

欢迎使用 Calamine 项目文档！本文档提供了 Calamine 库的完整技术文档，包括架构设计、模块功能和实现细节。

## 文档目录

### 1. 项目概览
**文件**: `01-project-overview.md`

包含项目的基本信息：
- 项目简介和版本信息
- 支持的文件格式
- 核心功能列表
- 主要依赖和特性
- 性能特点
- 使用场景

### 2. 架构设计
**文件**: `02-architecture.md`

详细介绍项目的架构设计：
- 总体架构和分层设计
- 核心设计模式（Trait 抽象、枚举多态等）
- 关键组件设计
- 数据流分析
- 扩展性设计
- 性能优化策略

### 3. 模块功能详解
**文件**: `03-modules.md`

详细说明各个模块的职责和功能：
- 核心模块（lib.rs、datatype.rs）
- 格式解析器模块（xlsx、xlsb、xls、ods）
- 基础设施模块（cfb、vba、formats、utils）
- 便利模块（auto、errors）
- 模块依赖关系图
- 扩展点说明

### 4. XLSX 模块详解
**文件**: `modules/xlsx.md`

XLSX 格式解析器的详细文档：
- 模块结构和核心组件
- XLSX 文件格式解析
- 单元格读取机制
- 懒加载支持
- 性能优化策略
- 使用示例和错误处理

### 5. XLS 模块详解
**文件**: `modules/xls.md`

XLS 格式解析器的详细文档：
- 模块结构和核心组件
- XLS 文件格式解析
- 单元格数据类型
- 公式解析机制
- 性能考虑和限制
- 使用示例和错误处理

## 快速导航

### 新手入门
1. 阅读项目概览（01-project-overview.md）了解基本信息
2. 查看架构设计（02-architecture.md）理解整体设计
3. 浏览模块功能（03-modules.md）了解各个模块

### 深入学习
1. 选择感兴趣的格式模块：
   - XLSX 格式：modules/xlsx.md
   - XLS 格式：modules/xls.md
   - XLSB 格式：详见源码和注释
   - ODS 格式：详见源码和注释

2. 研究基础设施模块：
   - CFB 解析：src/cfb.rs
   - VBA 解析：src/vba.rs
   - 格式处理：src/formats.rs
   - 工具函数：src/utils.rs

### 贡献和扩展
1. 参考架构设计了解扩展点
2. 查看模块功能了解如何添加新格式
3. 研究现有模块的实现

## 文档约定

### 代码示例
文档中的代码示例使用 Rust 语言，并遵循以下格式：

```rust
use calamine::{open_workbook, Xlsx, Reader};

let mut workbook: Xlsx<_> = open_workbook("file.xlsx")?;
let range = workbook.worksheet_range("Sheet1")?;
```

### 类型表示
- 模块：粗体（如 **xlsx**）
- 结构体/枚举：`MonoSpaced`（如 `Xlsx`）
- trait：`MonoSpaced`（如 `Reader`）
- 函数：`MonoSpaced()`（如 `open_workbook()`）
- 文件路径：斜体（如 *src/xlsx/mod.rs*）

### 图表和表格
文档中使用图表和表格来展示：
- 架构图
- 流程图
- 对比表
- 依赖关系图

## 相关资源

### 官方资源
- GitHub 仓库：https://github.com/tafia/calamine
- API 文档：https://docs.rs/calamine
- Crates.io：https://crates.io/crates/calamine

### 标准参考
- ECMA-376 (Office Open XML)：https://www.ecma-international.org/publications-and-standards/standards/ecma-376/
- OpenDocument v1.2：https://docs.oasis-open.org/office/v1.2/OpenDocument-v1.2-os-part1.html
- MS-CFB：https://learn.microsoft.com/en-us/openspecs/office_file_formats/ms-cfb/

### 社区
- GitHub Issues：https://github.com/tafia/calamine/issues
- GitHub Discussions：https://github.com/tafia/calamine/discussions

## 文档版本

- 当前版本：v0.35.0
- 最后更新：2026-05-30
- 维护者：Johann Tuffe

## 反馈和贡献

如果您发现文档中的错误或有改进建议，请：
1. 在 GitHub 上提交 Issue
2. 创建 Pull Request 修复问题
3. 参与讨论提供反馈

## 许可证

本文档遵循 MIT 许可证，与 Calamine 项目保持一致。

---

**注意**：本文档旨在帮助开发者和贡献者理解 Calamine 的内部实现和设计。如果您只是想使用 Calamine 库，建议查看 README.md 和官方 API 文档。