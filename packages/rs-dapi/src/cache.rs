use dapi_grpc::Message;
use lru::LruCache;
use std::num::NonZeroUsize;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use tokio::task::JoinSet;
use tokio_util::bytes::Bytes;
#[derive(Clone)]
pub struct LruResponseCache {
    inner: Arc<RwLock<LruCache<[u8; 32], Bytes>>>,
    /// Background workers for cache management; will be aborted when last reference is dropped
    #[allow(dead_code)]
    workers: Arc<JoinSet<()>>,
}

impl LruResponseCache {
    /// Create a cache and start a background worker that clears the cache
    /// whenever a signal is received on the provided broadcast receiver.
    pub fn new(capacity: usize, mut rx: broadcast::Receiver<()>) -> Self {
        let cap = NonZeroUsize::new(capacity.max(1)).unwrap();
        let inner = Arc::new(RwLock::new(LruCache::new(cap)));
        let inner_clone = inner.clone();
        let mut workers = tokio::task::join_set::JoinSet::new();
        workers.spawn(async move {
            while rx.recv().await.is_ok() {
                inner_clone.write().await.clear();
            }
            tracing::debug!("Cache invalidation task exiting");
        });

        Self {
            inner,
            workers: Arc::new(workers),
        }
    }

    pub async fn clear(&self) {
        self.inner.write().await.clear();
    }

    #[inline(always)]
    pub async fn get<T>(&self, key: &[u8; 32]) -> Option<T>
    where
        T: Message + Default,
    {
        let mut lock = self.inner.write().await;
        if let Some(bytes) = lock.get(key).cloned() {
            T::decode(bytes.as_ref()).ok()
        } else {
            None
        }
    }

    pub async fn put<T>(&self, key: [u8; 32], value: &T)
    where
        T: Message,
    {
        let mut buf = Vec::with_capacity(value.encoded_len());
        if value.encode(&mut buf).is_ok() {
            self.inner.write().await.put(key, Bytes::from(buf));
        }
    }
}

#[inline(always)]
pub fn make_cache_key<M: Message>(method: &str, request: &M) -> [u8; 32] {
    use blake3::Hasher;
    let mut hasher = Hasher::new();
    hasher.update(method.as_bytes());
    hasher.update(&[0]);
    let mut buf = Vec::with_capacity(request.encoded_len());
    let _ = request.encode(&mut buf);
    hasher.update(&buf);
    hasher.finalize().into()
}
