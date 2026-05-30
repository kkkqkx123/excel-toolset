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
| Web 输出格式 | 阶段5 Web API | `api_response.rs` |

// src/api_response.rs

  └── api_response.rs         # API 响应生成 [已实现]

- [x] `api_response.rs` 输出符合 API 格式
- [ ] 单元测试覆盖所有新增逻辑