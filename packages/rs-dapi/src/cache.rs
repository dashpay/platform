use quick_cache::{Weighter, sync::Cache};
use std::fmt::Debug;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use tracing::{debug, warn};

use crate::DapiError;
use crate::metrics::{self, MethodLabel};
use crate::services::streaming_service::SubscriptionHandle;
use crate::sync::Workers;

/// Estimated average size of a cache entry in bytes, used for initial capacity planning.
const ESTIMATED_ENTRY_SIZE_BYTES: u64 = 1024;

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
    inner: Arc<Cache<CacheIndex, CachedValue, CachedValueWeighter>>,
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
    /// Message digest; when None, all lookups will miss
    digest: Option<CacheIndex>,
}

type CacheIndex = u128;

impl CacheKey {
    #[inline(always)]
    pub fn new<M: serde::Serialize + Debug>(method: &'static str, key: &M) -> CacheKey {
        make_cache_key(method, key)
    }

    #[inline(always)]
    pub const fn method(self) -> &'static str {
        self.method
    }

    #[inline(always)]
    pub fn method_label(&self) -> MethodLabel {
        MethodLabel::from_type_name(self.method)
    }

    pub const fn digest(self) -> Option<CacheIndex> {
        self.digest
    }
}
#[derive(Clone)]
struct CachedValue {
    inserted_at: Instant,
    data: serde_bytes::ByteBuf,
}

impl Debug for CachedValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CachedValue")
            .field("inserted_at", &self.inserted_at)
            .field("data", &hex::encode(&self.data))
            .field("data_len", &self.data.len())
            .finish()
    }
}

impl CachedValue {
    /// Capture the current instant and serialize the provided value into bytes.
    ///
    /// Returns None if serialization fails.
    fn new<T: serde::Serialize>(data: T) -> Option<Self> {
        // We don't use bincode, as we have hit a bug in bincode
        // that causes deserialization to fail in some cases within get_with_ttl.
        let serialized = rmp_serde::to_vec(&data)
            .inspect_err(|e| {
                tracing::debug!("Failed to serialize value for caching: {}", e);
            })
            .ok()?;

        Some(Self {
            inserted_at: Instant::now(),
            data: serialized.into(),
        })
    }

    /// Deserialize the cached bytes into the requested type if possible.
    fn value<T: serde::de::DeserializeOwned>(&self) -> Result<T, DapiError> {
        rmp_serde::from_slice(&self.data).map_err(|e| {
            DapiError::invalid_data(format!("Failed to deserialize cached value: {}", e))
        })
    }
}

#[derive(Clone, Default)]
struct CachedValueWeighter;

impl Weighter<CacheIndex, CachedValue> for CachedValueWeighter {
    /// Estimate cache entry weight by combining struct overhead and payload size.
    fn weight(&self, _key: &CacheIndex, value: &CachedValue) -> u64 {
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
            while let Some(event) = receiver.recv().await {
                tracing::trace!(?event, "Cache invalidation event received, clearing cache");
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
    fn new_cache(capacity: u64) -> Arc<Cache<CacheIndex, CachedValue, CachedValueWeighter>> {
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

    /// Helper to get and parse the cached value
    fn get_and_parse<T: serde::de::DeserializeOwned + Debug>(
        &self,
        key: &CacheKey,
    ) -> Option<(T, Instant)> {
        let cached_value = self.inner.get(&key.digest()?)?;

        let value = match cached_value.value() {
            Ok(cv) => Some(cv),
            Err(error) => {
                debug!(%error, method = key.method(), "Failed to deserialize cached value, interpreting as cache miss and dropping");
                self.remove(key);

                None
            }
        };

        tracing::trace!(
            method = key.method(),
            age_ms = cached_value.inserted_at.elapsed().as_millis(),
            ?cached_value,
            ?value,
            "Cache hit"
        );

        value.map(|v| (v, cached_value.inserted_at))
    }

    /// Retrieve a cached value by key, deserializing it into the requested type.
    pub fn get<T>(&self, key: &CacheKey) -> Option<T>
    where
        T: serde::de::DeserializeOwned + Debug,
    {
        let method_label = key.method_label();
        match self.get_and_parse(key) {
            Some((v, _)) => {
                metrics::cache_hit(self.label.as_ref(), &method_label);
                Some(v)
            }
            None => {
                metrics::cache_miss(self.label.as_ref(), &method_label);
                None
            }
        }
    }

    /// Get a value with TTL semantics; returns None if entry is older than TTL.
    #[inline(always)]
    pub fn get_with_ttl<T>(&self, key: &CacheKey, ttl: Duration) -> Option<T>
    where
        T: serde::de::DeserializeOwned + Debug,
    {
        let Some((value, inserted_at)) = self.get_and_parse(key) else {
            metrics::cache_miss(self.label.as_ref(), &key.method_label());
            return None;
        };

        let method_label = key.method_label();

        if inserted_at.elapsed() <= ttl {
            metrics::cache_hit(self.label.as_ref(), &method_label);

            return value;
        }

        // expired, drop it
        self.remove(key);
        // treat as miss
        metrics::cache_miss(self.label.as_ref(), &method_label);
        None
    }

    /// Remove a cached value by key.
    /// Returns true if an entry was removed.
    pub fn remove(&self, key: &CacheKey) -> bool {
        let Some(index) = key.digest() else {
            return false;
        };

        let removed = self.inner.remove(&index).is_some();
        if removed {
            observe_memory(&self.inner, self.label.as_ref());
        }
        removed
    }

    /// Insert or replace a cached value for the given key.
    ///
    /// On error during serialization, the value is not cached.
    #[inline]
    pub fn put<T>(&self, key: CacheKey, value: &T)
    where
        T: serde::Serialize,
    {
        let Some(index) = key.digest() else {
            // serialization of key failed, skip caching
            debug!(
                method = key.method(),
                "Cache key serialization failed, skipping cache"
            );
            return;
        };

        if let Some(cv) = CachedValue::new(value) {
            self.inner.insert(index, cv);
            observe_memory(&self.inner, self.label.as_ref());
        }
    }

    /// Get a cached value or compute it using `producer` and insert into cache.
    /// The `producer` is executed only on cache miss.
    pub async fn get_or_try_insert<T, F, Fut, E>(&self, key: CacheKey, producer: F) -> Result<T, E>
    where
        T: serde::Serialize + serde::de::DeserializeOwned,
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<T, E>>,
        E: From<DapiError> + Debug,
    {
        let method_label = key.method_label();
        // calculate index; if serialization fails, always miss
        let Some(index) = key.digest() else {
            // serialization of key failed, always miss
            warn!(
                method = key.method(),
                "Cache key serialization failed, skipping cache"
            );
            metrics::cache_miss(self.label.as_ref(), &method_label);
            return producer().await;
        };

        let cache_hit = Arc::new(AtomicBool::new(true));
        let inner_hit = cache_hit.clone();

        let item = self
            .inner
            .get_or_insert_async(&index, async move {
                // wrapped in async block to not execute producer immediately
                // executed only on cache miss
                inner_hit.store(false, Ordering::SeqCst);

                match producer().await {
                    Ok(v) => CachedValue::new(v)
                        .ok_or_else(|| DapiError::invalid_data("Failed to serialize value").into()),
                    Err(e) => Err(e),
                }
            })
            .await
            .and_then(|cv| cv.value().map_err(Into::into));

        let hit = cache_hit.load(Ordering::SeqCst);
        match (hit, &item) {
            (true, Ok(_)) => {
                tracing::trace!(method = key.method(), "Cache hit");
                metrics::cache_hit(self.label.as_ref(), &method_label);
            }
            (true, Err(error)) => {
                tracing::debug!(
                    method = key.method(),
                    ?error,
                    "Cache hit but failed to deserialize cached value, dropping entry and recording as a miss"
                );
                metrics::cache_miss(self.label.as_ref(), &method_label);
                self.remove(&key);
            }
            (false, Ok(_)) => {
                tracing::trace!(
                    method = key.method(),
                    "Cache miss, value produced and cached"
                );
                metrics::cache_miss(self.label.as_ref(), &method_label);
                observe_memory(&self.inner, self.label.as_ref());
            }
            (false, Err(error)) => {
                tracing::debug!(
                    method = key.method(),
                    ?error,
                    "Cache miss, value production failed"
                );
                metrics::cache_miss(self.label.as_ref(), &method_label);
            }
        }

        item
    }
}

#[inline(always)]
fn observe_memory(cache: &Arc<Cache<CacheIndex, CachedValue, CachedValueWeighter>>, label: &str) {
    metrics::cache_memory_usage_bytes(label, cache.weight());
    metrics::cache_memory_capacity_bytes(label, cache.capacity());
    metrics::cache_entries(label, cache.len());
}

#[inline(always)]
/// Combine a method name and serializable key into a stable 128-bit cache key.
///
/// Sets digest to None if serialization fails, causing all lookups to miss.
pub fn make_cache_key<M: serde::Serialize + Debug>(method: &'static str, key: &M) -> CacheKey {
    let digest = match rmp_serde::to_vec(key) {
        Ok(mut data) => {
            data.push(0); // separator
            data.extend(method.as_bytes());
            Some(xxhash_rust::xxh3::xxh3_128(&data))
        }
        Err(error) => {
            debug!(?key, %error, "Failed to serialize cache key");
            None
        }
    };
    CacheKey { method, digest }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dapi_grpc::platform::v0::{
        GetStatusRequest, GetStatusResponse, get_status_request,
        get_status_response::{self, GetStatusResponseV0, get_status_response_v0::Time},
    };
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    #[tokio::test(flavor = "multi_thread")]
    /// Test that all cache methods work as expected.
    ///
    /// We have hit a bug in bincode that causes deserialization to fail when used through
    /// get_with_ttl. This test ensures it works correctly in that case.
    async fn all_cache_methods_must_work() {
        // Configure tracing for the test
        let _ = tracing_subscriber::fmt()
            .with_max_level(tracing::Level::TRACE)
            .with_test_writer()
            .try_init();

        // Given some cache, request, response and ttl
        let cache = LruResponseCache::with_capacity("platform", ESTIMATED_ENTRY_SIZE_BYTES * 4);
        let request = GetStatusRequest {
            version: Some(get_status_request::Version::V0(
                get_status_request::GetStatusRequestV0 {},
            )),
        };
        let key = make_cache_key("get_status", &request);

        let cached_time = Time {
            local: 42,
            block: Some(100),
            genesis: Some(200),
            epoch: Some(300),
        };

        let response = GetStatusResponse {
            version: Some(get_status_response::Version::V0(GetStatusResponseV0 {
                time: Some(cached_time),
                ..Default::default()
            })),
        };

        let ttl = Duration::from_secs(30);

        // When we put the response in the cache
        cache.put(key, &response);

        // Then all methods should return the cached response
        // 1. Directly inspect the raw cache entry

        let inner_cached_value = cache
            .inner
            .get(&key.digest().expect("digest present"))
            .expect("cache should contain raw entry");
        assert!(
            !inner_cached_value.data.is_empty(),
            "serialized cache entry should not be empty"
        );
        let decoded_from_raw = inner_cached_value
            .value::<GetStatusResponse>()
            .expect("raw decode should succeed");
        assert_eq!(
            decoded_from_raw, response,
            "raw cache entry should deserialize to stored response"
        );

        // 2. Use the typed get method
        let get_response = cache
            .get::<GetStatusResponse>(&key)
            .expect("expected plain get to succeed");

        assert_eq!(
            get_response, response,
            "plain cache get should match stored response"
        );

        // 3. Use internal get_and_parse method
        let (get_and_parse_response, _inserted_at) = cache
            .get_and_parse::<GetStatusResponse>(&key)
            .expect("expected get_and_parse to succeed");

        assert_eq!(
            get_and_parse_response, response,
            "get_and_parse value should match stored response"
        );

        // 4. Use the get_with_ttl method
        let get_with_ttl_response = cache
            .get_with_ttl::<GetStatusResponse>(&key, ttl)
            .expect("expected get_status response to be cached");

        // HERE IT FAILS WITH BINCODE!!!
        assert_eq!(
            get_with_ttl_response, response,
            "get_with_ttl cached response should match stored value"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn get_or_try_insert_caches_successful_values() {
        let _ = tracing_subscriber::fmt()
            .with_max_level(tracing::Level::TRACE)
            .with_test_writer()
            .try_init();

        let cache = LruResponseCache::with_capacity("test_cache", ESTIMATED_ENTRY_SIZE_BYTES * 2);
        let key = CacheKey::new("get_u64", &"key");
        let produced_value = 1337_u64;
        let producer_calls = Arc::new(AtomicUsize::new(0));

        let initial_calls = producer_calls.clone();
        let first = cache
            .get_or_try_insert::<_, _, _, DapiError>(key, || {
                let initial_calls = initial_calls.clone();
                async move {
                    initial_calls.fetch_add(1, Ordering::SeqCst);
                    Ok(produced_value)
                }
            })
            .await
            .expect("value should be produced on first call");

        assert_eq!(first, produced_value, "produced value must be returned");
        assert_eq!(
            producer_calls.load(Ordering::SeqCst),
            1,
            "producer should run exactly once on cache miss"
        );

        let cached = cache
            .get::<u64>(&key)
            .expect("value should be cached after first call");
        assert_eq!(cached, produced_value, "cached value must match producer");

        let follow_up_calls = producer_calls.clone();
        let second = cache
            .get_or_try_insert::<_, _, _, DapiError>(key, || {
                let follow_up_calls = follow_up_calls.clone();
                async move {
                    follow_up_calls.fetch_add(10, Ordering::SeqCst);
                    Ok(produced_value + 1)
                }
            })
            .await
            .expect("cached value should be returned on second call");

        assert_eq!(
            second, produced_value,
            "second call must yield cached value rather than producer result"
        );
        assert_eq!(
            producer_calls.load(Ordering::SeqCst),
            1,
            "producer should not run again when cache contains value"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn get_or_try_insert_does_not_cache_errors() {
        let _ = tracing_subscriber::fmt()
            .with_max_level(tracing::Level::TRACE)
            .with_test_writer()
            .try_init();

        let cache =
            LruResponseCache::with_capacity("test_cache_errors", ESTIMATED_ENTRY_SIZE_BYTES);
        let key = CacheKey::new("get_error", &"key");
        let producer_calls = Arc::new(AtomicUsize::new(0));

        let failing_calls = producer_calls.clone();
        let first_attempt: Result<u64, DapiError> = cache
            .get_or_try_insert::<u64, _, _, DapiError>(key, || {
                let failing_calls = failing_calls.clone();
                async move {
                    failing_calls.fetch_add(1, Ordering::SeqCst);
                    Err(DapiError::invalid_data("boom"))
                }
            })
            .await;

        assert!(
            first_attempt.is_err(),
            "failed producer result should be returned to caller"
        );
        assert_eq!(
            producer_calls.load(Ordering::SeqCst),
            1,
            "producer should run once even when it errors"
        );
        assert!(
            cache.get::<u64>(&key).is_none(),
            "failed producer must not populate the cache"
        );

        let successful_calls = producer_calls.clone();
        let expected_value = 9001_u64;
        let second_attempt = cache
            .get_or_try_insert::<u64, _, _, DapiError>(key, || {
                let successful_calls = successful_calls.clone();
                async move {
                    successful_calls.fetch_add(1, Ordering::SeqCst);
                    Ok(expected_value)
                }
            })
            .await
            .expect("second attempt should succeed and cache value");

        assert_eq!(
            second_attempt, expected_value,
            "successful producer result should be returned"
        );
        assert_eq!(
            producer_calls.load(Ordering::SeqCst),
            2,
            "producer should run again after an error because nothing was cached"
        );
        let cached = cache
            .get::<u64>(&key)
            .expect("successful producer must populate cache");
        assert_eq!(
            cached, expected_value,
            "cached value should match successful producer output"
        );
    }
}
