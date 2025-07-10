use moka::future::Cache as MokaCache;
use std::hash::Hash;
use std::time::Duration;
use tracing::{debug, info};

use crate::pmc::models::PmcFullText;

/// Configuration for memory cache
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// Maximum number of items to store in memory cache
    pub max_capacity: u64,
    /// Time-to-live for cached items
    pub time_to_live: Duration,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_capacity: 1000,
            time_to_live: Duration::from_secs(7 * 24 * 60 * 60), // 7 days
        }
    }
}

/// Memory-only cache implementation using Moka
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

/// Type alias for PMC cache
pub type PmcCache = MemoryCache<String, PmcFullText>;

/// Create a cache instance based on configuration
pub fn create_cache(config: &CacheConfig) -> PmcCache {
    MemoryCache::new(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_memory_cache_basic() {
        let config = CacheConfig {
            max_capacity: 10,
            time_to_live: Duration::from_secs(60),
        };
        let cache = MemoryCache::<String, String>::new(&config);

        // Test insert and get
        cache.insert("key1".to_string(), "value1".to_string()).await;
        assert_eq!(
            cache.get(&"key1".to_string()).await,
            Some("value1".to_string())
        );

        // Test cache miss
        assert_eq!(cache.get(&"nonexistent".to_string()).await, None);

        // Test clear
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
