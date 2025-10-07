use quick_cache::{Weighter, sync::Cache};
use std::fmt::Debug;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use crate::DapiError;
use crate::metrics::{self};
use crate::services::streaming_service::SubscriptionHandle;
use crate::sync::Workers;

/// Estimated average size of a cache entry in bytes, used for initial capacity planning.
const ESTIMATED_ENTRY_SIZE_BYTES: u64 = 1024;
/// Fixed bincode configuration for stable serialization.
const BINCODE_CFG: bincode::config::Configuration = bincode::config::standard(); // keep this fixed for stability

#[derive(Clone)]
/// An LRU cache for storing serialized responses, keyed by method name and request parameters.
/// Uses a background worker to invalidate the cache on demand.
///
/// Entries are weighted by their estimated memory usage to better utilize the configured capacity.
///
/// The cache is thread-safe, cheaply cloneable, and can be shared across multiple threads.
///
/// # Panics
///
/// Panics if serialization of keys or values fails.
pub struct LruResponseCache {
    inner: Arc<Cache<CacheKey, CachedValue, CachedValueWeighter>>,
    label: Arc<str>,
    #[allow(dead_code)]
    workers: Workers,
}

impl Debug for LruResponseCache {
    /// Display cache size, total weight, and capacity for debugging output.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "LruResponseCache {{ size: {}, weight: {}, capacity: {} }}",
            self.inner.len(),
            self.inner.weight(),
            self.inner.capacity()
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct CacheKey {
    method: &'static str,
    digest: u128,
}

impl CacheKey {
    #[inline(always)]
    pub fn new<M: serde::Serialize>(method: &'static str, key: &M) -> CacheKey {
        make_cache_key(method, key)
    }

    #[inline(always)]
    pub const fn method(self) -> &'static str {
        self.method
    }

    #[inline(always)]
    pub const fn digest(self) -> u128 {
        self.digest
    }
}
#[derive(Clone)]
struct CachedValue {
    inserted_at: Instant,
    data: Vec<u8>,
}

impl CachedValue {
    #[inline(always)]
    /// Capture the current instant and serialize the provided value into bytes.
    ///
    /// Panics if serialization fails.
    fn new<T: serde::Serialize>(data: T) -> Self {
        let data = bincode::serde::encode_to_vec(&data, BINCODE_CFG)
            .expect("Failed to serialize cache value");

        Self {
            inserted_at: Instant::now(),
            data,
        }
    }

    #[inline(always)]
    /// Deserialize the cached bytes into the requested type if possible.
    fn value<T: serde::de::DeserializeOwned>(&self) -> Option<T> {
        if let Ok((v, _)) = bincode::serde::decode_from_slice(&self.data, BINCODE_CFG) {
            Some(v)
        } else {
            None
        }
    }
}

#[derive(Clone, Default)]
struct CachedValueWeighter;

impl Weighter<CacheKey, CachedValue> for CachedValueWeighter {
    /// Estimate cache entry weight by combining struct overhead and payload size.
    fn weight(&self, _key: &CacheKey, value: &CachedValue) -> u64 {
        let structural = std::mem::size_of::<CachedValue>() as u64;
        let payload = value.data.len() as u64;
        (structural + payload).max(1)
    }
}

impl LruResponseCache {
    /// Create a cache with a fixed capacity and without any external invalidation.
    /// Use this when caching immutable responses (e.g., blocks by hash).
    /// `capacity` is expressed in bytes.
    pub fn with_capacity(label: impl Into<Arc<str>>, capacity: u64) -> Self {
        let label = label.into();
        let cache = Self {
            inner: Self::new_cache(capacity),
            label: label.clone(),
            workers: Workers::new(),
        };
        observe_memory(&cache.inner, cache.label.as_ref());
        cache
    }
    /// Create a cache and start a background worker that clears the cache
    /// whenever a signal is received on the provided receiver.
    /// `capacity` is expressed in bytes.
    pub fn new(label: impl Into<Arc<str>>, capacity: u64, receiver: SubscriptionHandle) -> Self {
        let label = label.into();
        let inner = Self::new_cache(capacity);
        let inner_clone = inner.clone();
        let label_clone = label.clone();
        let workers = Workers::new();
        workers.spawn(async move {
            while receiver.recv().await.is_some() {
                inner_clone.clear();
                observe_memory(&inner_clone, label_clone.as_ref());
            }
            tracing::debug!("Cache invalidation task exiting");
            Result::<(), DapiError>::Ok(())
        });

        let cache = Self {
            inner,
            label,
            workers,
        };
        observe_memory(&cache.inner, cache.label.as_ref());
        cache
    }

    /// Create the underlying cache with weighted capacity based on estimated entry size.
    fn new_cache(capacity: u64) -> Arc<Cache<CacheKey, CachedValue, CachedValueWeighter>> {
        let capacity_bytes = capacity.max(1);
        let estimated_items_u64 = (capacity_bytes / ESTIMATED_ENTRY_SIZE_BYTES).max(1);
        let estimated_items = estimated_items_u64.min(usize::MAX as u64) as usize;
        Arc::new(Cache::with_weighter(
            estimated_items,
            capacity_bytes,
            CachedValueWeighter,
        ))
    }

    /// Remove all entries from the cache.
    pub fn clear(&self) {
        self.inner.clear();
        observe_memory(&self.inner, self.label.as_ref());
    }

    #[inline(always)]
    /// Retrieve a cached value by key, deserializing it into the requested type.
    pub fn get<T>(&self, key: &CacheKey) -> Option<T>
    where
        T: serde::Serialize + serde::de::DeserializeOwned,
    {
        match self.inner.get(key).and_then(|cv| cv.value()) {
            Some(cv) => {
                metrics::cache_hit(self.label.as_ref(), key.method());
                Some(cv)
            }
            None => {
                metrics::cache_miss(self.label.as_ref(), key.method());
                None
            }
        }
    }

    /// Get a value with TTL semantics; returns None if entry is older than TTL.
    pub fn get_with_ttl<T>(&self, key: &CacheKey, ttl: Duration) -> Option<T>
    where
        T: serde::Serialize + serde::de::DeserializeOwned,
    {
        if let Some(cv) = self.inner.get(key) {
            if cv.inserted_at.elapsed() <= ttl {
                metrics::cache_hit(self.label.as_ref(), key.method());
                return cv.value();
            }

            // expired, drop it
            self.inner.remove(key);
            observe_memory(&self.inner, self.label.as_ref());
        }

        metrics::cache_miss(self.label.as_ref(), key.method());
        None
    }

    /// Insert or replace a cached value for the given key.
    ///
    /// On error during serialization, the value is not cached.
    #[inline]
    pub fn put<T>(&self, key: CacheKey, value: &T)
    where
        T: serde::Serialize + serde::de::DeserializeOwned,
    {
        let cv = CachedValue::new(value);
        self.inner.insert(key, cv);
        observe_memory(&self.inner, self.label.as_ref());
    }

    /// Get a cached value or compute it using `producer` and insert into cache.
    /// The `producer` is executed only on cache miss.
    pub async fn get_or_try_insert<T, F, Fut, E>(&self, key: CacheKey, producer: F) -> Result<T, E>
    where
        T: serde::Serialize + serde::de::DeserializeOwned,
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<T, E>>,
    {
        use futures::future::FutureExt;

        let cache_hit = Arc::new(AtomicBool::new(true));
        let inner_hit = cache_hit.clone();

        let item = self
            .inner
            .get_or_insert_async(&key, async move {
                // wrapped in async block to not execute producer immediately
                // executed only on cache miss
                inner_hit.store(false, Ordering::SeqCst);

                producer()
                    .map(|result| result.map(|value| CachedValue::new(value)))
                    .await
            })
            .await
            .map(|cv| {
                observe_memory(&self.inner, self.label.as_ref());
                cv.value().expect("Deserialization must succeed")
            });

        if cache_hit.load(Ordering::SeqCst) {
            metrics::cache_hit(self.label.as_ref(), key.method());
        } else {
            metrics::cache_miss(self.label.as_ref(), key.method());
            observe_memory(&self.inner, self.label.as_ref());
        }

        item
    }
}

#[inline(always)]
fn observe_memory(cache: &Arc<Cache<CacheKey, CachedValue, CachedValueWeighter>>, label: &str) {
    metrics::cache_memory_usage_bytes(label, cache.weight());
    metrics::cache_memory_capacity_bytes(label, cache.capacity());
    metrics::cache_entries(label, cache.len());
}

#[inline(always)]
/// Combine a method name and serializable key into a stable 128-bit cache key.
///
/// Panics if serialization fails.
pub fn make_cache_key<M: serde::Serialize>(method: &'static str, key: &M) -> CacheKey {
    let mut data = Vec::with_capacity(ESTIMATED_ENTRY_SIZE_BYTES as usize); // preallocate some space
    bincode::serde::encode_into_std_write(key, &mut data, BINCODE_CFG)
        .expect("Failed to serialize cache key");
    data.push(0); // separator
    data.extend(method.as_bytes());

    CacheKey {
        method,
        digest: xxhash_rust::xxh3::xxh3_128(&data),
    }
}
