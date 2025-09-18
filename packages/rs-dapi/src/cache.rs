use lru::LruCache;
use std::fmt::Debug;
use std::num::NonZeroUsize;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tokio::task::JoinSet;
use tokio_util::bytes::Bytes;

use crate::services::streaming_service::SubscriptionHandle;

#[derive(Clone)]
pub struct LruResponseCache {
    inner: Arc<Mutex<LruCache<[u8; 32], CachedValue>>>,
    /// Background workers for cache management; will be aborted when last reference is dropped
    #[allow(dead_code)]
    workers: Arc<JoinSet<()>>,
}

impl Debug for LruResponseCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let lock = self.inner.try_lock();
        if let Ok(guard) = lock {
            write!(
                f,
                "LruResponseCache {{ size: {}, capacity: {} }}",
                guard.len(),
                guard.cap()
            )
        } else {
            write!(f, "LruResponseCache {{ <locked> }}")
        }
    }
}

#[derive(Clone)]
struct CachedValue {
    inserted_at: Instant,
    bytes: Bytes,
}

impl LruResponseCache {
    /// Create a cache with a fixed capacity and without any external invalidation.
    /// Use this when caching immutable responses (e.g., blocks by hash).
    pub fn with_capacity(capacity: usize) -> Self {
        let cap = NonZeroUsize::new(capacity.max(1)).unwrap();
        let inner = Arc::new(Mutex::new(LruCache::new(cap)));
        Self {
            inner,
            workers: Arc::new(tokio::task::join_set::JoinSet::new()),
        }
    }
    /// Create a cache and start a background worker that clears the cache
    /// whenever a signal is received on the provided receiver.
    pub fn new<T: Send + 'static>(capacity: usize, receiver: SubscriptionHandle<T>) -> Self {
        let cap = NonZeroUsize::new(capacity.max(1)).unwrap();
        let inner = Arc::new(Mutex::new(LruCache::new(cap)));
        let inner_clone = inner.clone();
        let mut workers = tokio::task::join_set::JoinSet::new();
        workers.spawn(async move {
            while receiver.recv().await.is_some() {
                inner_clone.lock().await.clear();
            }
            tracing::debug!("Cache invalidation task exiting");
        });

        Self {
            inner,
            workers: Arc::new(workers),
        }
    }

    pub async fn clear(&self) {
        self.inner.lock().await.clear();
    }

    #[inline(always)]
    pub async fn get<T>(&self, key: &[u8; 32]) -> Option<T>
    where
        T: serde::Serialize + serde::de::DeserializeOwned,
    {
        let mut lock = self.inner.lock().await;
        lock.get(key)
            .map(|cv| cv.bytes.clone())
            .and_then(|b| serde_json::from_slice::<T>(&b).ok())
    }

    /// Get a value with TTL semantics; returns None if entry is older than TTL.
    pub async fn get_with_ttl<T>(&self, key: &[u8; 32], ttl: Duration) -> Option<T>
    where
        T: serde::Serialize + serde::de::DeserializeOwned,
    {
        let mut lock = self.inner.lock().await;
        if let Some(cv) = lock.get(key).cloned() {
            if cv.inserted_at.elapsed() <= ttl {
                return serde_json::from_slice::<T>(&cv.bytes).ok();
            }
            // expired, drop it
            lock.pop(key);
        }
        None
    }

    pub async fn put<T>(&self, key: [u8; 32], value: &T)
    where
        T: serde::Serialize + serde::de::DeserializeOwned,
    {
        if let Ok(buf) = serde_json::to_vec(value) {
            let cv = CachedValue {
                inserted_at: Instant::now(),
                bytes: Bytes::from(buf),
            };
            self.inner.lock().await.put(key, cv);
        }
    }

    /// Get a cached value or compute it using `producer` and insert into cache.
    /// The `producer` is executed only on cache miss.
    pub async fn get_or_try_insert<T, F, Fut, E>(&self, key: [u8; 32], producer: F) -> Result<T, E>
    where
        T: serde::Serialize + serde::de::DeserializeOwned,
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<T, E>>,
    {
        if let Some(value) = self.get::<T>(&key).await {
            return Ok(value);
        }

        let value = producer().await?;
        self.put(key, &value).await;
        Ok(value)
    }
}

#[inline(always)]
pub fn make_cache_key<M: serde::Serialize + serde::de::DeserializeOwned>(
    method: &str,
    key: &M,
) -> [u8; 32] {
    use blake3::Hasher;
    let mut hasher = Hasher::new();
    hasher.update(method.as_bytes());
    hasher.update(&[0]);
    let serialized_request = serde_json::to_vec(key).expect("Key must be serializable");
    hasher.update(&serialized_request);
    hasher.finalize().into()
}

const BINCODE_CFG: bincode::config::Configuration = bincode::config::standard(); // keep this fixed for stability

fn serialize<T: serde::Serialize>(value: &T) -> Option<Vec<u8>> {
    bincode::serde::encode_to_vec(&value, BINCODE_CFG).ok() // deterministic
}

fn deserialize<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Option<T> {
    bincode::serde::decode_from_slice(bytes, BINCODE_CFG)
        .ok()
        .map(|(v, _)| v) // deterministic
}
