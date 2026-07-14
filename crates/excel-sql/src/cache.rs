//! Query cache with LRU eviction and TTL expiry.
//!
//! Cache keys are `{path}|{sql}` — combining the file path and the SQL query.
//! On insertion, if the number of entries exceeds `max_entries`, the
//! least-recently-accessed entry is evicted. On retrieval, entries whose age
//! exceeds `ttl_seconds` are expired and removed.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::converter::QueryResult;

/// Configuration knobs for `QueryCache`.
pub struct QueryCacheConfig {
    /// Maximum number of cached entries before LRU eviction kicks in.
    pub max_entries: usize,
    /// Entry lifetime in seconds. Entries older than this are considered stale.
    pub ttl_seconds: u64,
    /// Built-in invalidation (reserved for future use, e.g. file-watch hooks).
    pub auto_invalidate: bool,
}

impl Default for QueryCacheConfig {
    fn default() -> Self {
        Self {
            max_entries: 100,
            ttl_seconds: 300,
            auto_invalidate: true,
        }
    }
}

struct CacheEntry {
    result: QueryResult,
    created_at: Instant,
    last_accessed: Instant,
}

/// In-memory query cache backed by a `HashMap` with LRU eviction and TTL expiry.
///
/// Thread-safety is the caller's responsibility — typically wrap in `Mutex`.
pub struct QueryCache {
    entries: HashMap<String, CacheEntry>,
    config: QueryCacheConfig,
}

impl QueryCache {
    /// Create a new cache with the given configuration.
    pub fn new(config: QueryCacheConfig) -> Self {
        Self {
            entries: HashMap::new(),
            config,
        }
    }

    /// Look up a cached query result by key.
    ///
    /// Returns `None` when the entry is absent or its TTL has expired.
    /// On a cache hit the entry's `last_accessed` timestamp is updated (LRU promotion).
    pub fn get(&mut self, key: &str) -> Option<&QueryResult> {
        let now = Instant::now();
        let ttl = Duration::from_secs(self.config.ttl_seconds);

        if let Some(entry) = self.entries.get_mut(key) {
            if now.duration_since(entry.created_at) > ttl {
                self.entries.remove(key);
                return None;
            }
            entry.last_accessed = now;
            Some(&entry.result)
        } else {
            None
        }
    }

    /// Store a query result under the given key.
    ///
    /// If the cache is full the least-recently-accessed entry is evicted.
    pub fn put(&mut self, key: String, result: QueryResult) {
        let now = Instant::now();

        if self.entries.len() >= self.config.max_entries {
            self.evict_lru();
        }

        self.entries.insert(
            key,
            CacheEntry {
                result,
                created_at: now,
                last_accessed: now,
            },
        );
    }

    /// Remove every cache entry whose key starts with `path` followed by `|`.
    ///
    /// Call this when the underlying Excel file has been modified so stale
    /// results are not served for the same path.
    pub fn invalidate_file(&mut self, path: &str) {
        let prefix = format!("{path}|");
        self.entries.retain(|k, _| !k.starts_with(&prefix));
    }

    /// Remove all cached entries.
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Number of entries currently in the cache.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Build the canonical cache key from a file path and SQL string.
    pub fn make_key(path: &str, sql: &str) -> String {
        format!("{path}|{sql}")
    }

    // --- private helpers ---

    fn evict_lru(&mut self) {
        let mut oldest_key: Option<String> = None;
        let mut oldest_time = Instant::now();

        for (k, v) in &self.entries {
            if v.last_accessed <= oldest_time {
                oldest_time = v.last_accessed;
                oldest_key = Some(k.clone());
            }
        }

        if let Some(k) = oldest_key {
            self.entries.remove(&k);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use excel_types::{CellData, CellDataType};

    fn make_result(value: &str) -> QueryResult {
        QueryResult {
            columns: vec!["c0".to_string()],
            rows: vec![vec![CellData {
                value: Some(value.to_string()),
                data_type: CellDataType::String,
                formula: None,
            }]],
            row_count: 1,
        }
    }

    // ── basic put / get ──

    #[test]
    fn test_put_and_get() {
        let mut cache = QueryCache::new(QueryCacheConfig::default());
        let key = QueryCache::make_key("/a.xlsx", "SELECT * FROM t");
        cache.put(key.clone(), make_result("r1"));
        let res = cache.get(&key).expect("entry should be present");
        assert_eq!(res.rows[0][0].value.as_deref(), Some("r1"));
    }

    #[test]
    fn test_get_missing_key() {
        let mut cache = QueryCache::new(QueryCacheConfig::default());
        assert!(cache.get("nonexistent").is_none());
    }

    // ── TTL ──

    #[test]
    fn test_ttl_expiry() {
        let mut cache = QueryCache::new(QueryCacheConfig {
            ttl_seconds: 0,
            ..QueryCacheConfig::default()
        });
        let key = QueryCache::make_key("/f.xlsx", "SELECT *");
        cache.put(key.clone(), make_result("stale"));
        // TTL 0 means instantly expired
        assert!(cache.get(&key).is_none());
        assert!(cache.entries.is_empty());
    }

    // ── LRU eviction ──

    #[test]
    fn test_lru_eviction() {
        let mut cache = QueryCache::new(QueryCacheConfig {
            max_entries: 2,
            ..QueryCacheConfig::default()
        });
        let k1 = QueryCache::make_key("/f.xlsx", "SELECT 1");
        let k2 = QueryCache::make_key("/f.xlsx", "SELECT 2");
        let k3 = QueryCache::make_key("/f.xlsx", "SELECT 3");

        cache.put(k1.clone(), make_result("a"));
        cache.put(k2.clone(), make_result("b"));

        // Access k1 to make it more recent than k2
        cache.get(&k1);

        // k3 insertion should evict k2 (the LRU)
        cache.put(k3.clone(), make_result("c"));

        assert!(cache.get(&k1).is_some()); // still present
        assert!(cache.get(&k2).is_none()); // evicted
        assert!(cache.get(&k3).is_some()); // newly inserted
    }

    // ── invalidate_file ──

    #[test]
    fn test_invalidate_file() {
        let mut cache = QueryCache::new(QueryCacheConfig::default());
        let k1 = QueryCache::make_key("/f1.xlsx", "SELECT 1");
        let k2 = QueryCache::make_key("/f1.xlsx", "SELECT 2");
        let k3 = QueryCache::make_key("/f2.xlsx", "SELECT 1");

        cache.put(k1.clone(), make_result("a"));
        cache.put(k2.clone(), make_result("b"));
        cache.put(k3.clone(), make_result("c"));

        cache.invalidate_file("/f1.xlsx");

        assert!(cache.get(&k1).is_none());
        assert!(cache.get(&k2).is_none());
        assert!(cache.get(&k3).is_some()); // f2 unaffected
    }

    // ── clear / len / is_empty ──

    #[test]
    fn test_clear_and_len() {
        let mut cache = QueryCache::new(QueryCacheConfig::default());
        assert!(cache.is_empty());
        assert_eq!(cache.len(), 0);

        cache.put(QueryCache::make_key("/a.xlsx", "SELECT 1"), make_result("x"));
        assert_eq!(cache.len(), 1);
        assert!(!cache.is_empty());

        cache.clear();
        assert!(cache.is_empty());
    }

    // ── max_entries = 0 ──

    #[test]
    fn test_zero_capacity_insert_evicts_immediately() {
        let mut cache = QueryCache::new(QueryCacheConfig {
            max_entries: 0,
            ..QueryCacheConfig::default()
        });
        let key = QueryCache::make_key("/f.xlsx", "SELECT 1");
        cache.put(key.clone(), make_result("x"));
        // Inserted but immediately evicted (len >= max_entries)
        assert!(cache.get(&key).is_none());
        assert_eq!(cache.len(), 0);
    }
}
