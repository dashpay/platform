use std::{
    fmt::Display,
    sync::{Arc, Mutex},
};

use lru::LruCache;

use crate::{
    request_settings::AppliedRequestSettings,
    transport::{CoreGrpcClient, PlatformGrpcClient},
    Uri,
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
    pub fn get_or_create<E>(
        &self,
        prefix: PoolPrefix,
        uri: &Uri,
        settings: Option<&AppliedRequestSettings>,
        create: impl FnOnce() -> Result<PoolItem, E>,
    ) -> Result<PoolItem, E> {
        if let Some(cli) = self.get(prefix, uri, settings) {
            return Ok(cli);
        }

        let cli = create();
        if let Ok(cli) = &cli {
            self.put(uri, settings, cli.clone());
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use dapi_grpc::tonic::transport::Channel;
    use std::str::FromStr;

    fn test_uri() -> Uri {
        Uri::from_str("http://127.0.0.1:3000").unwrap()
    }

    fn make_platform_pool_item() -> PoolItem {
        let channel = Channel::builder(test_uri()).connect_lazy();
        PoolItem::Platform(PlatformGrpcClient::new(channel))
    }

    fn make_core_pool_item() -> PoolItem {
        let channel = Channel::builder(test_uri()).connect_lazy();
        PoolItem::Core(CoreGrpcClient::new(channel))
    }

    #[test]
    fn test_connection_pool_new() {
        let pool = ConnectionPool::new(10);
        let result = pool.get(PoolPrefix::Platform, &test_uri(), None);
        assert!(result.is_none());
    }

    #[test]
    fn test_connection_pool_default() {
        let pool = ConnectionPool::default();
        let result = pool.get(PoolPrefix::Core, &test_uri(), None);
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_connection_pool_put_and_get_platform() {
        let pool = ConnectionPool::new(10);
        let uri = test_uri();
        let item = make_platform_pool_item();

        pool.put(&uri, None, item);

        let result = pool.get(PoolPrefix::Platform, &uri, None);
        assert!(result.is_some());
        assert!(matches!(result.unwrap(), PoolItem::Platform(_)));
    }

    #[tokio::test]
    async fn test_connection_pool_put_and_get_core() {
        let pool = ConnectionPool::new(10);
        let uri = test_uri();
        let item = make_core_pool_item();

        pool.put(&uri, None, item);

        let result = pool.get(PoolPrefix::Core, &uri, None);
        assert!(result.is_some());
        assert!(matches!(result.unwrap(), PoolItem::Core(_)));
    }

    #[tokio::test]
    async fn test_connection_pool_get_or_create_creates_new() {
        let pool = ConnectionPool::new(10);
        let uri = test_uri();

        let result: Result<PoolItem, String> =
            pool.get_or_create(PoolPrefix::Platform, &uri, None, || {
                Ok(make_platform_pool_item())
            });

        assert!(result.is_ok());

        // Second call should return cached version
        let mut create_called = false;
        let result2: Result<PoolItem, String> =
            pool.get_or_create(PoolPrefix::Platform, &uri, None, || {
                create_called = true;
                Ok(make_platform_pool_item())
            });

        assert!(result2.is_ok());
        assert!(
            !create_called,
            "create should not be called for cached item"
        );
    }

    #[test]
    fn test_connection_pool_get_or_create_error_not_cached() {
        let pool = ConnectionPool::new(10);
        let uri = test_uri();

        let result: Result<PoolItem, String> =
            pool.get_or_create(PoolPrefix::Platform, &uri, None, || {
                Err("creation failed".to_string())
            });

        assert!(result.is_err());

        // Pool should still be empty after failed creation
        let cached = pool.get(PoolPrefix::Platform, &uri, None);
        assert!(cached.is_none());
    }

    #[test]
    fn test_pool_prefix_display() {
        assert_eq!(format!("{}", PoolPrefix::Core), "Core");
        assert_eq!(format!("{}", PoolPrefix::Platform), "Platform");
    }

    #[tokio::test]
    async fn test_pool_prefix_from_pool_item() {
        let platform_item = make_platform_pool_item();
        let prefix: PoolPrefix = (&platform_item).into();
        assert!(matches!(prefix, PoolPrefix::Platform));

        let core_item = make_core_pool_item();
        let prefix: PoolPrefix = (&core_item).into();
        assert!(matches!(prefix, PoolPrefix::Core));
    }

    #[tokio::test]
    async fn test_pool_item_from_platform_client() {
        let channel = Channel::builder(test_uri()).connect_lazy();
        let client = PlatformGrpcClient::new(channel);
        let item: PoolItem = client.into();
        assert!(matches!(item, PoolItem::Platform(_)));
    }

    #[tokio::test]
    async fn test_pool_item_from_core_client() {
        let channel = Channel::builder(test_uri()).connect_lazy();
        let client = CoreGrpcClient::new(channel);
        let item: PoolItem = client.into();
        assert!(matches!(item, PoolItem::Core(_)));
    }

    #[tokio::test]
    async fn test_pool_item_into_platform_client() {
        let item = make_platform_pool_item();
        let _client: PlatformGrpcClient = item.into();
    }

    #[tokio::test]
    async fn test_pool_item_into_core_client() {
        let item = make_core_pool_item();
        let _client: CoreGrpcClient = item.into();
    }

    #[tokio::test]
    #[should_panic(expected = "ClientType is not Platform")]
    async fn test_pool_item_core_into_platform_panics() {
        let item = make_core_pool_item();
        let _client: PlatformGrpcClient = item.into();
    }

    #[tokio::test]
    #[should_panic(expected = "ClientType is not Core")]
    async fn test_pool_item_platform_into_core_panics() {
        let item = make_platform_pool_item();
        let _client: CoreGrpcClient = item.into();
    }

    #[tokio::test]
    async fn test_connection_pool_different_prefixes_different_keys() {
        let pool = ConnectionPool::new(10);
        let uri = test_uri();

        pool.put(&uri, None, make_platform_pool_item());

        // Core prefix should not find a Platform item
        let result = pool.get(PoolPrefix::Core, &uri, None);
        assert!(result.is_none());

        // Platform prefix should find it
        let result = pool.get(PoolPrefix::Platform, &uri, None);
        assert!(result.is_some());
    }

    #[tokio::test]
    async fn test_connection_pool_clone_shares_data() {
        let pool = ConnectionPool::new(10);
        let pool_clone = pool.clone();
        let uri = test_uri();

        pool.put(&uri, None, make_platform_pool_item());

        // Clone should see the same data
        let result = pool_clone.get(PoolPrefix::Platform, &uri, None);
        assert!(result.is_some());
    }
}
