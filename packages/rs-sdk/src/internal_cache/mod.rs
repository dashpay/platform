use crate::error::Error;
use crate::platform::transition::put_settings::PutSettings;
use crate::platform::Identifier;
use dpp::identity::identity_nonce::{IDENTITY_NONCE_VALUE_FILTER, MAX_MISSING_IDENTITY_REVISIONS};
use dpp::prelude::IdentityNonce;
use std::collections::BTreeMap;
use std::future::Future;
use tokio::sync::Mutex;

#[cfg(not(target_arch = "wasm32"))]
use std::time::{SystemTime, UNIX_EPOCH};

/// The default identity nonce stale time in seconds (20 minutes).
const DEFAULT_IDENTITY_NONCE_STALE_TIME_S: u64 = 1200;

/// Cached nonce state for a single identity or identity-contract pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct NonceCacheEntry {
    pub(crate) current_nonce: IdentityNonce,
    pub(crate) last_fetch_timestamp: u64,
    pub(crate) last_fetched_platform_nonce: IdentityNonce,
}

/// Compound key for identity-contract nonce lookups.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct IdentityContractPair {
    pub(crate) identity_id: Identifier,
    pub(crate) contract_id: Identifier,
}

impl From<(Identifier, Identifier)> for IdentityContractPair {
    fn from((identity_id, contract_id): (Identifier, Identifier)) -> Self {
        Self {
            identity_id,
            contract_id,
        }
    }
}

/// Nonce cache for identity and identity-contract nonces.
///
/// Encapsulates all nonce caching logic previously spread across
/// `Sdk`. Uses per-map locking so identity
/// and contract nonce queries don't block each other.
pub(crate) struct NonceCache {
    identity_nonces: Mutex<BTreeMap<Identifier, NonceCacheEntry>>,
    contract_nonces: Mutex<BTreeMap<IdentityContractPair, NonceCacheEntry>>,
    default_stale_time_s: u64,
}

impl Default for NonceCache {
    fn default() -> Self {
        Self {
            identity_nonces: Mutex::new(BTreeMap::new()),
            contract_nonces: Mutex::new(BTreeMap::new()),
            default_stale_time_s: DEFAULT_IDENTITY_NONCE_STALE_TIME_S,
        }
    }
}

/// Helper function to get current timestamp in seconds.
/// Works in both native and WASM environments.
fn get_current_time_seconds() -> u64 {
    #[cfg(not(target_arch = "wasm32"))]
    {
        match SystemTime::now().duration_since(UNIX_EPOCH) {
            Ok(n) => n.as_secs(),
            Err(_) => panic!("SystemTime before UNIX EPOCH!"),
        }
    }
    #[cfg(target_arch = "wasm32")]
    {
        // In WASM, we use JavaScript's Date.now() which returns milliseconds
        // We need to convert to seconds
        (js_sys::Date::now() / 1000.0) as u64
    }
}

impl NonceCache {
    /// Get or fetch identity nonce from cache.
    pub(crate) async fn get_identity_nonce<F, Fut>(
        &self,
        identity_id: Identifier,
        bump_first: bool,
        settings: &PutSettings,
        fetch_from_platform: F,
    ) -> Result<IdentityNonce, Error>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<u64, Error>>,
    {
        Self::get_or_fetch_nonce(
            &self.identity_nonces,
            identity_id,
            bump_first,
            settings,
            self.default_stale_time_s,
            fetch_from_platform,
        )
        .await
    }

    /// Get or fetch identity-contract nonce from cache.
    pub(crate) async fn get_identity_contract_nonce<F, Fut>(
        &self,
        identity_id: Identifier,
        contract_id: Identifier,
        bump_first: bool,
        settings: &PutSettings,
        fetch_from_platform: F,
    ) -> Result<IdentityNonce, Error>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<u64, Error>>,
    {
        Self::get_or_fetch_nonce(
            &self.contract_nonces,
            IdentityContractPair {
                identity_id,
                contract_id,
            },
            bump_first,
            settings,
            self.default_stale_time_s,
            fetch_from_platform,
        )
        .await
    }

    /// Marks all nonce cache entries for the given identity as stale.
    pub(crate) async fn refresh(&self, identity_id: &Identifier) {
        {
            let mut guard = self.identity_nonces.lock().await;
            if let Some(entry) = guard.get_mut(identity_id) {
                entry.last_fetch_timestamp = 0;
            }
        }
        {
            let mut guard = self.contract_nonces.lock().await;
            for (pair, entry) in guard.iter_mut() {
                if pair.identity_id == *identity_id {
                    entry.last_fetch_timestamp = 0;
                }
            }
        }
    }

    /// Shared nonce cache logic. Checks staleness and drift, fetches from
    /// Platform when needed, and maintains the cache entry.
    ///
    /// The cache lock is held only briefly to read/write the entry; it is
    /// **not** held across the async `fetch_from_platform` call so that other
    /// callers are not blocked during the network round-trip.
    async fn get_or_fetch_nonce<K: Ord + Copy, F, Fut>(
        cache: &Mutex<BTreeMap<K, NonceCacheEntry>>,
        key: K,
        bump_first: bool,
        settings: &PutSettings,
        default_stale_time_s: u64,
        fetch_from_platform: F,
    ) -> Result<IdentityNonce, Error>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<u64, Error>>,
    {
        let current_time_s = get_current_time_seconds();

        // Phase 1: Try to serve from cache without a network fetch.
        {
            let mut cache_guard = cache.lock().await;
            if let Some(entry) = cache_guard.get(&key).copied() {
                let stale_by_time = entry.last_fetch_timestamp
                    < current_time_s.saturating_sub(
                        settings
                            .identity_nonce_stale_time_s
                            .unwrap_or(default_stale_time_s),
                    );
                // Precautionary: use >= so we stay within Platform's own
                // MAX_MISSING_IDENTITY_REVISIONS limit.
                let drifted = entry
                    .current_nonce
                    .saturating_sub(entry.last_fetched_platform_nonce)
                    >= MAX_MISSING_IDENTITY_REVISIONS;

                if !stale_by_time && !drifted {
                    // Cache is fresh -- serve from it.
                    if bump_first {
                        let insert_nonce = entry.current_nonce + 1;
                        cache_guard.insert(
                            key,
                            NonceCacheEntry {
                                current_nonce: insert_nonce,
                                last_fetch_timestamp: current_time_s,
                                last_fetched_platform_nonce: entry.last_fetched_platform_nonce,
                            },
                        );
                        return Ok(insert_nonce);
                    } else {
                        return Ok(entry.current_nonce);
                    }
                }
            }
        } // lock released -- entry was absent or stale

        // Phase 2: Fetch from Platform without holding the cache lock.
        // Strip the upper "missing revisions" bits immediately so the
        // cache only ever holds plain nonce values.
        let platform_nonce = fetch_from_platform().await? & IDENTITY_NONCE_VALUE_FILTER;

        // Phase 3: Re-acquire lock and update cache.
        // Re-read the entry since another caller may have updated the cache
        // while we were fetching. Keep the higher of cached vs Platform
        // nonce to avoid regression (Platform may not have indexed a recent
        // successful broadcast yet).
        let mut cache_guard = cache.lock().await;
        let base_nonce = match cache_guard.get(&key) {
            Some(entry) if entry.current_nonce > platform_nonce => entry.current_nonce,
            _ => platform_nonce,
        };
        let insert_nonce = if bump_first {
            base_nonce + 1
        } else {
            base_nonce
        };
        cache_guard.insert(
            key,
            NonceCacheEntry {
                current_nonce: insert_nonce,
                last_fetch_timestamp: current_time_s,
                last_fetched_platform_nonce: platform_nonce,
            },
        );
        Ok(insert_nonce)
    }
}

#[cfg(test)]
mod nonce_cache_tests {
    use super::*;
    use crate::platform::transition::put_settings::PutSettings;
    use dpp::identity::identity_nonce::{
        IDENTITY_NONCE_VALUE_FILTER, MAX_MISSING_IDENTITY_REVISIONS,
    };
    use std::collections::BTreeMap;
    use std::sync::Arc;
    use test_case::test_case;
    use tokio::sync::Mutex;

    /// Helper: shorthand for get_or_fetch_nonce.
    async fn fetch(
        cache: &Mutex<BTreeMap<u32, NonceCacheEntry>>,
        bump: bool,
        settings: &PutSettings,
        platform_nonce: u64,
    ) -> u64 {
        NonceCache::get_or_fetch_nonce(
            cache,
            1u32,
            bump,
            settings,
            DEFAULT_IDENTITY_NONCE_STALE_TIME_S,
            || async move { Ok(platform_nonce) },
        )
        .await
        .unwrap()
    }

    fn now_s() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }

    fn empty_cache() -> Mutex<BTreeMap<u32, NonceCacheEntry>> {
        Mutex::new(BTreeMap::new())
    }

    fn seeded_cache(
        key: u32,
        current_nonce: u64,
        timestamp: u64,
        last_platform_nonce: u64,
    ) -> Mutex<BTreeMap<u32, NonceCacheEntry>> {
        let mut map = BTreeMap::new();
        map.insert(
            key,
            NonceCacheEntry {
                current_nonce,
                last_fetch_timestamp: timestamp,
                last_fetched_platform_nonce: last_platform_nonce,
            },
        );
        Mutex::new(map)
    }

    fn never_stale() -> PutSettings {
        PutSettings {
            identity_nonce_stale_time_s: Some(u64::MAX),
            ..Default::default()
        }
    }

    // --- Empty cache: platform fetched, result = platform +/- bump ---
    //                   (platform_nonce, bump,  expected_result, expected_stored_platform)
    #[test_case(42,  false, 42, 42  ; "basic no bump")]
    #[test_case(42,  true,  43, 42  ; "basic with bump")]
    #[test_case(0,   false, 0,  0   ; "zero no bump")]
    #[test_case(0,   true,  1,  0   ; "zero with bump")]
    #[test_case(IDENTITY_NONCE_VALUE_FILTER, false, IDENTITY_NONCE_VALUE_FILTER, IDENTITY_NONCE_VALUE_FILTER ; "filter max no bump")]
    #[test_case(IDENTITY_NONCE_VALUE_FILTER, true,  IDENTITY_NONCE_VALUE_FILTER + 1, IDENTITY_NONCE_VALUE_FILTER ; "filter max with bump overflows")]
    #[test_case(42 | (3 << 40), false, 42, 42 ; "upper bits stripped no bump")]
    #[test_case(42 | (3 << 40), true,  43, 42 ; "upper bits stripped with bump")]
    #[tokio::test]
    async fn empty_cache_fetch(
        platform_nonce: u64,
        bump: bool,
        expected: u64,
        expected_stored_platform: u64,
    ) {
        let cache = empty_cache();
        let result = fetch(&cache, bump, &Default::default(), platform_nonce).await;
        assert_eq!(result, expected);
        let guard = cache.lock().await;
        let entry = guard.get(&1u32).unwrap();
        assert_eq!(entry.current_nonce, expected);
        assert_eq!(entry.last_fetched_platform_nonce, expected_stored_platform);
    }

    // --- Fresh cache hit (no platform fetch) ---
    //                   (cached, last_platform, bump, expected)
    #[test_case(10, 10, false, 10 ; "no bump returns cached")]
    #[test_case(10, 10, true,  11 ; "bump increments cached")]
    #[test_case(10 + MAX_MISSING_IDENTITY_REVISIONS - 1, 10, false, 10 + MAX_MISSING_IDENTITY_REVISIONS - 1 ; "drift just below max serves from cache")]
    #[tokio::test]
    async fn fresh_cache_hit(cached_nonce: u64, last_platform: u64, bump: bool, expected: u64) {
        let cache = seeded_cache(1, cached_nonce, now_s(), last_platform);
        let settings = never_stale();
        let result = NonceCache::get_or_fetch_nonce(
            &cache,
            1u32,
            bump,
            &settings,
            DEFAULT_IDENTITY_NONCE_STALE_TIME_S,
            || async { panic!("should not fetch from platform") },
        )
        .await
        .unwrap();
        assert_eq!(result, expected);
    }

    // --- Stale or drifted cache triggers re-fetch ---
    //                   (cached, cached_plat, platform_returns, bump, stale_by_drift, expected, expected_stored_plat)
    #[test_case(10, 10, 15, false, false, 15, 15 ; "stale uses higher platform")]
    #[test_case(10, 10, 15, true,  false, 16, 15 ; "stale uses higher platform with bump")]
    #[test_case(20, 10, 15, false, false, 20, 15 ; "preserves higher cached nonce")]
    #[test_case(20, 10, 15, true,  false, 21, 15 ; "preserves higher cached nonce with bump")]
    #[test_case(10,  5, 50, false, false, 50, 50 ; "platform much higher replaces cache")]
    #[test_case(10,  5, 50, true,  false, 51, 50 ; "platform much higher replaces cache with bump")]
    #[test_case(100, 90, 50 | (5 << 40), false, false, 100, 50 ; "upper bits stripped cache preserved")]
    #[test_case(10 + MAX_MISSING_IDENTITY_REVISIONS, 10, 10 + MAX_MISSING_IDENTITY_REVISIONS, false, true, 10 + MAX_MISSING_IDENTITY_REVISIONS, 10 + MAX_MISSING_IDENTITY_REVISIONS ; "drift at max triggers refetch")]
    #[tokio::test]
    async fn cache_refetch(
        cached_nonce: u64,
        cached_platform: u64,
        platform_returns: u64,
        bump: bool,
        stale_by_drift: bool,
        expected: u64,
        expected_stored_platform: u64,
    ) {
        let (timestamp, settings) = if stale_by_drift {
            (now_s(), never_stale())
        } else {
            (0, PutSettings::default())
        };
        let cache = seeded_cache(1, cached_nonce, timestamp, cached_platform);
        let result = fetch(&cache, bump, &settings, platform_returns).await;
        assert_eq!(result, expected);
        let guard = cache.lock().await;
        let entry = guard.get(&1u32).unwrap();
        assert_eq!(entry.last_fetched_platform_nonce, expected_stored_platform);
    }

    // --- Multiple sequential bumps from cache ---
    #[tokio::test]
    async fn multiple_bumps_from_fresh_cache() {
        let cache = seeded_cache(1, 5, now_s(), 5);
        let settings = never_stale();
        for expected in 6..=10 {
            let result = NonceCache::get_or_fetch_nonce(
                &cache,
                1u32,
                true,
                &settings,
                DEFAULT_IDENTITY_NONCE_STALE_TIME_S,
                || async { panic!("should not fetch from platform") },
            )
            .await
            .unwrap();
            assert_eq!(result, expected);
        }
        let guard = cache.lock().await;
        let entry = guard.get(&1u32).unwrap();
        assert_eq!(entry.current_nonce, 10);
        assert_eq!(
            entry.last_fetched_platform_nonce, 5,
            "last platform nonce unchanged through bumps"
        );
    }

    // --- Fetch error propagates, cache untouched ---
    #[tokio::test]
    async fn fetch_error_propagates_and_cache_unchanged() {
        let cache = seeded_cache(1, 10, 0, 10);
        let result = NonceCache::get_or_fetch_nonce(
            &cache,
            1u32,
            false,
            &Default::default(),
            DEFAULT_IDENTITY_NONCE_STALE_TIME_S,
            || async { Err(crate::Error::Generic("platform unavailable".to_string())) },
        )
        .await;
        assert!(result.is_err());
        let guard = cache.lock().await;
        let entry = guard.get(&1u32).unwrap();
        assert_eq!(entry.current_nonce, 10);
        assert_eq!(
            entry.last_fetch_timestamp, 0,
            "timestamp should not have changed"
        );
        assert_eq!(entry.last_fetched_platform_nonce, 10);
    }

    // --- refresh marks entry stale ---
    #[tokio::test]
    async fn refresh_marks_entries_stale_forcing_refetch() {
        let cache = seeded_cache(1, 10, now_s(), 10);
        let settings = never_stale();

        // Confirm cache is served.
        let result = NonceCache::get_or_fetch_nonce(
            &cache,
            1u32,
            false,
            &settings,
            DEFAULT_IDENTITY_NONCE_STALE_TIME_S,
            || async { panic!("should not fetch") },
        )
        .await
        .unwrap();
        assert_eq!(result, 10);

        // Simulate refresh: set timestamp to 0.
        cache
            .lock()
            .await
            .get_mut(&1u32)
            .unwrap()
            .last_fetch_timestamp = 0;

        // With default settings (stale_time=1200s), timestamp=0 is stale.
        let result = fetch(&cache, false, &PutSettings::default(), 20).await;
        assert_eq!(result, 20, "should re-fetch after refresh");
    }

    // --- Different keys are isolated ---
    #[tokio::test]
    async fn different_keys_are_isolated() {
        let cache = empty_cache();
        let settings = never_stale();

        // Seed two keys via fetches.
        for (key, val) in [(1u32, 100u64), (2u32, 200u64)] {
            NonceCache::get_or_fetch_nonce(
                &cache,
                key,
                true,
                &Default::default(),
                DEFAULT_IDENTITY_NONCE_STALE_TIME_S,
                || async move { Ok(val) },
            )
            .await
            .unwrap();
        }

        // Read back: each key has its own value, served from cache.
        for (key, expected) in [(1u32, 101u64), (2u32, 201u64)] {
            let result = NonceCache::get_or_fetch_nonce(
                &cache,
                key,
                false,
                &settings,
                DEFAULT_IDENTITY_NONCE_STALE_TIME_S,
                || async { panic!("should serve from cache") },
            )
            .await
            .unwrap();
            assert_eq!(result, expected);
        }
    }

    // --- Concurrent cache update during fetch ---
    //                   (concurrent_nonce, platform_returns, bump, expected)
    #[test_case(15, 10, false, 15 ; "concurrent higher wins")]
    #[test_case(5,  20, true,  21 ; "platform higher wins with bump")]
    #[tokio::test]
    async fn concurrent_update(
        concurrent_nonce: u64,
        platform_returns: u64,
        bump: bool,
        expected: u64,
    ) {
        let cache = Arc::new(empty_cache());
        let cache_clone = Arc::clone(&cache);
        let result = NonceCache::get_or_fetch_nonce(
            &cache,
            1u32,
            bump,
            &Default::default(),
            DEFAULT_IDENTITY_NONCE_STALE_TIME_S,
            || async move {
                // Simulate another caller updating cache during our fetch.
                cache_clone.lock().await.insert(
                    1u32,
                    NonceCacheEntry {
                        current_nonce: concurrent_nonce,
                        last_fetch_timestamp: now_s(),
                        last_fetched_platform_nonce: concurrent_nonce,
                    },
                );
                Ok(platform_returns)
            },
        )
        .await
        .unwrap();
        assert_eq!(result, expected);
    }
}
