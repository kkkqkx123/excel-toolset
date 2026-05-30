# 阶段5：Diff 子系统（excel-diff）

**目标**：实现独立的 Rust Diff 引擎，支持结构化 Excel 对比，可作为 Git diff 驱动和 API 后端。
**产出**：`excel-diff` crate，提供结构化 diff 计算、公式降噪、Git 集成、Web 前后端 API。

---

## 5.1 项目定位

`excel-diff` 是 workspace 中的独立 crate，专注于 Excel 内容对比：

| 维度 | excel-core | excel-diff |
|------|-----------|------------|
| 核心任务 | Excel 原子操作（读写/编辑） | Excel 对比 + 结构化 diff |
| 依赖关系 | 自包含 | 依赖 `excel-core` (仅读取相关类型) |
| 输出 | 操作结果 | 结构化 diff JSON |
| 用户场景 | AI Agent / 开发者 | Git 用户 / Web 用户 / 审计工具 |

## 5.2 架构

### 核心模块结构

```
crates/excel-diff/
├── Cargo.toml
└── src/
    ├── lib.rs                 # 公共 API
    ├── diff_core.rs          # Diff 核心算法
    ├── formula_tracker.rs    # 公式依赖追踪
    ├── git_driver.rs         # Git 集成
    └── web_output.rs         # Web 前端数据格式
```

### 依赖图

```
excel-diff → excel-core (仅 CellData、CellRef、SheetData 等读取类型)
excel-core -/-> excel-diff (核心写操作不依赖 diff，避免循环依赖)
```

## 5.3 核心 API

### 5.3.1 独立 Diff API

| 函数 | 说明 |
|------|------|
| `diff_files(old_path, new_path) -> Result<FileDiff>` | 全文件对比 |
| `diff_sheets(old_path, new_path, sheet) -> Result<SheetDiff>` | 指定 sheet 对比 |
| `diff_range(old_path, new_path, sheet, range) -> Result<RangeDiff>` | 指定区域对比 |

### 5.3.2 内存 Diff API

供写入操作后调用（由 CLI/HTTP 入口层调用）：

```rust
pub fn compute_diffs(
    old_data: &SheetData,
    new_data: &SheetData
) -> Vec<CellDiff>
```

### 5.3.3 Git 集成 API

```rust
pub fn install_git_driver() -> Result<()>
// 写入 .gitattributes: *.xlsx diff=excel-diff
// 写入 git config: diff.excel-diff.command = "excel-cli diff"
```

## 5.4 Diff 数据结构

```rust
pub enum DiffType {
    Add,       // 新增单元格/行
    Delete,    // 删除单元格/行
    Modify,    // 主动修改（值或公式文本变化）
    Passive,   // 被动更新（公式文本不变，值变化）
    NoChange,  // 无变化
}

pub struct CellDiff {
    pub row: u32,
    pub col: u16,
    pub cell_ref: String,
    pub diff_type: DiffType,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
    pub old_formula: Option<String>,
    pub new_formula: Option<String>,
}

pub struct FileDiff {
    pub file_hash_match: bool,
    pub sheet_names_changed: bool,
    pub sheets: Vec<SheetDiff>,
    pub summary: DiffSummary,
}

pub struct DiffSummary {
    pub adds: usize,
    pub deletes: usize,
    pub modifies: usize,
    pub passives: usize,
    pub total_changes: usize,
}
```

## 5.5 公式降噪算法

### 5.5.1 问题

修改单元格 A2（值 100 → 200），公式单元格 A5=`SUM(A2:A4)` 的值从 300 → 400 自动更新。区分：
- **主动修改**：A2 修改（红色标记）
- **被动更新**：A5 自动更新（黄色标记）

### 5.5.2 算法

1. **公式依赖图**：解析公式文本，建立 `A5 → {A2, A3, A4}` 的依赖关系
2. **公式文本对比**：
   - 公式文本相同 → `Passive`（被动）
   - 公式文本不同 → `Modify`（主动）
3. **无公式对比**：直接对比单元格值

### 5.5.3 实现步骤

```rust
fn compute_formula_passive(
    old_data: &SheetData,
    new_data: &SheetData,
    row: u32,
    col: u16
) -> DiffType {
    let old_cell = old_data.get(row, col);
    let new_cell = new_data.get(row, col);
    
    match (&old_cell.formula, &new_cell.formula) {
        (Some(f1), Some(f2)) if f1 == f2 => {
            if old_cell.value != new_cell.value {
                DiffType::Passive
            } else {
                DiffType::NoChange
            }
        }
        // 其余情况...
    }
}
```

## 5.6 CLI 接口（通过 excel-cli 调用）

```bash
# 独立 Diff 查询
excel-cli diff file old.xlsx new.xlsx [--sheet] [--range]

# Git 集成
excel-cli diff install-git-driver

# 查看历史
excel-cli diff log [--path <file>] [--limit 10]
excel-cli diff show <commit-hash>

# 版本回滚
excel-cli diff checkout <commit-hash> [--output <path>]
```

## 5.7 Web 前后端

### 5.7.1 Web API（通过 excel-http 调用）

| 接口 | 说明 |
|------|------|
| `GET /api/diff?old=<commit>&new=<commit>` | 获取版本 diff |
| `GET /api/log?path=<file>` | 获取文件历史 |
| `GET /api/export/<commit>` | 导出历史版本 |
| `POST /api/upload` | 上传 Excel（触发 Git commit） |

### 5.7.2 Web 前端

- 纯静态 HTML/JS/CSS，零依赖
- 表格 diff 渲染（红色=主动修改，黄色=被动更新，绿色=新增，灰色=删除）
- 版本切换下拉框
- Sheet 切换标签
- 搜索框（按单元格内容/位置过滤）
- 一键回滚按钮

### 5.7.3 前端示例

```json
// API 响应
{
  "success": true,
  "message": "Diff computed",
  "data": {
    "diff": {
      "file_hash_match": false,
      "sheets": [...],
      "summary": {...}
    }
  }
}
```

```html
<!-- 前端渲染 -->
<table class="diff-table">
  <tr style="color:red">
    <td>Sheet1!A2</td>
    <td>100 → 200</td>
    <td>主动修改</td>
  </tr>
  <tr style="color:yellow">
    <td>Sheet1!A5</td>
    <td>300 → 400</td>
    <td>被动更新 (公式: =SUM(A2:A4))</td>
  </tr>
</table>
```

## 5.8 验证标准

- [ ] `excel-diff diff_files` 输出结构化 JSON diff
- [ ] Git diff 驱动注册后 `git diff` 不再乱码
- [ ] 公式降噪正确区分主动/被动修改
- [ ] Web API 返回格式正确的 diff 数据
- [ ] 前端静态页面正确渲染 diff 表格
- [ ] 性能：10MB 文件 diff 时间 < 2s