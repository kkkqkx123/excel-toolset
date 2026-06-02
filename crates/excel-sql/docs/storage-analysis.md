这是个很好的问题。对于**Excel需要多次更新**的场景，持久化的策略需要重新评估。

## 核心判断标准

是否需要持久化取决于**数据更新频率**与**查询频率**的关系：

| 场景       | 更新频率      | 查询/更新比 | 推荐策略                |
| ---------- | ------------- | ----------- | ----------------------- |
| 实时报表   | 每分钟        | > 100:1     | ✅ 持久化 + 增量更新    |
| 数据分析   | 每小时        | > 10:1      | ✅ 持久化，定期全量刷新 |
| 交互式查询 | 用户触发更新  | 1:1 ~ 5:1   | ⚠️ 可选，看数据量       |
| 频繁更新   | 每秒/每查询前 | < 1:1       | ❌ 纯内存，查询前加载   |

## 针对你的 Excel 场景分析

### 情况1：同一 Excel 文件，多次小改动

```rust
// 用户行为：打开 → 查询 → 改2个单元格 → 保存 → 再查询
// 更新频率低，查询次数多 → 适合持久化
```

**策略**：持久化 + 版本标记

```rust
pub struct VersionedCache {
    conn: Connection,
    file_path: PathBuf,
    last_modified: SystemTime,
    data_hash: u64,
}

impl VersionedCache {
    pub fn load_or_refresh(&mut self, excel_path: &Path) -> Result<(), AppError> {
        let metadata = fs::metadata(excel_path)?;
        let modified = metadata.modified()?;
        let current_hash = self.compute_hash(excel_path)?;

        // 仅在文件变化时重新加载
        if modified > self.last_modified || current_hash != self.data_hash {
            self.refresh_cache(excel_path)?;
            self.last_modified = modified;
            self.data_hash = current_hash;
        }
        Ok(())
    }

    fn refresh_cache(&mut self, path: &Path) -> Result<(), AppError> {
        // 方案A: 删除旧表，重建
        self.conn.execute_batch("DROP TABLE IF EXISTS data")?;
        let new_data = parse_excel(path)?;
        load_sheet_to_db(&self.conn, "data", &new_data, true)?;

        // 方案B: 使用 REPLACE（更高效）
        // CREATE OR REPLACE TABLE data AS SELECT * FROM new_data
        Ok(())
    }
}
```

### 情况2：每次查询都可能是新数据

```rust
// 用户行为：上传新Excel → 查询 → 上传另一个Excel → 查询
// 每次都是全新数据，旧缓存无意义 → 不需要持久化
```

**策略**：纯内存，用完即弃

```rust
pub fn query_uploaded_excel(data: SheetData, sql: &str) -> Result<QueryResult, AppError> {
    let conn = Connection::open_in_memory()?;  // 每次新建
    load_sheet_to_db(&conn, "data", &data, true)?;
    let result = execute_query(&conn, sql)?;
    // conn 自动关闭，数据释放
    Ok(result)
}
```

### 情况3：混合场景（最常见）

```rust
// 用户有多个Excel文件，部分经常查，部分偶尔查
// 例如：销售数据月报（经常查）+ 临时上传的数据（查一次）
```

**策略**：LRU 缓存 + 自动淘汰

```rust
pub struct LruExcelCache {
    cache_dir: PathBuf,
    max_size: usize,  // 最多缓存几个文件
    access_times: HashMap<String, Instant>,
}

impl LruExcelCache {
    pub fn get_or_load(&mut self, key: &str, loader: impl FnOnce() -> SheetData) -> Result<Connection, AppError> {
        // 更新访问时间
        self.access_times.insert(key.to_string(), Instant::now());

        let cache_path = self.cache_dir.join(format!("{}.duckdb", key));

        if cache_path.exists() && self.is_fresh(&cache_path) {
            // 命中缓存，直接打开
            return Ok(Connection::open(&cache_path)?);
        }

        // 未命中，检查是否需要淘汰
        if self.cache_dir.read_dir()?.count() >= self.max_size {
            self.evict_least_recently_used()?;
        }

        // 加载新数据到缓存
        let data = loader();
        let conn = Connection::open(&cache_path)?;
        load_sheet_to_db(&conn, "data", &data, true)?;
        Ok(conn)
    }

    fn evict_least_recently_used(&mut self) -> Result<(), AppError> {
        // 删除最久未访问的缓存文件
        if let Some((oldest_key, _)) = self.access_times.iter()
            .min_by_key(|(_, time)| *time) {
            let path = self.cache_dir.join(format!("{}.duckdb", oldest_key));
            let _ = fs::remove_file(path);
            self.access_times.remove(oldest_key);
        }
        Ok(())
    }
}
```

## 更新后是否删除旧缓存？

### 答案：**不需要立即删除，但要标记为脏**

```rust
pub struct CacheEntry {
    conn: Connection,
    is_dirty: bool,
    version: u32,
}

impl CacheManager {
    /// 更新 Excel 数据
    pub fn update_excel(&mut self, name: &str, new_data: SheetData) -> Result<(), AppError> {
        if let Some(entry) = self.caches.get_mut(name) {
            // 方案1: 标记为脏，延迟更新（推荐）
            entry.is_dirty = true;

            // 方案2: 立即更新（如果查询频繁）
            // self.refresh_cache(name, new_data)?;
        } else {
            // 新文件，直接加载
            self.load_new(name, new_data)?;
        }
        Ok(())
    }

    /// 查询时检查并刷新脏缓存
    pub fn query(&mut self, name: &str, sql: &str) -> Result<QueryResult, AppError> {
        let entry = self.caches.get_mut(name).ok_or(AppError::TableNotFound)?;

        if entry.is_dirty {
            // 查询前刷新脏数据
            self.refresh_cache(name, self.get_source_data(name)?)?;
            entry.is_dirty = false;
        }

        execute_query(&entry.conn, sql)
    }
}
```

### 为什么不要立即删除？

1. **旧缓存可能正在被使用**：如果有长时间运行的查询正在读取
2. **写时复制效率低**：DuckDB 不支持行级锁，删除重建成本高
3. **可以双缓冲**：保留旧数据直到新数据就绪

```rust
/// 无锁更新模式（推荐）
pub fn atomic_update(&mut self, name: &str, new_data: SheetData) -> Result<(), AppError> {
    // 1. 在新连接中构建新数据
    let new_conn = Connection::open_in_memory()?;
    load_sheet_to_db(&new_conn, name, &new_data, true)?;

    // 2. 原子替换（Rust 的 Arc 交换）
    let old = self.caches.insert(name.to_string(), Arc::new(new_conn));

    // 3. 旧连接会在最后一个引用释放后自动清理
    drop(old);
    Ok(())
}
```

## 最终建议矩阵

| Excel 更新频率 | 数据量  | 并发查询 | 推荐方案                 |
| -------------- | ------- | -------- | ------------------------ |
| 低（天级）     | 任意    | 高       | ✅ 持久化缓存 + 版本检测 |
| 中（小时级）   | < 100MB | 中       | ✅ 持久化 + 主动刷新     |
| 中（小时级）   | > 1GB   | 低       | ⚠️ 内存 + 查询时加载     |
| 高（分钟级）   | 任意    | 任意     | ❌ 纯内存，不用持久化    |
| 一次性查询     | 任意    | 低       | ❌ 纯内存，用完即弃      |

### 实用建议

对于大多数 Excel SQL 查询场景（数据分析师的工作流），**推荐采用"智能持久化"**：

```rust
pub fn smart_load(
    excel_path: &Path,
    force_reload: bool,
) -> Result<Connection, AppError> {
    let cache_path = get_cache_path(excel_path);

    // 自动判断是否需要刷新
    let should_reload = force_reload
        || !cache_path.exists()
        || is_excel_newer_than_cache(excel_path, &cache_path)
        || get_cache_size(&cache_path) > 1024 * 1024 * 500; // 500MB 自动清理

    if should_reload {
        // 重新加载并缓存
        load_excel_to_cache(excel_path, &cache_path)?;
    }

    Connection::open(&cache_path)
}
```

这样既获得了查询性能（持久化），又确保了数据新鲜度（自动刷新），还控制了存储成本（大小限制）。
