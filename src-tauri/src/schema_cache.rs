//! Per-connection cache for schema-listing operations with TTL + explicit
//! invalidation. Wraps the three read-only driver methods that populate the
//! sidebar and autocomplete.

use std::collections::HashMap;
use std::sync::RwLock;
use std::time::{Duration, Instant};

use uuid::Uuid;

use crate::db::driver::{ColumnInfo, Driver, SchemaInfo, TableInfo};
use crate::error::{AppError, AppResult};

pub const CACHE_TTL: Duration = Duration::from_secs(300);

pub struct SchemaCache {
    entries: RwLock<HashMap<Uuid, CachedSchema>>,
}

struct CachedSchema {
    schemas: Option<Vec<SchemaInfo>>,
    tables: HashMap<String, Vec<TableInfo>>,
    columns: HashMap<(String, String), Vec<ColumnInfo>>,
    fetched_at: Instant,
}

impl CachedSchema {
    fn empty() -> Self {
        Self {
            schemas: None,
            tables: HashMap::new(),
            columns: HashMap::new(),
            fetched_at: Instant::now(),
        }
    }
    fn is_fresh(&self) -> bool {
        self.fetched_at.elapsed() < CACHE_TTL
    }
}

impl SchemaCache {
    pub fn new() -> Self {
        Self {
            entries: RwLock::new(HashMap::new()),
        }
    }

    pub async fn schemas(&self, id: Uuid, driver: &dyn Driver) -> AppResult<Vec<SchemaInfo>> {
        // Fast path: read lock
        if let Ok(read) = self.entries.read() {
            if let Some(entry) = read.get(&id) {
                if entry.is_fresh() {
                    if let Some(cached) = &entry.schemas {
                        return Ok(cached.clone());
                    }
                }
            }
        }
        // Slow path: fetch + write lock
        let fresh = driver.list_schemas().await?;
        let mut write = self
            .entries
            .write()
            .map_err(|_| AppError::Other("cache lock poisoned".into()))?;
        let entry = write.entry(id).or_insert_with(CachedSchema::empty);
        entry.schemas = Some(fresh.clone());
        entry.fetched_at = Instant::now();
        Ok(fresh)
    }

    pub async fn tables(
        &self,
        id: Uuid,
        schema: &str,
        driver: &dyn Driver,
    ) -> AppResult<Vec<TableInfo>> {
        if let Ok(read) = self.entries.read() {
            if let Some(entry) = read.get(&id) {
                if entry.is_fresh() {
                    if let Some(cached) = entry.tables.get(schema) {
                        return Ok(cached.clone());
                    }
                }
            }
        }
        let fresh = driver.list_tables(schema).await?;
        let mut write = self
            .entries
            .write()
            .map_err(|_| AppError::Other("cache lock poisoned".into()))?;
        let entry = write.entry(id).or_insert_with(CachedSchema::empty);
        entry.tables.insert(schema.to_string(), fresh.clone());
        entry.fetched_at = Instant::now();
        Ok(fresh)
    }

    pub async fn columns(
        &self,
        id: Uuid,
        schema: &str,
        table: &str,
        driver: &dyn Driver,
    ) -> AppResult<Vec<ColumnInfo>> {
        let key = (schema.to_string(), table.to_string());
        if let Ok(read) = self.entries.read() {
            if let Some(entry) = read.get(&id) {
                if entry.is_fresh() {
                    if let Some(cached) = entry.columns.get(&key) {
                        return Ok(cached.clone());
                    }
                }
            }
        }
        let fresh = driver.list_columns(schema, table).await?;
        let mut write = self
            .entries
            .write()
            .map_err(|_| AppError::Other("cache lock poisoned".into()))?;
        let entry = write.entry(id).or_insert_with(CachedSchema::empty);
        entry.columns.insert(key, fresh.clone());
        entry.fetched_at = Instant::now();
        Ok(fresh)
    }

    pub fn invalidate(&self, id: Uuid) {
        if let Ok(mut write) = self.entries.write() {
            write.remove(&id);
        }
    }

    pub fn drop_entry(&self, id: Uuid) {
        self.invalidate(id);
    }
}

impl Default for SchemaCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::ddl::Dialect;
    use crate::db::driver::QueryResult;
    use async_trait::async_trait;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// Minimal Driver mock for cache testing. Counts calls to the three read-only
    /// methods the cache uses; every other required method panics if invoked.
    struct TestDriver {
        schemas_calls: AtomicUsize,
        tables_calls: AtomicUsize,
        columns_calls: AtomicUsize,
        schemas_return: Vec<SchemaInfo>,
        tables_return: Vec<TableInfo>,
        columns_return: Vec<ColumnInfo>,
    }

    impl TestDriver {
        fn new() -> Self {
            Self {
                schemas_calls: AtomicUsize::new(0),
                tables_calls: AtomicUsize::new(0),
                columns_calls: AtomicUsize::new(0),
                schemas_return: vec![SchemaInfo {
                    name: "public".into(),
                }],
                tables_return: vec![TableInfo {
                    name: "users".into(),
                    kind: "table".into(),
                }],
                columns_return: vec![],
            }
        }
    }

    #[async_trait]
    impl Driver for TestDriver {
        async fn list_schemas(&self) -> AppResult<Vec<SchemaInfo>> {
            self.schemas_calls.fetch_add(1, Ordering::SeqCst);
            Ok(self.schemas_return.clone())
        }
        async fn list_tables(&self, _schema: &str) -> AppResult<Vec<TableInfo>> {
            self.tables_calls.fetch_add(1, Ordering::SeqCst);
            Ok(self.tables_return.clone())
        }
        async fn list_columns(&self, _schema: &str, _table: &str) -> AppResult<Vec<ColumnInfo>> {
            self.columns_calls.fetch_add(1, Ordering::SeqCst);
            Ok(self.columns_return.clone())
        }
        async fn list_primary_keys(&self, _schema: &str, _table: &str) -> AppResult<Vec<String>> {
            unimplemented!()
        }
        async fn run_query(&self, _sql: &str) -> AppResult<QueryResult> {
            unimplemented!()
        }
        async fn close(&self) {}
        fn dialect(&self) -> Dialect {
            Dialect::Postgres
        }
    }

    #[tokio::test]
    async fn schemas_first_call_hits_driver() {
        let cache = SchemaCache::new();
        let driver = TestDriver::new();
        let id = Uuid::new_v4();
        let result = cache.schemas(id, &driver).await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(driver.schemas_calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn schemas_second_call_within_ttl_is_cached() {
        let cache = SchemaCache::new();
        let driver = TestDriver::new();
        let id = Uuid::new_v4();
        cache.schemas(id, &driver).await.unwrap();
        cache.schemas(id, &driver).await.unwrap();
        cache.schemas(id, &driver).await.unwrap();
        assert_eq!(driver.schemas_calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn tables_cached_per_schema() {
        let cache = SchemaCache::new();
        let driver = TestDriver::new();
        let id = Uuid::new_v4();
        cache.tables(id, "public", &driver).await.unwrap();
        cache.tables(id, "public", &driver).await.unwrap();
        cache.tables(id, "atlas", &driver).await.unwrap();
        assert_eq!(driver.tables_calls.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn columns_cached_per_schema_and_table() {
        let cache = SchemaCache::new();
        let driver = TestDriver::new();
        let id = Uuid::new_v4();
        cache.columns(id, "public", "users", &driver).await.unwrap();
        cache.columns(id, "public", "users", &driver).await.unwrap();
        cache
            .columns(id, "public", "orders", &driver)
            .await
            .unwrap();
        assert_eq!(driver.columns_calls.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn invalidate_forces_refetch() {
        let cache = SchemaCache::new();
        let driver = TestDriver::new();
        let id = Uuid::new_v4();
        cache.schemas(id, &driver).await.unwrap();
        cache.invalidate(id);
        cache.schemas(id, &driver).await.unwrap();
        assert_eq!(driver.schemas_calls.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn invalidate_drops_all_slices() {
        let cache = SchemaCache::new();
        let driver = TestDriver::new();
        let id = Uuid::new_v4();
        cache.schemas(id, &driver).await.unwrap();
        cache.tables(id, "public", &driver).await.unwrap();
        cache.columns(id, "public", "users", &driver).await.unwrap();
        cache.invalidate(id);
        cache.schemas(id, &driver).await.unwrap();
        cache.tables(id, "public", &driver).await.unwrap();
        cache.columns(id, "public", "users", &driver).await.unwrap();
        assert_eq!(driver.schemas_calls.load(Ordering::SeqCst), 2);
        assert_eq!(driver.tables_calls.load(Ordering::SeqCst), 2);
        assert_eq!(driver.columns_calls.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn drop_entry_removes_connection() {
        let cache = SchemaCache::new();
        let driver = TestDriver::new();
        let id = Uuid::new_v4();
        cache.schemas(id, &driver).await.unwrap();
        cache.drop_entry(id);
        cache.schemas(id, &driver).await.unwrap();
        assert_eq!(driver.schemas_calls.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn different_connections_have_separate_caches() {
        let cache = SchemaCache::new();
        let driver = TestDriver::new();
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();
        cache.schemas(id1, &driver).await.unwrap();
        cache.schemas(id2, &driver).await.unwrap();
        cache.schemas(id1, &driver).await.unwrap();
        cache.schemas(id2, &driver).await.unwrap();
        assert_eq!(driver.schemas_calls.load(Ordering::SeqCst), 2);
    }
}
