use crate::error::Error;
use crate::platform::transition::put_settings::PutSettings;
use crate::platform::{Fetch, Identifier};
use crate::Sdk;
use dpp::identity::identity_nonce::{IDENTITY_NONCE_VALUE_FILTER, MAX_MISSING_IDENTITY_REVISIONS};
use dpp::prelude::IdentityNonce;
use drive_proof_verifier::types::{IdentityContractNonceFetcher, IdentityNonceFetcher};
use lru::LruCache;
use std::fmt::Debug;
use std::future::Future;
use std::hash::Hash;
use std::num::NonZeroUsize;
use tokio::sync::Mutex;

#[cfg(not(target_arch = "wasm32"))]
use std::time::{SystemTime, UNIX_EPOCH};

/// The default identity nonce stale time in seconds (20 minutes).
const DEFAULT_IDENTITY_NONCE_STALE_TIME_S: u64 = 1200;

/// Maximum number of entries in each nonce LRU cache.
const DEFAULT_NONCE_CACHE_SIZE: NonZeroUsize =
    NonZeroUsize::new(1000).expect("DEFAULT_NONCE_CACHE_SIZE must be > 0");

/// Sentinel timestamp used by [`NonceCache::refresh`] to mark cache entries
/// as stale without removing them.  Any real `SystemTime` timestamp will be
/// greater than this value, so the staleness check in [`NonceCache::get_or_fetch_nonce`]
/// will always trigger a re-fetch for entries with this timestamp.
const STALE_TIMESTAMP: u64 = 0;

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

/// Nonce cache for identity and identity-contract nonces.
///
/// Encapsulates all nonce caching logic previously spread across
/// `Sdk`. Uses per-map locking so identity
/// and contract nonce queries don't block each other.
///
/// Backed by [`LruCache`] to bound memory usage and automatically
/// evict least-recently-used entries.
///
/// # LRU eviction
///
/// Each map is capped at [`DEFAULT_NONCE_CACHE_SIZE`] (1000) entries.
/// Evicting an entry with unconfirmed local bumps may cause a nonce
/// collision on re-fetch.  Unlikely in practice (requires >1000
/// concurrent identities); the rejection is transient and resolves
/// on retry once the pending transaction confirms.
///
/// # Drift threshold
///
/// Up to 23 nonces can be bumped from cache before a Platform re-fetch
/// is forced (drift limit [`MAX_MISSING_IDENTITY_REVISIONS`] = 24).
/// The 20-minute stale timer provides an additional backstop.  If
/// transactions fail silently, the cache may advance past the on-chain
/// nonce until the next re-fetch realigns it.
///
/// # DAPI node staleness
///
/// A stale DAPI node may report an identity as unknown.  The fetch
/// closure surfaces this as [`Error::IdentityNonceNotFound`] rather
/// than defaulting to nonce 0.  Worst case: one state transition
/// attempt fails and must be retried against a different node.
pub(crate) struct NonceCache {
    identity_nonces: Mutex<LruCache<Identifier, NonceCacheEntry>>,
    contract_nonces: Mutex<LruCache<IdentityContractPair, NonceCacheEntry>>,
    default_stale_time_s: u64,
}

impl Debug for NonceCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NonceCache")
            .field("default_stale_time_s", &self.default_stale_time_s)
            .finish_non_exhaustive()
    }
}

impl Default for NonceCache {
    fn default() -> Self {
        Self {
            identity_nonces: Mutex::new(LruCache::new(DEFAULT_NONCE_CACHE_SIZE)),
            contract_nonces: Mutex::new(LruCache::new(DEFAULT_NONCE_CACHE_SIZE)),
            default_stale_time_s: DEFAULT_IDENTITY_NONCE_STALE_TIME_S,
        }
    }
}

/// Helper function to get current timestamp in seconds.
/// Works in both native and WASM environments.
fn get_current_time_seconds() -> Result<u64, Error> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .map_err(|e| Error::Generic(format!("SystemTime before UNIX EPOCH: {e}")))
    }
    #[cfg(target_arch = "wasm32")]
    {
        // In WASM, we use JavaScript's Date.now() which returns milliseconds
        // We need to convert to seconds
        Ok((js_sys::Date::now() / 1000.0) as u64)
    }
}

/// Increment `nonce` by one, returning an error if it is already at the
/// 40-bit maximum ([`IDENTITY_NONCE_VALUE_FILTER`]).
fn bump_nonce(nonce: u64) -> Result<u64, Error> {
    if nonce >= IDENTITY_NONCE_VALUE_FILTER {
        return Err(Error::NonceOverflow(nonce));
    }
    Ok(nonce + 1)
}

impl NonceCache {
    /// Get or fetch identity nonce from cache, querying Platform via the SDK
    /// when the cached value is stale or absent.
    pub(crate) async fn get_identity_nonce(
        &self,
        sdk: &Sdk,
        identity_id: Identifier,
        bump_first: bool,
        settings: &PutSettings,
    ) -> Result<IdentityNonce, Error> {
        let request_settings = settings.request_settings;
        Self::get_or_fetch_nonce(
            &self.identity_nonces,
            identity_id,
            bump_first,
            settings,
            self.default_stale_time_s,
            || async move {
                let fetcher =
                    IdentityNonceFetcher::fetch_with_settings(sdk, identity_id, request_settings)
                        .await?
                        .ok_or_else(|| {
                            tracing::warn!(
                                identity_id = %identity_id,
                                "Platform returned no nonce for identity; \
                                 node may be stale or identity may not exist yet"
                            );
                            Error::IdentityNonceNotFound(format!(
                                "identity {identity_id}: platform returned no nonce"
                            ))
                        })?;
                Ok(fetcher.0)
            },
        )
        .await
    }

    /// Get or fetch identity-contract nonce from cache, querying Platform via
    /// the SDK when the cached value is stale or absent.
    pub(crate) async fn get_identity_contract_nonce(
        &self,
        sdk: &Sdk,
        identity_id: Identifier,
        contract_id: Identifier,
        bump_first: bool,
        settings: &PutSettings,
    ) -> Result<IdentityNonce, Error> {
        let request_settings = settings.request_settings;
        Self::get_or_fetch_nonce(
            &self.contract_nonces,
            IdentityContractPair {
                identity_id,
                contract_id,
            },
            bump_first,
            settings,
            self.default_stale_time_s,
            || async move {
                let fetcher = IdentityContractNonceFetcher::fetch_with_settings(
                    sdk,
                    (identity_id, contract_id),
                    request_settings,
                )
                .await?
                .ok_or_else(|| {
                    tracing::warn!(
                        identity_id = %identity_id,
                        contract_id = %contract_id,
                        "Platform returned no nonce for identity-contract pair; \
                         node may be stale or identity may not exist yet"
                    );
                    Error::IdentityNonceNotFound(format!(
                        "identity {identity_id} contract {contract_id}: \
                         platform returned no nonce"
                    ))
                })?;
                Ok(fetcher.0)
            },
        )
        .await
    }

    /// Marks all nonce cache entries for the given identity as stale,
    /// forcing a fresh Platform fetch on the next access while
    /// **preserving** the cached nonce value.
    ///
    /// This is critical for correctness: after a broadcast failure the
    /// cached nonce (already bumped) must be preserved so that the
    /// Phase 3 `max(cached, platform)` merge in [`get_or_fetch_nonce`]
    /// can advance past the nonce that was already used, preventing
    /// "tx already exists in cache" errors on retry.
    ///
    /// # LRU promotion
    ///
    /// We intentionally use `get_mut()` (which promotes entries to
    /// most-recently-used) rather than `peek_mut()`.  A refreshed entry
    /// is one whose nonce is actively in use — it was just bumped for a
    /// broadcast attempt — so it should be preserved over idle entries
    /// under LRU eviction pressure.
    pub(crate) async fn refresh(&self, identity_id: &Identifier) {
        {
            let mut guard = self.identity_nonces.lock().await;
            if let Some(entry) = guard.get_mut(identity_id) {
                entry.last_fetch_timestamp = STALE_TIMESTAMP;
            }
        }
        {
            let mut guard = self.contract_nonces.lock().await;
            // LruCache doesn't support iter_mut, so collect keys first
            // then mark each entry stale via get_mut.
            let keys: Vec<IdentityContractPair> = guard
                .iter()
                .filter(|(pair, _)| pair.identity_id == *identity_id)
                .map(|(pair, _)| *pair)
                .collect();
            for key in keys {
                if let Some(entry) = guard.get_mut(&key) {
                    entry.last_fetch_timestamp = STALE_TIMESTAMP;
                }
            }
        }
    }

    /// Shared nonce cache logic. Checks staleness and drift, fetches from
    /// Platform when needed, and maintains the cache entry.
    ///
    /// Uses a three-phase approach:
    ///   1. Check cache under lock — return immediately if fresh.
    ///   2. Fetch from Platform **without** holding the lock.
    ///   3. Re-acquire lock and merge using `max(cached, platform)`.
    ///
    /// This accepts a narrow TOCTOU race where two concurrent callers for the
    /// same key may both fetch from Platform, but the `max()` merge ensures no
    /// nonce regression.
    ///
    /// # Errors
    ///
    /// - Returns whatever error `fetch_from_platform` produces, including
    ///   [`Error::IdentityNonceNotFound`] when the queried DAPI node does
    ///   not know the identity. Callers should retry in that case; a
    ///   different node will likely have the data.
    /// - Returns [`Error::NonceOverflow`] if bumping would wrap past the
    ///   40-bit nonce value filter.
    async fn get_or_fetch_nonce<K: Hash + Eq + Copy + Debug, F, Fut>(
        cache: &Mutex<LruCache<K, NonceCacheEntry>>,
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
        let current_time_s = get_current_time_seconds()?;

        // Phase 1: Check cache under lock
        {
            let mut cache_guard = cache.lock().await;

            if let Some(entry) = cache_guard.get_mut(&key) {
                let stale_by_time = entry.last_fetch_timestamp
                    < current_time_s.saturating_sub(
                        settings
                            .identity_nonce_stale_time_s
                            .unwrap_or(default_stale_time_s),
                    );
                let drifted = entry
                    .current_nonce
                    .saturating_sub(entry.last_fetched_platform_nonce)
                    >= MAX_MISSING_IDENTITY_REVISIONS;

                if !stale_by_time && !drifted {
                    // Fresh hit — serve from cache, mutate in place.
                    if bump_first {
                        let insert_nonce = bump_nonce(entry.current_nonce)?;
                        entry.current_nonce = insert_nonce;
                        // Do NOT update last_fetch_timestamp on cache-only bumps
                        return Ok(insert_nonce);
                    } else {
                        return Ok(entry.current_nonce);
                    }
                }

                if stale_by_time {
                    tracing::trace!(key = ?key, "nonce cache stale, re-fetching from platform");
                } else {
                    tracing::trace!(key = ?key, "nonce cache drifted, re-fetching from platform");
                }
            } else {
                tracing::trace!(key = ?key, "nonce cache miss, fetching from platform");
            }
        } // lock released

        // Phase 2: Fetch from Platform without holding the lock
        //
        // Strip the upper "missing revisions" bits immediately so the
        // cache only ever holds plain nonce values.
        let platform_nonce = fetch_from_platform().await? & IDENTITY_NONCE_VALUE_FILTER;

        // Phase 3: Re-acquire lock, use max(cached, platform)
        //
        // Capture a fresh timestamp so last_fetch_timestamp reflects when
        // the data was actually received, not when the function was entered.
        let current_time_s = get_current_time_seconds()?;
        let mut cache_guard = cache.lock().await;

        // Keep the higher of cached vs Platform nonce to avoid regression
        // (Platform may not have indexed a recent successful broadcast yet).
        // Also preserve the higher last_fetched_platform_nonce so the drift
        // calculation doesn't inflate when Platform temporarily returns a
        // lower value than previously seen.
        let (base_nonce, effective_platform_nonce) = match cache_guard.peek(&key) {
            Some(entry) if entry.current_nonce > platform_nonce => {
                tracing::trace!(key = ?key, "nonce cache: preserved higher cached nonce over platform");
                (
                    entry.current_nonce,
                    entry.last_fetched_platform_nonce.max(platform_nonce),
                )
            }
            _ => (platform_nonce, platform_nonce),
        };
        let insert_nonce = if bump_first {
            bump_nonce(base_nonce)?
        } else {
            base_nonce
        };
        cache_guard.put(
            key,
            NonceCacheEntry {
                current_nonce: insert_nonce,
                last_fetch_timestamp: current_time_s,
                last_fetched_platform_nonce: effective_platform_nonce,
            },
        );
        Ok(insert_nonce)
    }
}

#[cfg(test)]
mod nonce_cache_tests {
    use super::*;
    use crate::platform::transition::put_settings::PutSettings;
    use test_case::test_case;

    /// Helper: shorthand for get_or_fetch_nonce that expects success.
    async fn fetch(
        cache: &Mutex<LruCache<u32, NonceCacheEntry>>,
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

    /// Helper: shorthand for get_or_fetch_nonce that returns the Result.
    async fn try_fetch(
        cache: &Mutex<LruCache<u32, NonceCacheEntry>>,
        bump: bool,
        settings: &PutSettings,
        platform_nonce: u64,
    ) -> Result<u64, Error> {
        NonceCache::get_or_fetch_nonce(
            cache,
            1u32,
            bump,
            settings,
            DEFAULT_IDENTITY_NONCE_STALE_TIME_S,
            || async move { Ok(platform_nonce) },
        )
        .await
    }

    fn now_s() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }

    fn lru_cap() -> NonZeroUsize {
        NonZeroUsize::new(16).unwrap()
    }

    fn empty_cache() -> Mutex<LruCache<u32, NonceCacheEntry>> {
        Mutex::new(LruCache::new(lru_cap()))
    }

    fn seeded_cache(
        key: u32,
        current_nonce: u64,
        timestamp: u64,
        last_platform_nonce: u64,
    ) -> Mutex<LruCache<u32, NonceCacheEntry>> {
        let mut map = LruCache::new(lru_cap());
        map.put(
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
        let mut guard = cache.lock().await;
        let entry = guard.get(&1u32).unwrap();
        assert_eq!(entry.current_nonce, expected);
        assert_eq!(entry.last_fetched_platform_nonce, expected_stored_platform);
    }

    // --- SEC-001: Bumping at filter max returns overflow error ---
    #[tokio::test]
    async fn filter_max_with_bump_returns_overflow_error() {
        let cache = empty_cache();
        let result = try_fetch(
            &cache,
            true,
            &Default::default(),
            IDENTITY_NONCE_VALUE_FILTER,
        )
        .await;
        assert!(
            matches!(result, Err(Error::NonceOverflow(_))),
            "expected NonceOverflow error, got: {result:?}"
        );
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
    #[test_case(100, 90, 50 | (5 << 40), false, false, 100, 90 ; "upper bits stripped cache preserved")]
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
        let mut guard = cache.lock().await;
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
        let mut guard = cache.lock().await;
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
        let entry = guard.peek(&1u32).unwrap();
        assert_eq!(entry.current_nonce, 10);
        assert_eq!(
            entry.last_fetch_timestamp, 0,
            "timestamp should not have changed"
        );
        assert_eq!(entry.last_fetched_platform_nonce, 10);
    }

    // --- refresh marks entries stale, preserving cached nonce value ---
    #[tokio::test]
    async fn refresh_marks_stale_but_preserves_cached_nonce() {
        use drive_proof_verifier::types::IdentityNonceFetcher;

        let mut sdk = crate::Sdk::new_mock();
        let identity_id = Identifier::default();
        // Use default stale time so that STALE_TIMESTAMP (0) is detected
        // as stale, exercising the full Phase 2 + Phase 3 re-fetch path.
        let settings = PutSettings::default();

        // Mock: platform always returns nonce 10.
        sdk.mock()
            .expect_fetch::<IdentityNonceFetcher, _>(identity_id, Some(IdentityNonceFetcher(10u64)))
            .await
            .expect("set mock expectation");

        // Seed via initial fetch (platform returns 10, bump to 11).
        let nonce = sdk
            .get_identity_nonce(identity_id, true, Some(settings))
            .await
            .unwrap();
        assert_eq!(nonce, 11);

        // Bump again from cache (11 → 12).
        let nonce = sdk
            .get_identity_nonce(identity_id, true, Some(settings))
            .await
            .unwrap();
        assert_eq!(nonce, 12);

        // Simulate broadcast failure: refresh marks the entry stale
        // but preserves the cached nonce value (12).
        sdk.refresh_identity_nonce(&identity_id).await;

        // Next call re-fetches from Platform (returns 10, the old value
        // because the tx hasn't confirmed yet).  Phase 3 should see
        // max(cached=12, platform=10) = 12, then bump to 13.
        // This is the fix for dashpay/dash-evo-tool#588: without
        // preserving the cached nonce, we'd get 11 (same as the first
        // broadcast), causing "tx already exists in cache".
        let nonce = sdk
            .get_identity_nonce(identity_id, true, Some(settings))
            .await
            .unwrap();
        assert_eq!(nonce, 13);
    }

    /// Refresh followed by a Platform fetch returning a HIGHER nonce
    /// (the pending tx was confirmed) should use the Platform value.
    #[tokio::test]
    async fn refresh_uses_platform_nonce_when_higher() {
        use drive_proof_verifier::types::IdentityNonceFetcher;

        let mut sdk = crate::Sdk::new_mock();
        let identity_id = Identifier::default();
        // Use default stale time so that timestamp=0 is detected as stale.
        let settings = PutSettings::default();

        // First expectation: platform returns 10.
        sdk.mock()
            .expect_fetch::<IdentityNonceFetcher, _>(identity_id, Some(IdentityNonceFetcher(10u64)))
            .await
            .expect("set mock expectation");

        // Seed: platform=10, bump to 11.
        let nonce = sdk
            .get_identity_nonce(identity_id, true, Some(settings))
            .await
            .unwrap();
        assert_eq!(nonce, 11);

        sdk.refresh_identity_nonce(&identity_id).await;

        // Update expectation: platform now returns 20.
        sdk.mock()
            .remove_fetch_expectation::<IdentityNonceFetcher, _>(identity_id)
            .await;
        sdk.mock()
            .expect_fetch::<IdentityNonceFetcher, _>(identity_id, Some(IdentityNonceFetcher(20u64)))
            .await
            .expect("set mock expectation");

        // Phase 3: max(cached=11, platform=20) = 20, bump to 21.
        let nonce = sdk
            .get_identity_nonce(identity_id, true, Some(settings))
            .await
            .unwrap();
        assert_eq!(nonce, 21);
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

    // --- LRU eviction: oldest entry evicted when capacity exceeded ---
    #[tokio::test]
    async fn lru_eviction_when_capacity_exceeded() {
        let cap = NonZeroUsize::new(2).unwrap();
        let cache: Mutex<LruCache<u32, NonceCacheEntry>> = Mutex::new(LruCache::new(cap));

        // Insert 3 entries into a size-2 cache.
        for key in 1u32..=3 {
            NonceCache::get_or_fetch_nonce(
                &cache,
                key,
                false,
                &Default::default(),
                DEFAULT_IDENTITY_NONCE_STALE_TIME_S,
                || async move { Ok(key as u64 * 10) },
            )
            .await
            .unwrap();
        }

        let guard = cache.lock().await;
        // Key 1 should have been evicted (LRU).
        assert!(guard.peek(&1u32).is_none(), "key 1 should be evicted");
        assert!(guard.peek(&2u32).is_some(), "key 2 should still exist");
        assert!(guard.peek(&3u32).is_some(), "key 3 should still exist");
    }

    // --- SEC-001: Nonce overflow returns error on cache-hit bump ---
    #[tokio::test]
    async fn nonce_overflow_returns_error_on_cache_hit() {
        // Seed cache with nonce at the filter boundary.
        // last_fetched_platform_nonce must be close enough to avoid drift-triggered refetch.
        let cache = seeded_cache(
            1,
            IDENTITY_NONCE_VALUE_FILTER,
            now_s(),
            IDENTITY_NONCE_VALUE_FILTER,
        );
        let settings = never_stale();

        let result = NonceCache::get_or_fetch_nonce(
            &cache,
            1u32,
            true,
            &settings,
            DEFAULT_IDENTITY_NONCE_STALE_TIME_S,
            || async { panic!("should not fetch from platform") },
        )
        .await;

        assert!(
            matches!(result, Err(Error::NonceOverflow(n)) if n == IDENTITY_NONCE_VALUE_FILTER),
            "expected NonceOverflow error at filter max, got: {result:?}"
        );
    }

    // --- Cache-only bumps do NOT update last_fetch_timestamp ---
    #[tokio::test]
    async fn cache_bump_does_not_update_timestamp() {
        let original_ts = now_s() - 100; // 100 seconds ago
        let cache = seeded_cache(1, 10, original_ts, 10);
        let settings = never_stale();

        // Bump from cache (no platform fetch).
        NonceCache::get_or_fetch_nonce(
            &cache,
            1u32,
            true,
            &settings,
            DEFAULT_IDENTITY_NONCE_STALE_TIME_S,
            || async { panic!("should not fetch from platform") },
        )
        .await
        .unwrap();

        let mut guard = cache.lock().await;
        let entry = guard.get(&1u32).unwrap();
        assert_eq!(
            entry.last_fetch_timestamp, original_ts,
            "timestamp should not be updated on cache-only bump"
        );
    }

    // --- Concurrent bumps from fresh cache all produce unique nonces ---
    #[tokio::test]
    async fn concurrent_bumps_produce_unique_nonces() {
        use std::sync::Arc;

        // Keep under MAX_MISSING_IDENTITY_REVISIONS (24) so the drift
        // threshold is not hit and all tasks stay on the cache-hit path.
        let num_tasks: u64 = 20;
        let cache = Arc::new(seeded_cache(1, 0, now_s(), 0));
        let barrier = Arc::new(tokio::sync::Barrier::new(num_tasks as usize));

        let mut handles = Vec::with_capacity(num_tasks as usize);
        for _ in 0..num_tasks {
            let cache = Arc::clone(&cache);
            let barrier = Arc::clone(&barrier);
            handles.push(tokio::spawn(async move {
                barrier.wait().await;
                NonceCache::get_or_fetch_nonce(
                    &cache,
                    1u32,
                    true,
                    &PutSettings {
                        identity_nonce_stale_time_s: Some(u64::MAX),
                        ..Default::default()
                    },
                    DEFAULT_IDENTITY_NONCE_STALE_TIME_S,
                    || async { panic!("should not fetch from platform") },
                )
                .await
                .unwrap()
            }));
        }

        let mut nonces = Vec::with_capacity(num_tasks as usize);
        for handle in handles {
            nonces.push(handle.await.unwrap());
        }

        nonces.sort();
        let before = nonces.len();
        nonces.dedup();
        assert_eq!(
            nonces.len(),
            before,
            "duplicate nonces detected under concurrent access"
        );
        // Should be a contiguous range 1..=num_tasks (bumped from 0).
        assert_eq!(nonces, (1..=num_tasks).collect::<Vec<_>>());
    }

    // --- Concurrent stale fetches all produce unique nonces ---
    #[tokio::test]
    async fn concurrent_stale_fetches_produce_unique_nonces() {
        use std::sync::Arc;

        let num_tasks: u64 = 50;
        // Stale timestamp forces all tasks into Phase 2 (platform fetch).
        let cache = Arc::new(seeded_cache(1, 0, 0, 0));
        let barrier = Arc::new(tokio::sync::Barrier::new(num_tasks as usize));

        let mut handles = Vec::with_capacity(num_tasks as usize);
        for _ in 0..num_tasks {
            let cache = Arc::clone(&cache);
            let barrier = Arc::clone(&barrier);
            handles.push(tokio::spawn(async move {
                barrier.wait().await;
                NonceCache::get_or_fetch_nonce(
                    &cache,
                    1u32,
                    true,
                    &PutSettings::default(),
                    DEFAULT_IDENTITY_NONCE_STALE_TIME_S,
                    || async { Ok(100u64) },
                )
                .await
                .unwrap()
            }));
        }

        let mut nonces = Vec::with_capacity(num_tasks as usize);
        for handle in handles {
            nonces.push(handle.await.unwrap());
        }

        nonces.sort();
        let before = nonces.len();
        nonces.dedup();
        assert_eq!(
            nonces.len(),
            before,
            "duplicate nonces detected under concurrent stale fetches"
        );
        // All should be > 100 (bumped from platform nonce).
        assert!(nonces.iter().all(|&n| n > 100));
    }

    // --- Reading after bump returns the bumped nonce without incrementing ---
    #[tokio::test]
    async fn read_after_bump_returns_bumped_nonce() {
        let cache = seeded_cache(1, 5, now_s(), 5);
        let settings = never_stale();

        // bump_first=true → 6
        let nonce = NonceCache::get_or_fetch_nonce(
            &cache,
            1u32,
            true,
            &settings,
            DEFAULT_IDENTITY_NONCE_STALE_TIME_S,
            || async { panic!("should not fetch from platform") },
        )
        .await
        .unwrap();
        assert_eq!(nonce, 6);

        // bump_first=false → still 6
        let nonce = NonceCache::get_or_fetch_nonce(
            &cache,
            1u32,
            false,
            &settings,
            DEFAULT_IDENTITY_NONCE_STALE_TIME_S,
            || async { panic!("should not fetch from platform") },
        )
        .await
        .unwrap();
        assert_eq!(nonce, 6);
    }
}
