# excel-diff 设计补充方案

基于 `docs/research/` 调研文档，分析需补充功能及设计。

## 一、当前实现 vs 需求差距

### 已实现 ✓
- `diff_files/sheets/range` 基础 diff
- `compute_cell_diffs` 单元格级对比
- `install_git_driver` Git 集成注册
- hash 快速检查

### 未实现（按优先级）

| 功能 | 来源需求 | 实现文件 |
|------|---------|---------|
| 公式依赖追踪 | 研究文档核心亮点 | `formula_tracker.rs` |
| Passive diff type | 区分主动/被动修改 | `diff_core.rs` |
| Web 输出格式 | 阶段5 Web API | `web_output.rs` |

---

## 二、公式依赖追踪设计

### 问题
修改 A2 → A5=SUM(A2:A4) 的值自动变化，当前实现会把 A5 也标记为 Modify，diff 爆炸。

### 解决方案

```rust
// src/formula_tracker.rs
pub struct FormulaTracker {
    /// key: cell_ref "A5", value: referenced cells {"A2", "A3", "A4"}
    dependencies: HashMap<String, HashSet<String>>,
}

impl FormulaTracker {
    pub fn build_from_sheet(sheet: &SheetData) -> Self;
    pub fn is_passive_change(
        &self,
        cell_ref: &str,
        old_formula: Option<&str>,
        new_formula: Option<&str>,
    ) -> bool;
}
```

### 算法

```rust
// 公式文本相同，值变化 → Passive
// 公式文本变化 → Modify
// 无公式 → 正常值对比
```

### 集成

```rust
// diff_core.rs 修改
pub fn compute_cell_diffs(old: &SheetData, new: &SheetData) -> Vec<CellDiff> {
    let tracker = FormulaTracker::build_from_sheet(new);
    // ... 对比逻辑
    // 判断 diff_type 时调用 tracker.is_passive_change()
}
```

---

## 三、Web 输出格式设计

### API 端点

```rust
// src/web_output.rs
pub fn to_api_response(diff: &FileDiff) -> serde_json::Value;
pub fn to_html_table(diff: &FileDiff) -> String;
```

### 输出格式

```json
{
  "success": true,
  "file_hash_match": false,
  "summary": {
    "adds": 2,
    "deletes": 1,
    "modifies": 3,
    "passives": 2,
    "total_changes": 8
  },
  "sheets": [
    {
      "sheet_name": "Sheet1",
      "row_count_diff": 0,
      "col_count_diff": 0,
      "cell_diffs": [
        {
          "cell_ref": "A2",
          "row": 1,
          "col": 0,
          "diff_type": "Modify",
          "old_value": "100",
          "new_value": "200",
          "old_formula": null,
          "new_formula": null
        },
        {
          "cell_ref": "A5",
          "row": 4,
          "col": 0,
          "diff_type": "Passive",
          "old_value": "300",
          "new_value": "400",
          "old_formula": "=SUM(A2:A4)",
          "new_formula": "=SUM(A2:A4)"
        }
      ]
    }
  ]
}
```

### 依赖链文本

对于 Passive 单元格，可选输出依赖链：

```json
{
  "cell_ref": "A5",
  "diff_type": "Passive",
  "dependency_chain": "A2 → A5(SUM) → B5 → C5"
}
```

---

## 五、DiffType 枚举扩展

当前 `DiffType` 定义于 `excel-core`:

```rust
// excel-core/src/types.rs
pub enum DiffType {
    Add,
    Delete,
    Modify,
    Passive,  // 新增：被动更新（公式级联）
    NoChange,
}
```

---

## 六、文件结构（阶段5完成后）

```
crates/excel-diff/
├── src/
│   ├── lib.rs
│   ├── diff_core.rs          # 核心 diff 逻辑
│   ├── formula_tracker.rs     # 公式依赖追踪 [待实现]
│   ├── git_driver.rs         # Git 集成
│   └── web_output.rs         # Web 输出格式 [待实现]
└── docs/
    ├── research/              # 调研文档
    └── design.md              # 本文档
```

---

## 七、验证标准

- [ ] `compute_cell_diffs` 正确区分 Modify/Passive
- [ ] `web_output.rs` 输出符合 API 格式
- [ ] 单元测试覆盖所有新增逻辑