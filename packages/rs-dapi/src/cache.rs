use quick_cache::{Weighter, sync::Cache};
use std::fmt::Debug;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio_util::bytes::Bytes;

use crate::DapiError;
use crate::metrics;
use crate::services::streaming_service::SubscriptionHandle;
use crate::sync::Workers;

const ESTIMATED_ENTRY_SIZE_BYTES: u64 = 1024;

#[derive(Clone)]
pub struct LruResponseCache {
    inner: Arc<Cache<CacheKey, CachedValue, CachedValueWeighter>>,
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
    bytes: Bytes,
}

impl CachedValue {
    #[inline(always)]
    /// Capture the current instant and serialize the provided value into bytes.
    fn new<T: serde::Serialize>(data: T) -> Self {
        Self {
            inserted_at: Instant::now(),
            bytes: Bytes::from(serialize(&data).unwrap()),
        }
    }

    /// Deserialize the cached bytes into the requested type if possible.
    fn value<T: serde::de::DeserializeOwned>(&self) -> Option<T> {
        deserialize::<T>(&self.bytes)
    }
}

#[derive(Clone, Default)]
struct CachedValueWeighter;

impl Weighter<CacheKey, CachedValue> for CachedValueWeighter {
    /// Estimate cache entry weight by combining struct overhead and payload size.
    fn weight(&self, _key: &CacheKey, value: &CachedValue) -> u64 {
        let structural = std::mem::size_of::<CachedValue>() as u64;
        let payload = value.bytes.len() as u64;
        (structural + payload).max(1)
    }
}

impl LruResponseCache {
    /// Create a cache with a fixed capacity and without any external invalidation.
    /// Use this when caching immutable responses (e.g., blocks by hash).
    /// `capacity` is expressed in bytes.
    pub fn with_capacity(capacity: u64) -> Self {
        let cache = Self {
            inner: Self::new_cache(capacity),
            workers: Workers::new(),
        };
        observe_memory(&cache.inner);
        cache
    }
    /// Create a cache and start a background worker that clears the cache
    /// whenever a signal is received on the provided receiver.
    /// `capacity` is expressed in bytes.
    pub fn new(capacity: u64, receiver: SubscriptionHandle) -> Self {
        let inner = Self::new_cache(capacity);
        let inner_clone = inner.clone();
        let workers = Workers::new();
        workers.spawn(async move {
            while receiver.recv().await.is_some() {
                inner_clone.clear();
                metrics::cache_memory_usage_bytes(inner_clone.weight());
                metrics::cache_memory_capacity_bytes(inner_clone.capacity());
                metrics::cache_entries(inner_clone.len());
            }
            tracing::debug!("Cache invalidation task exiting");
            Result::<(), DapiError>::Ok(())
        });

        let cache = Self { inner, workers };
        observe_memory(&cache.inner);
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
        observe_memory(&self.inner);
    }

    #[inline(always)]
    /// Retrieve a cached value by key, deserializing it into the requested type.
    pub fn get<T>(&self, key: &CacheKey) -> Option<T>
    where
        T: serde::Serialize + serde::de::DeserializeOwned,
    {
        match self.inner.get(key) {
            Some(cv) => {
                metrics::cache_hit(key.method());
                cv.value()
            }
            None => {
                metrics::cache_miss(key.method());
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
                metrics::cache_hit(key.method());
                return cv.value();
            }
            // expired, drop it
            self.inner.remove(key);
            observe_memory(&self.inner);
            metrics::cache_miss(key.method());
            return None;
        }
        metrics::cache_miss(key.method());
        None
    }

    /// Insert or replace a cached value for the given key.
    pub fn put<T>(&self, key: CacheKey, value: &T)
    where
        T: serde::Serialize + serde::de::DeserializeOwned,
    {
        let cv = CachedValue::new(value);
        self.inner.insert(key, cv);
        observe_memory(&self.inner);
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

        if let Some(value) = self.get::<T>(&key) {
            return Ok(value);
        }

        self.inner
            .get_or_insert_async(&key, async move {
                // wrapped in async block to not execute producer immediately
                producer()
                    .map(|result| result.map(|value| CachedValue::new(value)))
                    .await
            })
            .await
            .map(|cv| {
                observe_memory(&self.inner);
                cv.value().expect("Deserialization must succeed")
            })
    }
}

#[inline(always)]
fn observe_memory(cache: &Arc<Cache<CacheKey, CachedValue, CachedValueWeighter>>) {
    metrics::cache_memory_usage_bytes(cache.weight());
    metrics::cache_memory_capacity_bytes(cache.capacity());
    metrics::cache_entries(cache.len());
}

#[inline(always)]
/// Combine a method name and serializable key into a stable 128-bit cache key.
pub fn make_cache_key<M: serde::Serialize>(method: &'static str, key: &M) -> CacheKey {
    let mut prefix = method.as_bytes().to_vec();
    let mut serialized_request = serialize(key).expect("Key must be serializable");

    let mut data = Vec::with_capacity(prefix.len() + 1 + serialized_request.len());
    data.append(&mut prefix);
    data.push(0);
    data.append(&mut serialized_request);

    CacheKey {
        method,
        digest: xxhash_rust::xxh3::xxh3_128(&data),
    }
}

const BINCODE_CFG: bincode::config::Configuration = bincode::config::standard(); // keep this fixed for stability

/// Serialize a value using bincode with a fixed configuration, logging failures.
fn serialize<T: serde::Serialize>(value: &T) -> Option<Vec<u8>> {
    bincode::serde::encode_to_vec(value, BINCODE_CFG)
        .inspect_err(|e| tracing::warn!("Failed to serialize cache value: {}", e))
        .ok() // deterministic
}

/// Deserialize bytes produced by `serialize`, returning the value when successful.
fn deserialize<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Option<T> {
    bincode::serde::decode_from_slice(bytes, BINCODE_CFG)
        .inspect_err(|e| tracing::warn!("Failed to deserialize cache value: {}", e))
        .ok()
        .map(|(v, _)| v) // deterministic
}
