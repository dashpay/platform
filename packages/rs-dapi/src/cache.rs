use dapi_grpc::Message;
use lru::LruCache;
use std::num::NonZeroUsize;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_util::bytes::Bytes;

#[derive(Clone)]
pub struct LruResponseCache {
    inner: Arc<RwLock<LruCache<[u8; 32], Bytes>>>,
}

impl LruResponseCache {
    pub fn new(capacity: usize) -> Self {
        let cap = NonZeroUsize::new(capacity.max(1)).unwrap();
        Self {
            inner: Arc::new(RwLock::new(LruCache::new(cap))),
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
