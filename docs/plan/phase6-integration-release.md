# 阶段6：集成测试、文档与发布

**目标**：全链路验证、补齐文档、发布到 GitHub。
**产出**：可发布的开源项目。

---

## 6.1 集成测试

### 6.1.1 Rust 主项目测试

```bash
# 单元测试
cargo test

# 集成测试（tests/ 目录）
cargo test --test integration

# 端到端测试
# 1. 创建临时文件
# 2. CLI 执行写入/读取/编辑/删除 全流程
# 3. 验证每一步结果 JSON 结构
# 4. 验证文件一致性
```

**集成测试覆盖场景**：

| 场景 | 步骤 | 验证点 |
|------|------|--------|
| 基本 E2E | 创建文件 → 写入单元格 → 读取单元格 → 验证值 | 数据一致 |
| 修改流程 | 读取原文件 → 修改 → 保存 → 验证 diff 含变更 | diff 正确 |
| 安全流程 | dry_run 不修改文件 → 备份创建 → 回滚恢复 | 安全完整 |
| 异常处理 | 文件不存在 → sheet 不存在 → 无效单元格引用 | 错误统一 |
| 并发安全 | 同一文件连续多次写入操作 | 功能正常 |

### 6.1.2 Python Diff 项目测试

```bash
pytest tests/
```

**覆盖场景**：
- 两个相同 Excel diff → `NoChange`
- 单元格值修改 → `Modify` 正确标记
- 公式不变但值变 → `AUTO_UPDATE` 标记
- 公式文本变化 → `Modify` 主动标记
- 行新增/删除 → `Add`/`Delete` 行级 diff

### 6.1.3 跨项目集成测试

验证两个工具的协同工作：
1. Rust CLI 编辑 Excel 文件 → `git commit`
2. `excel-diff diff` 查看版本差异 → diff 正确

## 6.2 文档编写

| 文档 | 位置 | 内容 |
|------|------|------|
| README.md | 项目根目录 | 简介、安装、快速开始、命令列表 |
| 架构设计 | `docs/架构设计.md` | 已存在，补充实现细节 |
| CLI 使用手册 | `docs/cli-usage.md` | 所有子命令+参数+示例 |
| HTTP API 文档 | `docs/api-docs.md` | 路由+请求体+响应示例 |
| 开发指南 | `docs/development.md` | 环境配置、编码规范、PR 流程 |
| 安全规范 | `docs/security.md` | 备份策略、指纹校验、故障恢复 |

## 6.3 CI/CD 配置

### GitHub Actions

```yaml
# .github/workflows/ci.yml
on: [push, pull_request]
jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Build Rust
        run: cargo build --all-features
      - name: Run Rust tests
        run: cargo test
      - name: Lint
        run: cargo clippy -- -D warnings
      - name: Format check
        run: cargo fmt --check
```

**包含的检查项**：
- `cargo build`（全特性）
- `cargo test`（单元+集成）
- `cargo clippy`
- `cargo fmt`
- `cargo audit`（依赖安全扫描）
- 跨平台编译验证（ubuntu/macos/windows）

## 6.4 发布准备

### Rust 项目发布

```bash
# 验证发布
cargo publish --dry-run

# 本地打包
cargo build --release
# 产物：target/release/excel-tool-gateway.exe (Windows)
# 产物：target/release/excel-tool-gateway (Linux/macOS)
```

### Python 项目发布

```bash
# 构建
python -m build

# 发布到 PyPI
twine upload dist/*
```

### GitHub Release

| 资源 | 说明 |
|------|------|
| Rust CLI 静态二进制 | Windows/macOS/Linux 三平台 |
| pip install excel-diff | PyPI 发布 |
| Web 前端静态文件 | GitHub Pages 部署 |

## 6.5 性能验证

验证项目满足轻量化目标：

| 指标 | 目标值 | 衡量方式 |
|------|--------|----------|
| CLI 二进制大小 | < 10MB | `ls -lh target/release/` |
| 启动时间 | < 100ms | `hyperfine excel --help` |
| 读取 10MB xlsx | < 1s | 集成测试计时 |
| 写入 1000 单元格 | < 2s | 集成测试计时 |
| HTTP 首次响应 | < 200ms | `curl -w %{time_total}` |
| binary 内存占用 | < 50MB | `valgrind` / `heaptrack` |

## 6.6 发布检查清单

### 代码质量
- [ ] `cargo clippy` 无 warning
- [ ] `cargo fmt` 已格式化
- [ ] 所有测试通过
- [ ] 无用代码/注释清理
- [ ] API 无破坏性变更

### 安全性
- [ ] 所有写操作强制前置备份
- [ ] SSH/令牌等凭据不硬编码
- [ ] 默认 JSON 输出不含敏感信息

### 兼容性
- [ ] CLI 支持 Windows/macOS/Linux
- [ ] `.xlsx` / `.xlsm` / `.xls` 格式覆盖
- [ ] HTTP 接口 CORS 配置
- [ ] CLI `--help` 信息完整

### 文档
- [ ] README.md 完整
- [ ] CHANGELOG.md 记录版本变更
- [ ] 示例代码/命令已验证
- [ ] API 文档同步更新
