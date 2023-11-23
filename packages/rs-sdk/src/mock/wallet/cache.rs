//! Thread-safe cache of various objects inside the SDK.

use std::{hash::Hash, num::NonZeroUsize, sync::Arc};

/// Thread-safe cache of various objects inside the SDK.
///
/// This is used to cache objects that are expensive to fetch from the platform, like data contracts.
pub struct Cache<K: Hash + Eq, V> {
    // We use a Mutex to allow access to the cache when we don't have mutable &self
    // And we use Arc to allow multiple threads to access the cache without having to clone it
    inner: std::sync::RwLock<lru::LruCache<K, Arc<V>>>,
}

impl<K: Hash + Eq, V> Cache<K, V> {
    /// Create new cache
    pub fn new(capacity: NonZeroUsize) -> Self {
        Self {
            // inner: std::sync::Mutex::new(lru::LruCache::new(capacity)),
            inner: std::sync::RwLock::new(lru::LruCache::new(capacity)),
        }
    }

    /// Get a reference to the value stored under `k`.
    pub fn get(&self, k: &K) -> Option<Arc<V>> {
        let mut guard = self.inner.write().expect("cache lock poisoned");
        guard.get(k).map(Arc::clone)
    }

    /// Insert a new value into the cache.
    pub fn put(&self, k: K, v: V) {
        let mut guard = self.inner.write().expect("cache lock poisoned");
        guard.put(k, Arc::new(v));
    }
}
