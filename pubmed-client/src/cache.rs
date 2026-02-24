use moka::future::Cache as MokaCache;
use std::hash::Hash;
use std::time::Duration;
use tracing::{debug, info};

use crate::pmc::models::PmcFullText;

/// Selects which storage backend to use for caching
#[derive(Debug, Clone)]
pub enum CacheBackendConfig {
    /// In-memory cache using Moka (default)
    Memory,
    /// Redis-backed persistent cache
    ///
    /// Requires the `cache-redis` feature.
    #[cfg(feature = "cache-redis")]
    Redis {
        /// Redis connection URL, e.g. `"redis://127.0.0.1/"`
        url: String,
    },
    /// SQLite-backed persistent cache
    ///
    /// Requires the `cache-sqlite` feature.
    /// Not supported on WASM targets.
    #[cfg(feature = "cache-sqlite")]
    Sqlite {
        /// Path to the SQLite database file
        path: std::path::PathBuf,
    },
}

impl Default for CacheBackendConfig {
    fn default() -> Self {
        Self::Memory
    }
}

/// Configuration for response caching
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// Maximum number of items to store (used by the memory backend)
    pub max_capacity: u64,
    /// Time-to-live for cached items
    pub time_to_live: Duration,
    /// Which storage backend to use
    pub backend: CacheBackendConfig,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_capacity: 1000,
            time_to_live: Duration::from_secs(7 * 24 * 60 * 60), // 7 days
            backend: CacheBackendConfig::default(),
        }
    }
}

// ---------------------------------------------------------------------------
// Memory backend
// ---------------------------------------------------------------------------

/// In-memory cache backed by Moka
#[derive(Clone)]
pub struct MemoryCache<K, V> {
    cache: MokaCache<K, V>,
}

impl<K, V> MemoryCache<K, V>
where
    K: Hash + Eq + Clone + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    pub fn new(config: &CacheConfig) -> Self {
        let cache = MokaCache::builder()
            .max_capacity(config.max_capacity)
            .time_to_live(config.time_to_live)
            .build();
        Self { cache }
    }

    pub async fn get(&self, key: &K) -> Option<V> {
        let result = self.cache.get(key).await;
        if result.is_some() {
            debug!("Cache hit");
        } else {
            debug!("Cache miss");
        }
        result
    }

    pub async fn insert(&self, key: K, value: V) {
        self.cache.insert(key, value).await;
        info!("Item cached");
    }

    pub async fn clear(&self) {
        self.cache.invalidate_all();
        info!("Cache cleared");
    }

    pub fn entry_count(&self) -> u64 {
        self.cache.entry_count()
    }

    pub async fn sync(&self) {
        self.cache.run_pending_tasks().await;
    }
}

// ---------------------------------------------------------------------------
// Redis backend (feature = "cache-redis")
// ---------------------------------------------------------------------------

/// Redis-backed cache using JSON serialization.
///
/// TTL is applied per entry via `SET … EX`.  Each cache operation opens a new
/// multiplexed connection; for high-throughput use cases consider a connection
/// pool on top of this client.
///
/// `entry_count()` always returns 0 because a synchronous DBSIZE call is not
/// feasible here; use the memory backend if you need accurate counts.
#[cfg(feature = "cache-redis")]
#[derive(Clone)]
pub struct RedisCache {
    client: redis::Client,
    ttl: Duration,
}

#[cfg(feature = "cache-redis")]
impl RedisCache {
    pub fn new(url: &str, ttl: Duration) -> Result<Self, redis::RedisError> {
        let client = redis::Client::open(url)?;
        Ok(Self { client, ttl })
    }

    pub async fn get(&self, key: &str) -> Option<PmcFullText> {
        use redis::AsyncCommands;
        let mut conn = self.client.get_multiplexed_async_connection().await.ok()?;
        let json: String = conn.get(key).await.ok()?;
        if json.is_empty() {
            return None;
        }
        let value = serde_json::from_str(&json).ok();
        if value.is_some() {
            debug!("Cache hit (Redis)");
        } else {
            debug!("Cache miss (Redis): deserialization failed");
        }
        value
    }

    pub async fn insert(&self, key: String, value: &PmcFullText) {
        use redis::AsyncCommands;
        let Ok(mut conn) = self.client.get_multiplexed_async_connection().await else {
            return;
        };
        let Ok(json) = serde_json::to_string(value) else {
            return;
        };
        let ttl_secs = self.ttl.as_secs();
        let _: Result<(), _> = conn.set_ex(key, json, ttl_secs).await;
        info!("Item cached (Redis)");
    }

    pub async fn clear(&self) {
        let Ok(mut conn) = self.client.get_multiplexed_async_connection().await else {
            return;
        };
        let _: Result<(), _> = redis::cmd("FLUSHDB").query_async(&mut conn).await;
        info!("Cache cleared (Redis)");
    }

    /// Always returns 0; Redis does not provide a per-prefix key count without
    /// a full scan.
    pub fn entry_count(&self) -> u64 {
        0
    }

    pub async fn sync(&self) {
        // No-op for Redis
    }
}

// ---------------------------------------------------------------------------
// SQLite backend (feature = "cache-sqlite")
// ---------------------------------------------------------------------------

/// SQLite-backed cache using JSON serialization.
///
/// The database is created automatically if it does not exist.
/// Expired entries are not purged automatically; call [`SqliteCache::clear`]
/// or implement a periodic cleanup if storage space matters.
///
/// `entry_count()` returns the number of non-expired entries via a
/// non-blocking `try_lock`; it returns 0 if the mutex is currently held.
///
/// Not available on WASM targets.
#[cfg(feature = "cache-sqlite")]
#[derive(Clone)]
pub struct SqliteCache {
    conn: std::sync::Arc<std::sync::Mutex<rusqlite::Connection>>,
    ttl: Duration,
}

#[cfg(feature = "cache-sqlite")]
impl SqliteCache {
    pub fn new(path: &std::path::Path, ttl: Duration) -> rusqlite::Result<Self> {
        let conn = rusqlite::Connection::open(path)?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS pmc_cache (
                key        TEXT    PRIMARY KEY,
                value      TEXT    NOT NULL,
                expires_at INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_pmc_cache_expires ON pmc_cache (expires_at);",
        )?;
        Ok(Self {
            conn: std::sync::Arc::new(std::sync::Mutex::new(conn)),
            ttl,
        })
    }

    fn now_secs() -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64
    }

    pub async fn get(&self, key: &str) -> Option<PmcFullText> {
        let key = key.to_owned();
        let conn = std::sync::Arc::clone(&self.conn);
        tokio::task::spawn_blocking(move || {
            let now = Self::now_secs();
            let guard = conn.lock().unwrap();
            let result: rusqlite::Result<String> = guard.query_row(
                "SELECT value FROM pmc_cache WHERE key = ?1 AND expires_at > ?2",
                rusqlite::params![key, now],
                |row| row.get(0),
            );
            match result {
                Ok(json) => {
                    let value = serde_json::from_str(&json).ok();
                    if value.is_some() {
                        debug!("Cache hit (SQLite)");
                    } else {
                        debug!("Cache miss (SQLite): deserialization failed");
                    }
                    value
                }
                Err(_) => {
                    debug!("Cache miss (SQLite)");
                    None
                }
            }
        })
        .await
        .unwrap_or(None)
    }

    pub async fn insert(&self, key: String, value: PmcFullText) {
        let conn = std::sync::Arc::clone(&self.conn);
        let ttl = self.ttl;
        tokio::task::spawn_blocking(move || {
            let Ok(json) = serde_json::to_string(&value) else {
                return;
            };
            let expires_at = Self::now_secs() + ttl.as_secs() as i64;
            let guard = conn.lock().unwrap();
            let _ = guard.execute(
                "INSERT OR REPLACE INTO pmc_cache (key, value, expires_at) VALUES (?1, ?2, ?3)",
                rusqlite::params![key, json, expires_at],
            );
            info!("Item cached (SQLite)");
        })
        .await
        .ok();
    }

    pub async fn clear(&self) {
        let conn = std::sync::Arc::clone(&self.conn);
        tokio::task::spawn_blocking(move || {
            let guard = conn.lock().unwrap();
            let _ = guard.execute("DELETE FROM pmc_cache", []);
            info!("Cache cleared (SQLite)");
        })
        .await
        .ok();
    }

    pub fn entry_count(&self) -> u64 {
        let now = Self::now_secs();
        if let Ok(guard) = self.conn.try_lock() {
            guard
                .query_row(
                    "SELECT COUNT(*) FROM pmc_cache WHERE expires_at > ?1",
                    rusqlite::params![now],
                    |row| row.get::<_, i64>(0),
                )
                .map(|c| c as u64)
                .unwrap_or(0)
        } else {
            0
        }
    }

    pub async fn sync(&self) {
        // No-op for SQLite
    }
}

// ---------------------------------------------------------------------------
// Unified PmcCache enum
// ---------------------------------------------------------------------------

/// PMC response cache that dispatches to the configured backend.
#[derive(Clone)]
pub enum PmcCache {
    Memory(MemoryCache<String, PmcFullText>),
    #[cfg(feature = "cache-redis")]
    Redis(RedisCache),
    #[cfg(feature = "cache-sqlite")]
    Sqlite(SqliteCache),
}

impl PmcCache {
    pub async fn get(&self, key: &str) -> Option<PmcFullText> {
        match self {
            PmcCache::Memory(c) => c.get(&key.to_owned()).await,
            #[cfg(feature = "cache-redis")]
            PmcCache::Redis(c) => c.get(key).await,
            #[cfg(feature = "cache-sqlite")]
            PmcCache::Sqlite(c) => c.get(key).await,
        }
    }

    pub async fn insert(&self, key: String, value: PmcFullText) {
        match self {
            PmcCache::Memory(c) => c.insert(key, value).await,
            #[cfg(feature = "cache-redis")]
            PmcCache::Redis(c) => c.insert(key, &value).await,
            #[cfg(feature = "cache-sqlite")]
            PmcCache::Sqlite(c) => c.insert(key, value).await,
        }
    }

    pub async fn clear(&self) {
        match self {
            PmcCache::Memory(c) => c.clear().await,
            #[cfg(feature = "cache-redis")]
            PmcCache::Redis(c) => c.clear().await,
            #[cfg(feature = "cache-sqlite")]
            PmcCache::Sqlite(c) => c.clear().await,
        }
    }

    pub fn entry_count(&self) -> u64 {
        match self {
            PmcCache::Memory(c) => c.entry_count(),
            #[cfg(feature = "cache-redis")]
            PmcCache::Redis(c) => c.entry_count(),
            #[cfg(feature = "cache-sqlite")]
            PmcCache::Sqlite(c) => c.entry_count(),
        }
    }

    pub async fn sync(&self) {
        match self {
            PmcCache::Memory(c) => c.sync().await,
            #[cfg(feature = "cache-redis")]
            PmcCache::Redis(c) => c.sync().await,
            #[cfg(feature = "cache-sqlite")]
            PmcCache::Sqlite(c) => c.sync().await,
        }
    }
}

// ---------------------------------------------------------------------------
// Factory
// ---------------------------------------------------------------------------

/// Create a [`PmcCache`] from configuration.
///
/// Falls back to the memory backend if the configured backend cannot be
/// initialised (error is logged via `tracing::error!`).
pub fn create_cache(config: &CacheConfig) -> PmcCache {
    match &config.backend {
        CacheBackendConfig::Memory => PmcCache::Memory(MemoryCache::new(config)),
        #[cfg(feature = "cache-redis")]
        CacheBackendConfig::Redis { url } => match RedisCache::new(url, config.time_to_live) {
            Ok(c) => PmcCache::Redis(c),
            Err(e) => {
                tracing::error!("Failed to create Redis cache, falling back to memory: {e}");
                PmcCache::Memory(MemoryCache::new(config))
            }
        },
        #[cfg(feature = "cache-sqlite")]
        CacheBackendConfig::Sqlite { path } => match SqliteCache::new(path, config.time_to_live) {
            Ok(c) => PmcCache::Sqlite(c),
            Err(e) => {
                tracing::error!("Failed to create SQLite cache, falling back to memory: {e}");
                PmcCache::Memory(MemoryCache::new(config))
            }
        },
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_memory_cache_basic() {
        let config = CacheConfig {
            max_capacity: 10,
            time_to_live: Duration::from_secs(60),
            ..Default::default()
        };
        let cache = MemoryCache::<String, String>::new(&config);

        cache.insert("key1".to_string(), "value1".to_string()).await;
        assert_eq!(
            cache.get(&"key1".to_string()).await,
            Some("value1".to_string())
        );

        assert_eq!(cache.get(&"nonexistent".to_string()).await, None);

        cache.clear().await;
        assert_eq!(cache.get(&"key1".to_string()).await, None);
    }

    #[tokio::test]
    async fn test_cache_entry_count() {
        let config = CacheConfig::default();
        let cache = MemoryCache::<String, String>::new(&config);

        assert_eq!(cache.entry_count(), 0);

        cache.insert("key1".to_string(), "value1".to_string()).await;
        cache.sync().await;
        assert_eq!(cache.entry_count(), 1);

        cache.insert("key2".to_string(), "value2".to_string()).await;
        cache.sync().await;
        assert_eq!(cache.entry_count(), 2);

        cache.clear().await;
        cache.sync().await;
        assert_eq!(cache.entry_count(), 0);
    }
}
