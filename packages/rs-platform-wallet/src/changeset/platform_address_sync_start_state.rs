//! Platform-address provider state restored from storage.
//!
//! Returned as part of [`ClientStartState`](crate::changeset::ClientStartState)
//! by [`PlatformWalletPersistence::load`](crate::changeset::PlatformWalletPersistence::load)
//! so the platform wallet can rebuild
//! [`PlatformPaymentAddressProvider`](crate::wallet::platform_addresses::PlatformPaymentAddressProvider)
//! without replaying a changeset.

use crate::wallet::platform_addresses::PerWalletPlatformAddressState;

/// Platform-address provider state restored from storage.
///
/// Carries everything
/// [`PlatformPaymentAddressProvider::from_persisted`](crate::wallet::platform_addresses::PlatformPaymentAddressProvider::from_persisted)
/// needs to rebuild one wallet's provider — the per-account xpub +
/// `found`/`absent` map and the shared network-scoped incremental-sync
/// watermark. The live `AddressPool` in the wallet manager supplies
/// the current address set at reconstruction time.
#[derive(Debug, Default)]
pub struct PlatformAddressSyncStartState {
    /// Per-account committed state to restore for one wallet.
    pub per_account: PerWalletPlatformAddressState,
    /// Highest block height covered by the last successful sync.
    pub sync_height: u64,
    /// Latest block timestamp (Unix seconds) covered by the last sync.
    pub sync_timestamp: u64,
    /// Last block height with recent address changes (compaction marker).
    pub last_known_recent_block: u64,
}
