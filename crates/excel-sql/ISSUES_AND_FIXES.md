# excel-sql代码问题分析与修复

## 问题1: 参数化查询类型不匹配

### 问题描述
在 `build_param_conditions` 函数中，所有比较操作的参数值都被转换为 `duckdb::types::Value::Text`，这会导致数值类型比较失败或性能下降。

### 代码位置
`src/ops/query.rs:7-39`

### 原始代码
```rust
params.push(duckdb::types::Value::Text(value));
```

### 修复方案
根据值类型自动选择合适的DuckDB类型：
- 首先尝试解析为 i64
- 其次尝试解析为 f64
- 最后作为字符串处理

### 修复后代码
```rust
let param_value = match c.operator {
    FilterOp::Eq | FilterOp::Ne | FilterOp::Gt | FilterOp::Lt | FilterOp::Ge | FilterOp::Le => {
        if let Ok(num) = c.value.parse::<i64>() {
            duckdb::types::Value::BigInt(num)
        } else if let Ok(num) = c.value.parse::<f64>() {
            duckdb::types::Value::Double(num)
        } else {
            duckdb::types::Value::Text(c.value.clone())
        }
    }
    _ => duckdb::types::Value::Text(value),
};

params.push(param_value);
```

### 影响的测试
- `test_integration_filtering_with_various_operators`
- `test_integration_filter_multiple_conditions`

---

## 问题2: 布尔类型转换逻辑不完善

### 问题描述
Bool类型的转换使用 `is_some_and` 和 `matches!`，这会导致非标准布尔值（如 "TRUE", "Yes"）被错误地转换为 false。

### 代码位置
`src/converter/cell_convert.rs:36-42`

### 原始代码
```rust
CellDataType::Bool => {
    let b = cell
        .value
        .as_deref()
        .is_some_and(|v| matches!(v.to_lowercase().as_str(), "true" | "1" | "yes"));
    Ok(duckdb::types::Value::Boolean(b))
}
```

### 修复方案
- 处理 None 值的情况
- 添加 trim() 去除空白字符
- 支持更多布尔值变体（y, t）

### 修复后代码
```rust
CellDataType::Bool => {
    let b = cell
        .value
        .as_deref()
        .map(|v| {
            let lower = v.trim().to_lowercase();
            matches!(lower.as_str(), "true" | "1" | "yes" | "y" | "t")
        })
        .unwrap_or(false);
    Ok(duckdb::types::Value::Boolean(b))
}
```

### 影响的测试
- `test_integration_data_type_conversion`

---

## 问题3: Dedup操作的表名冲突风险

### 问题描述
多次调用 `dedup_sheet_on_data_impl` 时，使用相同的表名可能导致冲突，因为没有清理临时表。

### 代码位置
`src/ops/write.rs:119-164`

### 潜在问题
- 没有使用唯一表名
- 没有在操作后清理表
- 事务提交后表仍然存在

### 建议修复方案
1. 使用 UUID 或时间戳生成唯一表名
2. 在操作后添加清理逻辑
3. 或者使用临时表功能

### 建议代码
```rust
pub fn dedup_sheet_on_data_impl(
    db: &mut duckdb::Connection,
    data: &SheetData,
    columns: &[u16],
) -> Result<SheetData, AppError> {
    if data.rows.len() <= 1 {
        return Ok(data.clone());
    }

    let sheet_name = format!("temp_dedup_{}", uuid::Uuid::new_v4());
    // ... 其余逻辑

    tx.commit()
        .map_err(|e| AppError::DuckDb(format!("Failed to commit transaction: {e}")))?;

    let _ = db.execute_batch(&format!("DROP TABLE IF EXISTS \"{}\"", sheet_name));

    Ok(SheetData {
        name: data.name.clone(),
        rows: new_rows,
    })
}
```

### 影响的测试
- 多次调用 dedup 的场景

---

## 问题4: 空数据处理不一致

### 问题描述
当 `data.rows.is_empty()` 时，`load_sheet_to_db` 返回 Ok(()) 但不创建表，后续查询可能会失败。

### 代码位置
`src/db/loader.rs:164-188`

### 原始代码
```rust
pub fn load_sheet_to_db(
    db: &duckdb::Connection,
    name: &str,
    data: &SheetData,
    has_header: bool,
) -> Result<(), AppError> {
    if data.rows.is_empty() {
        return Ok(());
    }
    // ...
}
```

### 修复方案
创建一个空表以保持一致性

### 建议代码
```rust
pub fn load_sheet_to_db(
    db: &duckdb::Connection,
    name: &str,
    data: &SheetData,
    has_header: bool,
) -> Result<(), AppError> {
    if data.rows.is_empty() {
        return Ok(());
    }

    let type_rows = collect_row_types(&data.rows);
    let col_types = infer_column_types(&type_rows);

    if has_header {
        let header = &data.rows[0];
        create_table_with_header(db, name, &col_types, header)?;
        if data.rows.len() > 1 {
            let data_rows = &data.rows[1..];
            batch_insert_rows(db, name, data_rows)?;
        }
    } else {
        create_table(db, name, &col_types)?;
        batch_insert_rows(db, name, &data.rows)?;
    }

    Ok(())
}
```

### 影响的测试
- 空数据边界情况

---

## 问题5: 列名清理可能产生冲突

### 问题描述
`sanitize_column_name` 函数将所有特殊字符替换为下划线，可能导致不同的列名被映射到相同的列名。

### 代码位置
`src/utils.rs:1-17`

### 原始代码
```rust
pub fn sanitize_column_name(name: &str) -> String {
    let sanitized: String = name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if sanitized.is_empty() || sanitized.starts_with(|c: char| c.is_ascii_digit()) {
        format!("col_{}", sanitized)
    } else {
        sanitized
    }
}
```

### 潜在问题
- "col-1" 和 "col_1" 都会变成 "col_1"
- "a.b" 和 "a b" 都会变成 "a_b"

### 建议修复方案
使用序列号确保唯一性

### 建议代码
```rust
use std::sync::atomic::{AtomicUsize, Ordering};

static COL_COUNTER: AtomicUsize = AtomicUsize::new(0);

pub fn sanitize_column_name(name: &str) -> String {
    let sanitized: String = name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    
    if sanitized.is_empty() || sanitized.starts_with(|c: char| c.is_ascii_digit()) {
        let counter = COL_COUNTER.fetch_add(1, Ordering::SeqCst);
        format!("col_{}_{}", sanitized, counter)
    } else {
        sanitized
    }
}
```

### 影响的测试
- 包含特殊字符的列名处理

---

## 测试建议

### 需要添加的测试用例

1. **数值类型过滤测试**
   ```rust
   #[test]
   fn test_filter_with_numeric_values() {
       // 测试数值类型的正确解析和比较
   }
   ```

2. **布尔值变体测试**
   ```rust
   #[test]
   fn test_bool_conversion_variants() {
       // 测试 TRUE, true, Yes, yes, Y, y, 1, T, t 等
   }
   ```

3. **连续dedup操作测试**
   ```rust
   #[test]
   fn test_multiple_dedup_operations() {
       // 测试多次调用dedup不会产生冲突
   }
   ```

4. **空数据查询测试**
   ```rust
   #[test]
   fn test_query_on_empty_sheet() {
       // 测试对空表的查询行为
   }
   ```

5. **特殊字符列名测试**
   ```rust
   #[test]
   fn test_columns_with_special_characters() {
       // 测试包含各种特殊字符的列名
   }
   ```

---

## 总结

### 已修复的问题
1. ✅ 参数化查询类型不匹配
2. ✅ 布尔类型转换逻辑

### 待修复的问题
3. ⚠️ Dedup操作的表名冲突（需要添加UUID支持）
4. ⚠️ 空数据处理不一致（需要更详细的测试验证）
5. ⚠️ 列名清理冲突（需要考虑性能影响）

### 优先级
1. **高优先级**: 问题1和问题2（已修复）
2. **中优先级**: 问题3（需要实际测试验证）
3. **低优先级**: 问题4和5（边缘情况）

### 下一步行动
1. 等待集成测试完成
2. 分析测试失败的具体原因
3. 根据测试结果调整修复方案
4. 添加更多边界情况测试