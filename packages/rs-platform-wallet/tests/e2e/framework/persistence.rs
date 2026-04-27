//! Persistence shim for the e2e framework.
//!
//! Bank and test wallets use `NoPlatformPersistence` — every wallet
//! is reconstructible from its seed (registry-backed for test
//! wallets, env-var for the bank), so dropping the changeset deltas
//! between runs is safe and cheap. The trade-off is a single BLAST
//! pass at startup, which is fast on testnet.
//!
//! Wave 2 stub: declares a placeholder wrapper. Wave 3 either
//! re-exports `platform_wallet::persister::NoPlatformPersistence`
//! directly or defines a thin wrapper that records deltas in-memory
//! for assertions during cleanup-flow tests.

/// Marker stub for the persister handle.
///
/// Wave 2 placeholder — Wave 3 replaces with the real persister
/// type the harness uses.
pub struct TestPersister(());

impl TestPersister {
    /// Build a fresh persister.
    pub fn new() -> Self {
        Self(())
    }
}

impl Default for TestPersister {
    fn default() -> Self {
        Self::new()
    }
}
