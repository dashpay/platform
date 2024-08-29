use std::{
    fmt::Display,
    sync::{Arc, Mutex},
};

use dapi_grpc::tonic::transport::Uri;
use lru::LruCache;

use crate::{
    request_settings::AppliedRequestSettings,
    transport::{CoreGrpcClient, PlatformGrpcClient},
};

/// ConnectionPool represents pool of connections to DAPI nodes.
///
/// It can be cloned and shared between threads.
/// Cloning the pool will create a new reference to the same pool.
#[derive(Debug, Clone)]
pub struct ConnectionPool {
    inner: Arc<Mutex<LruCache<String, PoolItem>>>,
}

impl ConnectionPool {
    /// Create a new pool with a given capacity.
    /// The pool will evict the least recently used item when the capacity is reached.
    ///
    /// # Panics
    ///
    /// Panics if the capacity is zero.
    pub fn new(capacity: usize) -> Self {
        Self {
            inner: Arc::new(Mutex::new(LruCache::new(
                capacity.try_into().expect("must be non-zero"),
            ))),
        }
    }
}

impl Default for ConnectionPool {
    fn default() -> Self {
        Self::new(50)
    }
}

impl ConnectionPool {
    /// Get item from the pool for the given uri and settings.
    ///
    /// # Arguments
    /// * `prefix` -  Prefix for the item in the pool. Used to distinguish between Core and Platform clients.
    /// * `uri` - URI of the node.
    /// * `settings` - Applied request settings.
    pub fn get(
        &self,
        prefix: PoolPrefix,
        uri: &Uri,
        settings: Option<&AppliedRequestSettings>,
    ) -> Option<PoolItem> {
        let key = Self::key(prefix, uri, settings);
        self.inner.lock().expect("must lock").get(&key).cloned()
    }

    /// Get value from cache or create it using provided closure.
    /// If value is already in the cache, it will be returned.
    /// If value is not in the cache, it will be created by calling `create()` and stored in the cache.
    ///
    /// # Arguments
    /// * `prefix` -  Prefix for the item in the pool. Used to distinguish between Core and Platform clients.
    /// * `uri` - URI of the node.
    /// * `settings` - Applied request settings.
    pub fn get_or_create(
        &self,
        prefix: PoolPrefix,
        uri: &Uri,
        settings: Option<&AppliedRequestSettings>,
        create: impl FnOnce() -> PoolItem,
    ) -> PoolItem {
        if let Some(cli) = self.get(prefix, uri, settings) {
            return cli;
        }

        let cli = create();
        self.put(uri, settings, cli.clone());
        cli
    }

    /// Put item into the pool for the given uri and settings.
    pub fn put(&self, uri: &Uri, settings: Option<&AppliedRequestSettings>, value: PoolItem) {
        let key = Self::key(&value, uri, settings);
        self.inner.lock().expect("must lock").put(key, value);
    }

    fn key<C: Into<PoolPrefix>>(
        class: C,
        uri: &Uri,
        settings: Option<&AppliedRequestSettings>,
    ) -> String {
        let prefix: PoolPrefix = class.into();
        format!("{}:{}{:?}", prefix, uri, settings)
    }
}

/// Item stored in the pool.
///
/// We use an enum as we need to represent two different types of clients.
#[derive(Clone, Debug)]
pub enum PoolItem {
    Core(CoreGrpcClient),
    Platform(PlatformGrpcClient),
}

impl From<PlatformGrpcClient> for PoolItem {
    fn from(client: PlatformGrpcClient) -> Self {
        Self::Platform(client)
    }
}
impl From<CoreGrpcClient> for PoolItem {
    fn from(client: CoreGrpcClient) -> Self {
        Self::Core(client)
    }
}

impl From<PoolItem> for PlatformGrpcClient {
    fn from(client: PoolItem) -> Self {
        match client {
            PoolItem::Platform(client) => client,
            _ => {
                tracing::error!(
                    ?client,
                    "invalid connection fetched from pool: expected platform client"
                );
                panic!("ClientType is not Platform: {:?}", client)
            }
        }
    }
}

impl From<PoolItem> for CoreGrpcClient {
    fn from(client: PoolItem) -> Self {
        match client {
            PoolItem::Core(client) => client,
            _ => {
                tracing::error!(
                    ?client,
                    "invalid connection fetched from pool: expected core client"
                );
                panic!("ClientType is not Core: {:?}", client)
            }
        }
    }
}

/// Prefix for the item in the pool. Used to distinguish between Core and Platform clients.
pub enum PoolPrefix {
    Core,
    Platform,
}
impl Display for PoolPrefix {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PoolPrefix::Core => write!(f, "Core"),
            PoolPrefix::Platform => write!(f, "Platform"),
        }
    }
}
impl From<&PoolItem> for PoolPrefix {
    fn from(item: &PoolItem) -> Self {
        match item {
            PoolItem::Core(_) => PoolPrefix::Core,
            PoolItem::Platform(_) => PoolPrefix::Platform,
        }
    }
}
