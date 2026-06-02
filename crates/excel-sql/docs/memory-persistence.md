## DuckDB 集成最佳实践

针对你的场景（Excel 提供 SQL 查询），我整理了 DuckDB 集成的几个核心最佳实践：

### 1. 存储策略：内存 vs 持久化

DuckDB 支持两种模式，各有适用场景：

| 模式                            | 特点                         | 适用场景                          |
| ------------------------------- | ---------------------------- | --------------------------------- |
| **内存数据库** (`:memory:`)     | 进程退出即清空，启动快(~8ms) | 临时查询、单次分析、原型验证      |
| **持久化数据库** (`xxx.duckdb`) | 数据保存到磁盘，可跨会话复用 | 缓存 Excel 数据、重复查询同一文件 |

对于你的 Excel SQL 查询场景，**推荐混合策略**：

```rust
pub struct ExcelQueryEngine {
    // 持久化数据库用于缓存
    persistent_db: Option<duckdb::Connection>,
    // 可选：临时内存库用于单次查询
    temp_db: Option<duckdb::Connection>,
}

impl ExcelQueryEngine {
    /// 创建带缓存的引擎
    pub fn new_with_cache(cache_path: Option<&str>) -> Result<Self, AppError> {
        let persistent_db = match cache_path {
            Some(path) => Some(Connection::open(path)?),
            None => None,
        };
        Ok(Self { persistent_db, temp_db: None })
    }

    /// 加载 Excel 数据（优先使用缓存）
    pub fn load_excel(&mut self, name: &str, data: SheetData, use_cache: bool) -> Result<(), AppError> {
        let db = if use_cache && self.persistent_db.is_some() {
            self.persistent_db.as_ref().unwrap()
        } else {
            self.get_temp_db()?
        };
        load_sheet_to_db(db, name, &data, true)
    }
}
```

### 2. 是否需要查询后立即释放？

**答案是：取决于使用模式**。

#### 场景 A：单次临时查询 → 查询后释放

```rust
pub fn query_temp(sql: &str, data: SheetData) -> Result<QueryResult, AppError> {
    let conn = Connection::open_in_memory()?;  // 创建
    load_sheet_to_db(&conn, "data", &data, true)?;
    let result = execute_query(&conn, sql)?;
    // conn 离开作用域自动关闭，内存释放
    Ok(result)
}
```

#### 场景 B：重复查询同一 Excel → 保持连接，使用持久化缓存

```rust
pub struct CachedEngine {
    conn: Connection,  // 长生命周期，复用
    tables: Vec<String>,
}

impl CachedEngine {
    /// 加载后保持连接，后续查询直接使用
    pub fn load(&mut self, name: &str, data: SheetData) -> Result<(), AppError> {
        load_sheet_to_db(&self.conn, name, &data, true)?;
        self.tables.push(name.to_string());
        Ok(())
    }

    /// 查询已加载的表
    pub fn query(&self, sql: &str) -> Result<QueryResult, AppError> {
        // 直接使用已有的 conn，无需重新加载
        execute_query(&self.conn, sql)
    }
}
```

#### 场景 C：内存压力大 → 利用 DuckDB 自动溢出

DuckDB 支持自动将内存数据溢出到磁盘，无需手动干预：

```sql
-- 可配置内存上限（默认 80% 物理内存）
SET memory_limit = '4GB';
-- 设置临时目录用于溢出
SET temp_directory = '/tmp/duckdb_swap';
```

### 3. 并发与连接管理

DuckDB 的并发模型比较特殊：

**关键规则**：

- ✅ **单进程多线程**：支持，每个线程使用独立的 Connection
- ✅ **多进程只读**：支持
- ❌ **多进程写入**：不支持自动处理

```rust
// ✅ 正确：每线程一个连接
fn parallel_queries(conn: &Connection, queries: Vec<String>) {
    let handles: Vec<_> = queries.into_iter().map(|sql| {
        // 注意：不能共享 Connection，需要创建新连接
        let db = conn.try_clone().unwrap();
        thread::spawn(move || {
            let mut stmt = db.prepare(&sql).unwrap();
            // 执行查询...
        })
    }).collect();
}

// ❌ 错误：多线程共享同一连接
fn bad_parallel(conn: &Connection, queries: Vec<String>) {
    for sql in queries {
        thread::spawn(move || {
            conn.prepare(&sql)  // 编译错误：Connection 不能安全共享
        });
    }
}
```

### 4. 资源释放顺序（重要）

正确释放顺序：

```rust
/// 正确释放：逆序释放
pub fn cleanup_correctly(mut db: Database, conn: Connection, stmt: Statement) {
    // 1. 先释放 Statement
    drop(stmt);
    // 2. 再关闭 Connection
    drop(conn);
    // 3. 最后关闭 Database
    drop(db);
}

/// 错误示例：Database 先关闭会导致后续释放失败
pub fn cleanup_wrong(db: Database, conn: Connection) {
    drop(db);   // ❌ 先关闭 Database
    drop(conn); // 可能 panic 或泄漏
}
```

在 Rust 中，利用 RAII 和 Drop trait 可以自动处理，但要确保依赖关系正确。

### 5. 架构模式推荐

参考携程的实践，采用"内存工作集 + 定期快照"模式：

```rust
pub struct ExcelQueryService {
    // 内存工作集：热数据，查询速度快
    hot_conn: Connection,
    // 持久化存储：冷数据或缓存
    cold_storage: Option<Connection>,
}

impl ExcelQueryService {
    /// 定期将内存数据持久化
    pub fn snapshot(&mut self) -> Result<(), AppError> {
        if let Some(cold) = &self.cold_storage {
            // 将 hot_conn 中的表复制到持久化存储
            for table in self.list_tables()? {
                cold.execute_batch(&format!(
                    "CREATE OR REPLACE TABLE {} AS SELECT * FROM hot.{}",
                    table, table
                ))?;
            }
        }
        Ok(())
    }

    /// 从快照恢复（进程重启后）
    pub fn restore(&mut self) -> Result<(), AppError> {
        if let Some(cold) = &self.cold_storage {
            for table in self.list_cold_tables()? {
                cold.execute_batch(&format!(
                    "CREATE TABLE hot.{} AS SELECT * FROM cold.{}",
                    table, table
                ))?;
            }
        }
        Ok(())
    }
}
```

### 针对你的代码的具体改进建议

1. **添加连接池支持**：

```rust
pub struct ConnectionPool {
    connections: Vec<duckdb::Connection>,
    max_size: usize,
}

impl ConnectionPool {
    pub fn get_connection(&self) -> Result<duckdb::Connection, AppError> {
        // 从池中获取或创建新连接
    }
}
```

2. **添加缓存机制**：

```rust
pub struct CachedLoader {
    cache_dir: PathBuf,
    ttl: Duration,
}

impl CachedLoader {
    /// 将加载的 Excel 缓存为 DuckDB 文件
    pub fn load_with_cache(&self, name: &str, data: &SheetData) -> Result<(), AppError> {
        let cache_file = self.cache_dir.join(format!("{}.duckdb", name));
        if cache_file.exists() && self.is_valid(&cache_file) {
            // 直接使用缓存，跳过解析
            return Ok(());
        }
        // 解析并保存到缓存
        self.save_cache(&cache_file, data)
    }
}
```

3. **添加查询接口**（当前缺失的最关键部分）：

```rust
pub fn query(
    conn: &duckdb::Connection,
    sql: &str,
) -> Result<Vec<Vec<CellData>>, AppError> {
    let mut stmt = conn.prepare(sql)?;
    let columns = stmt.column_count();
    let rows = stmt.query_map([], |row| {
        let mut cells = Vec::with_capacity(columns);
        for i in 0..columns {
            cells.push(row_to_celldata(row, i)?);
        }
        Ok(cells)
    })?;
    rows.collect()
}
```
