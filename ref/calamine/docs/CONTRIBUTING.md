# 贡献指南

欢迎为 Calamine 项目做出贡献！本指南将帮助您了解如何参与项目开发。

## 开发环境设置

### 前置要求

- Rust 1.83 或更高版本
- Git

### 克隆仓库

```bash
git clone https://github.com/tafia/calamine.git
cd calamine
```

### 构建项目

```bash
cargo build
```

### 运行测试

```bash
# 运行所有测试
cargo test

# 运行特定测试
cargo test test_name

# 运行文档测试
cargo test --doc
```

### 运行基准测试

```bash
cargo bench
```

## 代码结构

### 主要目录

- `src/` - 源代码
- `tests/` - 集成测试
- `examples/` - 示例代码
- `fuzz/` - 模糊测试
- `docs/` - 项目文档

### 添加新功能

1. 在相应的模块中实现功能
2. 添加单元测试
3. 更新文档
4. 提交 Pull Request

## 代码规范

### Rust 风格

项目使用 `rustfmt` 进行代码格式化：

```bash
cargo fmt
```

### Clippy 检查

运行 Clippy 进行代码检查：

```bash
cargo clippy -- -D warnings
```

### 文档注释

所有公共 API 必须有文档注释：

```rust
/// 打开指定格式的工作簿
///
/// # 参数
///
/// * `path` - 工作簿文件路径
///
/// # 返回
///
/// 返回工作簿实例或错误
///
/// # 示例
///
/// ```rust
/// use calamine::{open_workbook, Xlsx};
///
/// let mut workbook: Xlsx<_> = open_workbook("file.xlsx")?;
/// # Ok::<(), calamine::Error>(())
/// ```
pub fn open_workbook<R, P>(path: P) -> Result<R, R::Error>
where
    P: AsRef<Path>,
    R: Reader<BufReader<File>>,
{
    // ...
}
```

## 测试

### 单元测试

在源文件中添加测试：

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_example() {
        assert_eq!(2 + 2, 4);
    }
}
```

### 集成测试

在 `tests/` 目录中添加集成测试：

```rust
// tests/test_format.rs
use calamine::{open_workbook, Xlsx, Reader};

#[test]
fn test_xlsx_reading() {
    let mut workbook: Xlsx<_> = open_workbook("tests/test.xlsx").unwrap();
    let range = workbook.worksheet_range("Sheet1").unwrap();
    // 测试代码
}
```

### 测试文件

将测试文件放在 `tests/` 目录中，确保：
- 文件大小适中
- 覆盖各种边界情况
- 不包含敏感信息

## 文档

### 更新文档

- 在 `docs/` 目录中更新相关文档
- 在 `src/` 中更新代码注释
- 在 `README.md` 中更新使用说明

### 生成文档

```bash
# 生成并打开文档
cargo doc --open

# 生成文档包括私有项
cargo doc --document-private-items
```

## Pull Request 流程

1. Fork 项目仓库
2. 创建功能分支：`git checkout -b feature/my-feature`
3. 提交更改：`git commit -m 'Add some feature'`
4. 推送分支：`git push origin feature/my-feature`
5. 创建 Pull Request

### Pull Request 标题

使用清晰的标题描述更改：

- `feat: 添加 CSV 格式支持`
- `fix: 修复 XLSX 空单元格解析错误`
- `docs: 更新 README 文档`
- `refactor: 重构单元格读取逻辑`

### Pull Request 描述

提供详细的更改说明：

```
## 更改内容

- 添加了 CSV 格式解析器
- 实现了 Reader trait
- 添加了单元测试和集成测试
- 更新了文档

## 测试

运行了所有测试，全部通过。

## 兼容性

- 无破坏性更改
- 与现有 API 兼容
```

## 问题报告

### 报告 Bug

在 GitHub Issues 中创建新问题，提供：
- Bug 描述
- 重现步骤
- 预期行为
- 实际行为
- 环境信息（操作系统、Rust 版本等）
- 最小复现代码

### 功能请求

在 GitHub Issues 中创建新问题，提供：
- 功能描述
- 使用场景
- 期望的 API 设计

## 发布流程

### 版本号

遵循语义化版本（Semantic Versioning）：
- 主版本号：不兼容的 API 更改
- 次版本号：向后兼容的功能新增
- 修订号：向后兼容的问题修正

### 发布步骤

1. 更新 `Cargo.toml` 中的版本号
2. 更新 `CHANGELOG.md`
3. 创建 Git 标签：`git tag -a v0.35.0 -m "Release v0.35.0"`
4. 推送标签：`git push origin v0.35.0`
5. 发布到 crates.io：`cargo publish`

## 许可证

贡献的代码将与项目使用相同的 MIT 许可证。

## 联系方式

- GitHub: https://github.com/tafia/calamine
- Issues: https://github.com/tafia/calamine/issues
- Discussions: https://github.com/tafia/calamine/discussions

感谢您的贡献！
