# 阶段5：Diff 子系统（excel-diff）

**目标**：独立于主项目，实现 Python 版的 Excel Diff 工具，专注 Git 集成 + 智能 diff 可视化。
**产出**：`excel-diff` CLI 工具 + 纯静态 Web 前端，实现 Git diff 替代、历史查询、前端渲染。

---

## 5.1 项目定位

`excel-diff` 是独立于 `excel-tool-gateway`（Rust）的附属工具，解决的是**不同问题**：

| 维度 | excel-tool-gateway (Rust) | excel-diff (Python) |
|------|--------------------------|---------------------|
| 核心任务 | Excel 原子操作（读写/编辑） | Git 版本管理 + 智能 diff 可视化 |
| 用户 | AI Agent / 开发者 | Git 用户 / 非 Git 用户 |
| 输出 | 操作结果 JSON | 结构化 diff + 前端渲染 |
| 依赖 | Rust 无头库 | openpyxl + GitPython |

## 5.2 架构

```
┌─────────────────────────────────────────────────┐
│ 展示层: 纯静态 Web 前端 (HTML/JS/CSS)            │
│ 零依赖，表格diff渲染、历史查询、版本回溯          │
└──────────────────────┬──────────────────────────┘
                       │ HTTP/CLI
┌──────────────────────▼──────────────────────────┐
│ 接口层: CLI 命令行 + Web API                    │
│ CLI: `excel-diff diff`, `excel-diff log`        │
│ API: FastAPI 封装核心引擎                        │
└──────────────────────┬──────────────────────────┘
                       │ 调用
┌──────────────────────▼──────────────────────────┐
│ 核心引擎层                                       │
│ 1. Excel 解析 + 结构化 diff                      │
│ 2. 公式降噪（主动修改 vs 被动更新）               │
│ 3. Git 操作封装（版本、diff、历史、回滚）          │
│ 4. AI Agent 操作日志合并                         │
│ 5. 统一输出 JSON                                 │
└──────────────────────┬──────────────────────────┘
                       │ 持久化
┌──────────────────────▼──────────────────────────┐
│ 存储层: Git 本地仓库 (无数据库)                  │
│ 存储所有 Excel 版本、修改历史                    │
└─────────────────────────────────────────────────┘
```

## 5.3 阶段5a：核心引擎 + CLI（MVP）

### 依赖

```txt
openpyxl>=3.1
GitPython>=3.1
click>=8.0
rich>=10.0
```

### 核心模块

```
excel-diff/
├── pyproject.toml
├── src/
│   ├── __init__.py
│   ├── cli.py              # Click 命令行定义
│   ├── engine/
│   │   ├── __init__.py
│   │   ├── excel_parser.py  # openpyxl 解析（值+公式+样式）
│   │   ├── diff_core.py     # 核心 diff 算法（值级、行级、单元格级）
│   │   ├── formula.py       # 公式解析与依赖链追踪
│   │   └── noise_filter.py  # 公式降噪（区分主动/被动修改）
│   ├── git/
│   │   ├── __init__.py
│   │   └── integration.py   # Git diff 驱动、日志、回滚
│   └── output/
│       ├── __init__.py
│       ├── json_formatter.py
│       └── markdown_formatter.py
```

### CLI 命令

```bash
# 替代 git diff（注册为 Git diff 驱动）
excel-diff diff <old-file> <new-file> [--sheet] [--range]
excel-diff diff HEAD~1 HEAD --sheet Sheet1

# 历史查询
excel-diff log [--path <file>] [--limit 10]
excel-diff show <commit-hash>

# 版本回滚
excel-diff checkout <commit-hash> [--output <path>]

# AI 操作日志合并
excel-diff merge-log <log.json> --into <path>

# 注册 Git 驱动
excel-diff install-git-driver
```

### Git 集成

注册 Git diff 驱动：
```bash
# 写入 .gitattributes
*.xlsx diff=excel-diff

# 写入 git config
git config diff.excel-diff.command "excel-diff diff"
```

### 5.3.1 公式降噪算法

区分**主动修改**和**被动更新**，是 diff 系统的核心差异化能力：

```
主动修改规则：
  1. 单元格值/公式内容发生直接变化 → 标记为 MODIFY
  2. 新增/删除行/列 → 标记为 ADD/DELETE

被动更新规则：
  1. 单元格包含公式，且公式文本未变但值变 → 标记为 AUTO_UPDATE
  2. 在 diff 报告中折叠显示或灰色标注
```

**实现方案**：
1. 解析每个单元格的公式文本（`openpyxl` 的 `data_only=False`）
2. 建立「公式单元格 → 引用单元格」的依赖图
3. diff 时：公式文本不变 → 值变化标记为被动；公式文本变化 → 标记为主动

## 5.4 阶段5b：Web 前后端

### Web 后端（FastAPI）

| 接口 | 说明 |
|------|------|
| `GET /api/diff?old=<commit>&new=<commit>` | 获取版本 diff |
| `GET /api/log?path=<file>` | 获取文件历史 |
| `GET /api/export/<commit>` | 导出历史版本 |
| `POST /api/upload` | 上传 Excel（触发 Git commit） |

### Web 前端（纯静态）

- 零依赖（无 React/Vue）
- 表格 diff 渲染（颜色区分：红=修改、绿=新增、灰=删除、黄=公式自动更新）
- 版本切换下拉框
- Sheet 切换标签
- 搜索框（按单元格内容/位置过滤）
- 一键回滚按钮

## 5.5 项目亮点

| 亮点 | 说明 |
|------|------|
| Git 原生集成 | 替代 `git diff` Excel 乱码，输出结构化文本 diff |
| AI+人工双源溯源 | 统一展示人工操作和 AI Agent 修改的历史 |
| 公式智能降噪 | 区分主动修改与公式级联被动变化 |
| 零依赖前端 | 纯静态 HTML，无需构建、无需服务端渲染 |
| 全格式兼容 | Excel (.xlsx/.xlsm) + CSV 统一 diff 体验 |

## 5.6 验证标准

- [ ] `excel-diff diff` 输出结构化 JSON diff
- [ ] Git diff 驱动注册后 `git diff` 不再乱码
- [ ] 公式降噪正确区分主动/被动修改
- [ ] `excel-diff log` 展示完整版本历史
- [ ] 版本回滚导出文件与原始 Excel 一致
- [ ] Web 前端正确渲染 diff 表格
- [ ] AI 操作日志合并后 diff 标注正确
