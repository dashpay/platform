use dapi_grpc::Message;
use lru::LruCache;
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

#[derive(Clone)]
struct CachedValue {
    inserted_at: Instant,
    bytes: Bytes,
}

impl LruResponseCache {
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
        T: Message + Default,
    {
        let mut lock = self.inner.lock().await;
        lock.get(key)
            .map(|cv| cv.bytes.clone())
            .and_then(|b| T::decode(b.as_ref()).ok())
    }

    /// Get a value with TTL semantics; returns None if entry is older than TTL.
    pub async fn get_with_ttl<T>(&self, key: &[u8; 32], ttl: Duration) -> Option<T>
    where
        T: Message + Default,
    {
        let mut lock = self.inner.lock().await;
        if let Some(cv) = lock.get(key).cloned() {
            if cv.inserted_at.elapsed() <= ttl {
                return T::decode(cv.bytes.as_ref()).ok();
            }
            // expired, drop it
            lock.pop(key);
        }
        None
    }

    pub async fn put<T>(&self, key: [u8; 32], value: &T)
    where
        T: Message,
    {
        let mut buf = Vec::with_capacity(value.encoded_len());
        if value.encode(&mut buf).is_ok() {
            let cv = CachedValue {
                inserted_at: Instant::now(),
                bytes: Bytes::from(buf),
            };
            self.inner.lock().await.put(key, cv);
        }
    }
}

#[inline(always)]
pub fn make_cache_key<M: Message>(method: &str, request: &M) -> [u8; 32] {
    use blake3::Hasher;
    let mut hasher = Hasher::new();
    hasher.update(method.as_bytes());
    hasher.update(&[0]);
    let serialized_request = request.encode_to_vec();
    hasher.update(&serialized_request);
    hasher.finalize().into()
}
