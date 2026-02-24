use moka::future::Cache as MokaCache;
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, info};

use crate::pmc::models::PmcFullText;

// ---------------------------------------------------------------------------
// CacheBackend trait
// ---------------------------------------------------------------------------

/// Async storage backend for a cache.
///
/// Implement this trait to provide a custom caching layer.  The type
/// parameter `V` is the cached value type.
///
/// # Object safety
///
/// `CacheBackend<V>` is object-safe when used with [`async_trait`], so
/// [`TypedCache<V>`] can hold any backend behind `Arc<dyn CacheBackend<V>>`.
#[async_trait::async_trait]
pub trait CacheBackend<V>: Send + Sync
where
    V: Send + Sync,
{
    /// Return the cached value for `key`, or `None` on a miss.
    async fn get(&self, key: &str) -> Option<V>;

    /// Store `value` under `key`.
    async fn insert(&self, key: String, value: V);

    /// Remove all entries.
    async fn clear(&self);

    /// Return the number of live entries (best-effort; may return 0 for some
    /// backends).
    fn entry_count(&self) -> u64;

    /// Flush any pending internal tasks (useful for testing).
    async fn sync(&self);
}

// ---------------------------------------------------------------------------
// CacheBackendConfig / CacheConfig
// ---------------------------------------------------------------------------

/// Selects which storage backend to use for caching.
#[derive(Debug, Clone, Default)]
pub enum CacheBackendConfig {
    /// In-memory cache using Moka (default).
    #[default]
    Memory,
    /// Redis-backed persistent cache.
    ///
    /// Requires the `cache-redis` feature.
    #[cfg(feature = "cache-redis")]
    Redis {
        /// Redis connection URL, e.g. `"redis://127.0.0.1/"`.
        url: String,
    },
    /// SQLite-backed persistent cache.
    ///
    /// Requires the `cache-sqlite` feature.
    /// Not supported on WASM targets.
    #[cfg(feature = "cache-sqlite")]
    Sqlite {
        /// Path to the SQLite database file.
        path: std::path::PathBuf,
    },
}

/// Configuration for response caching.
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// Maximum number of items to store (used by the memory backend).
    pub max_capacity: u64,
    /// Time-to-live for cached items.
    pub time_to_live: Duration,
    /// Which storage backend to use.
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

/// In-memory cache backed by [Moka](https://docs.rs/moka).
#[derive(Clone)]
pub struct MemoryCache<V> {
    cache: MokaCache<String, V>,
}

impl<V: Clone + Send + Sync + 'static> MemoryCache<V> {
    /// Create a new in-memory cache from `config`.
    pub fn new(config: &CacheConfig) -> Self {
        let cache = MokaCache::builder()
            .max_capacity(config.max_capacity)
            .time_to_live(config.time_to_live)
            .build();
        Self { cache }
    }
}

#[async_trait::async_trait]
impl<V: Clone + Send + Sync + 'static> CacheBackend<V> for MemoryCache<V> {
    async fn get(&self, key: &str) -> Option<V> {
        let result = self.cache.get(key).await;
        if result.is_some() {
            debug!("Cache hit");
        } else {
            debug!("Cache miss");
        }
        result
    }

    async fn insert(&self, key: String, value: V) {
        self.cache.insert(key, value).await;
        info!("Item cached");
    }

    async fn clear(&self) {
        self.cache.invalidate_all();
        info!("Cache cleared");
    }

    fn entry_count(&self) -> u64 {
        self.cache.entry_count()
    }

    async fn sync(&self) {
        self.cache.run_pending_tasks().await;
    }
}

// ---------------------------------------------------------------------------
// Redis backend  (feature = "cache-redis")
// ---------------------------------------------------------------------------

/// Redis-backed cache using JSON serialisation.
///
/// Each cache operation opens a new multiplexed connection.  TTL is applied
/// per entry via `SET … EX`.
///
/// `entry_count()` always returns 0; Redis `DBSIZE` counts all keys and
/// cannot be scoped to this cache without a full scan.
///
/// Requires the `cache-redis` feature.
#[cfg(feature = "cache-redis")]
#[derive(Clone)]
pub struct RedisCache<V> {
    client: redis::Client,
    ttl: Duration,
    _phantom: std::marker::PhantomData<fn() -> V>,
}

#[cfg(feature = "cache-redis")]
impl<V> RedisCache<V> {
    /// Open a Redis connection pool at `url`.
    pub fn new(url: &str, ttl: Duration) -> Result<Self, redis::RedisError> {
        let client = redis::Client::open(url)?;
        Ok(Self {
            client,
            ttl,
            _phantom: std::marker::PhantomData,
        })
    }
}

#[cfg(feature = "cache-redis")]
#[async_trait::async_trait]
impl<V> CacheBackend<V> for RedisCache<V>
where
    V: serde::Serialize + serde::de::DeserializeOwned + Send + Sync + 'static,
{
    async fn get(&self, key: &str) -> Option<V> {
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

    async fn insert(&self, key: String, value: V) {
        use redis::AsyncCommands;
        let Ok(mut conn) = self.client.get_multiplexed_async_connection().await else {
            return;
        };
        let Ok(json) = serde_json::to_string(&value) else {
            return;
        };
        let _: Result<(), _> = conn.set_ex(key, json, self.ttl.as_secs()).await;
        info!("Item cached (Redis)");
    }

    async fn clear(&self) {
        let Ok(mut conn) = self.client.get_multiplexed_async_connection().await else {
            return;
        };
        let _: Result<(), _> = redis::cmd("FLUSHDB").query_async(&mut conn).await;
        info!("Cache cleared (Redis)");
    }

    fn entry_count(&self) -> u64 {
        0
    }

    async fn sync(&self) {
        // No-op for Redis
    }
}

// ---------------------------------------------------------------------------
// SQLite backend  (feature = "cache-sqlite")
// ---------------------------------------------------------------------------

/// SQLite-backed cache using JSON serialisation.
///
/// The database schema is created automatically on first use.  Expired entries
/// are not purged automatically; call [`CacheBackend::clear`] or run
/// `DELETE FROM cache WHERE expires_at <= unixepoch()` periodically.
///
/// `entry_count()` uses `try_lock`; returns 0 if the mutex is currently held.
///
/// Requires the `cache-sqlite` feature.  Not available on WASM targets.
#[cfg(feature = "cache-sqlite")]
#[derive(Clone)]
pub struct SqliteCache<V> {
    conn: Arc<std::sync::Mutex<rusqlite::Connection>>,
    ttl: Duration,
    _phantom: std::marker::PhantomData<fn() -> V>,
}

#[cfg(feature = "cache-sqlite")]
impl<V> SqliteCache<V> {
    /// Open (or create) a SQLite database at `path`.
    pub fn new(path: &std::path::Path, ttl: Duration) -> rusqlite::Result<Self> {
        let conn = rusqlite::Connection::open(path)?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS cache (
                key        TEXT    PRIMARY KEY,
                value      TEXT    NOT NULL,
                expires_at INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_cache_expires ON cache (expires_at);",
        )?;
        Ok(Self {
            conn: Arc::new(std::sync::Mutex::new(conn)),
            ttl,
            _phantom: std::marker::PhantomData,
        })
    }
}

#[cfg(feature = "cache-sqlite")]
fn sqlite_now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

#[cfg(feature = "cache-sqlite")]
#[async_trait::async_trait]
impl<V> CacheBackend<V> for SqliteCache<V>
where
    V: serde::Serialize + serde::de::DeserializeOwned + Send + Sync + 'static,
{
    async fn get(&self, key: &str) -> Option<V> {
        let key = key.to_owned();
        let conn = Arc::clone(&self.conn);
        tokio::task::spawn_blocking(move || {
            let now = sqlite_now_secs();
            let guard = conn.lock().unwrap();
            let result: rusqlite::Result<String> = guard.query_row(
                "SELECT value FROM cache WHERE key = ?1 AND expires_at > ?2",
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

    async fn insert(&self, key: String, value: V) {
        let conn = Arc::clone(&self.conn);
        let ttl = self.ttl;
        tokio::task::spawn_blocking(move || {
            let Ok(json) = serde_json::to_string(&value) else {
                return;
            };
            let expires_at = sqlite_now_secs() + ttl.as_secs() as i64;
            let guard = conn.lock().unwrap();
            let _ = guard.execute(
                "INSERT OR REPLACE INTO cache (key, value, expires_at) VALUES (?1, ?2, ?3)",
                rusqlite::params![key, json, expires_at],
            );
            info!("Item cached (SQLite)");
        })
        .await
        .ok();
    }

    async fn clear(&self) {
        let conn = Arc::clone(&self.conn);
        tokio::task::spawn_blocking(move || {
            let guard = conn.lock().unwrap();
            let _ = guard.execute("DELETE FROM cache", []);
            info!("Cache cleared (SQLite)");
        })
        .await
        .ok();
    }

    fn entry_count(&self) -> u64 {
        let now = sqlite_now_secs();
        if let Ok(guard) = self.conn.try_lock() {
            guard
                .query_row(
                    "SELECT COUNT(*) FROM cache WHERE expires_at > ?1",
                    rusqlite::params![now],
                    |row| row.get::<_, i64>(0),
                )
                .map(|c| c as u64)
                .unwrap_or(0)
        } else {
            0
        }
    }

    async fn sync(&self) {
        // No-op for SQLite
    }
}

// ---------------------------------------------------------------------------
// TypedCache — type-erased wrapper
// ---------------------------------------------------------------------------

/// A type-erased, cloneable cache for values of type `V`.
///
/// Wraps any [`CacheBackend<V>`] behind an `Arc<dyn …>`, so it can be
/// cloned cheaply and stored in structs without generics leaking into the
/// public API.
///
/// The concrete backend is selected via [`CacheConfig::backend`].
///
/// # Example
///
/// ```no_run
/// use pubmed_client::cache::{TypedCache, MemoryCache, CacheConfig};
///
/// # tokio_test::block_on(async {
/// let config = CacheConfig::default();
/// let cache: TypedCache<String> = TypedCache::new(MemoryCache::new(&config));
/// cache.insert("key".to_string(), "value".to_string()).await;
/// assert_eq!(cache.get("key").await, Some("value".to_string()));
/// # });
/// ```
#[derive(Clone)]
pub struct TypedCache<V: Send + Sync + 'static> {
    inner: Arc<dyn CacheBackend<V>>,
}

impl<V: Send + Sync + 'static> TypedCache<V> {
    /// Wrap `backend` in a [`TypedCache`].
    pub fn new(backend: impl CacheBackend<V> + 'static) -> Self {
        Self {
            inner: Arc::new(backend),
        }
    }

    pub async fn get(&self, key: &str) -> Option<V> {
        self.inner.get(key).await
    }

    pub async fn insert(&self, key: String, value: V) {
        self.inner.insert(key, value).await;
    }

    pub async fn clear(&self) {
        self.inner.clear().await;
    }

    pub fn entry_count(&self) -> u64 {
        self.inner.entry_count()
    }

    pub async fn sync(&self) {
        self.inner.sync().await;
    }
}

/// Type alias for the PMC full-text response cache.
pub type PmcCache = TypedCache<PmcFullText>;

// ---------------------------------------------------------------------------
// Factory
// ---------------------------------------------------------------------------

/// Create a [`PmcCache`] from `config`.
///
/// Falls back to the in-memory backend (with a logged error) when the
/// configured backend cannot be initialised.
pub fn create_cache(config: &CacheConfig) -> PmcCache {
    match &config.backend {
        CacheBackendConfig::Memory => TypedCache::new(MemoryCache::new(config)),
        #[cfg(feature = "cache-redis")]
        CacheBackendConfig::Redis { url } => match RedisCache::new(url, config.time_to_live) {
            Ok(c) => TypedCache::new(c),
            Err(e) => {
                tracing::error!("Failed to create Redis cache, falling back to memory: {e}");
                TypedCache::new(MemoryCache::new(config))
            }
        },
        #[cfg(feature = "cache-sqlite")]
        CacheBackendConfig::Sqlite { path } => match SqliteCache::new(path, config.time_to_live) {
            Ok(c) => TypedCache::new(c),
            Err(e) => {
                tracing::error!("Failed to create SQLite cache, falling back to memory: {e}");
                TypedCache::new(MemoryCache::new(config))
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
        let cache = TypedCache::new(MemoryCache::<String>::new(&config));

        cache.insert("key1".to_string(), "value1".to_string()).await;
        assert_eq!(cache.get("key1").await, Some("value1".to_string()));
        assert_eq!(cache.get("nonexistent").await, None);

        cache.clear().await;
        assert_eq!(cache.get("key1").await, None);
    }

    #[tokio::test]
    async fn test_cache_entry_count() {
        let config = CacheConfig::default();
        let cache = TypedCache::new(MemoryCache::<String>::new(&config));

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
