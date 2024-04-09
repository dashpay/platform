/// Cache errors
#[derive(Debug, thiserror::Error)]
pub enum CacheError {
    /// Overflow error
    #[error("global cache is blocked for block execution")]
    GlobalCacheIsBlocked,
}
