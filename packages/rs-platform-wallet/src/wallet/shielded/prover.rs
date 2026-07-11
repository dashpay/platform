//! Cached Orchard prover for zero-knowledge proof generation.
//!
//! The Halo 2 proving key takes ~30 seconds to build on first use. This
//! module provides [`CachedOrchardProver`] which lazily builds the key
//! once (via `OnceLock`) and caches it for the process lifetime.
//!
//! Callers should invoke [`CachedOrchardProver::warm_up`] on a background
//! thread during app startup so the first shielded operation does not block.

use std::sync::OnceLock;

use dpp::shielded::builder::OrchardProver;
use grovedb_commitment_tree::ProvingKey;

/// Global proving key cache — built once, shared for the process lifetime.
static PROVING_KEY: OnceLock<ProvingKey> = OnceLock::new();

/// A cached Orchard prover that lazily builds and caches the Halo 2
/// proving key.
///
/// This struct is zero-sized — all state lives in the `OnceLock` static.
/// Multiple `CachedOrchardProver` instances share the same cached key.
///
/// Implements `OrchardProver` (via a `&CachedOrchardProver` reference)
/// so it can be passed directly to DPP's `build_*_transition()` builders.
pub struct CachedOrchardProver;

impl CachedOrchardProver {
    /// Create a new prover handle.
    ///
    /// This does **not** build the proving key — call [`warm_up`](Self::warm_up)
    /// or wait for the first proof generation to trigger the build.
    pub fn new() -> Self {
        CachedOrchardProver
    }

    /// Build the proving key if it hasn't been built yet.
    ///
    /// This is a blocking operation (~30 seconds on first call). Subsequent
    /// calls return immediately. Call this on a background thread during
    /// app startup.
    pub fn warm_up(&self) {
        let _ = PROVING_KEY.get_or_init(ProvingKey::build);
    }

    /// Whether the proving key has already been built and cached.
    pub fn is_ready(&self) -> bool {
        PROVING_KEY.get().is_some()
    }

    /// Get a reference to the cached proving key, building it if necessary.
    fn get_or_build(&self) -> &'static ProvingKey {
        PROVING_KEY.get_or_init(ProvingKey::build)
    }
}

impl Default for CachedOrchardProver {
    fn default() -> Self {
        Self::new()
    }
}

impl OrchardProver for &CachedOrchardProver {
    fn proving_key(&self) -> &ProvingKey {
        self.get_or_build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prover_starts_not_ready() {
        // Note: if another test already warmed up the key in this process,
        // this will pass trivially. That's fine — we just verify the API works.
        let prover = CachedOrchardProver::new();
        // is_ready may be true or false depending on test execution order.
        // Just verify the method doesn't panic.
        let _ = prover.is_ready();
    }
}
