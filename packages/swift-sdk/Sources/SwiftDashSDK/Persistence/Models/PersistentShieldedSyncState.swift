import Foundation
import SwiftData

/// SwiftData row for per-subwallet shielded sync watermarks.
///
/// Mirrors `platform_wallet::changeset::ShieldedChangeSet::synced_indices`
/// from the Rust side. One row per `(walletId, accountIndex)`. Updated
/// via the `on_persist_shielded_synced_indices_fn` FFI callback;
/// streamed back to Rust on cold start via
/// `on_load_shielded_sync_states_fn` so the rehydrated
/// `SubwalletState` resumes incremental sync from where it left off.
@Model
public final class PersistentShieldedSyncState {
    /// Composite uniqueness on `(walletId, accountIndex)` — at
    /// most one watermark row per subwallet.
    #Unique<PersistentShieldedSyncState>([\.walletId, \.accountIndex])
    #Index<PersistentShieldedSyncState>([\.walletId])

    public var walletId: Data
    public var accountIndex: UInt32
    /// Sync watermark: count of note positions scanned = the next global
    /// commitment-tree index to scan (exclusive). `0` = nothing scanned
    /// yet — *not* the last index scanned.
    public var lastSyncedIndex: UInt64

    public var lastUpdated: Date

    public init(
        walletId: Data,
        accountIndex: UInt32,
        lastSyncedIndex: UInt64 = 0
    ) {
        self.walletId = walletId
        self.accountIndex = accountIndex
        self.lastSyncedIndex = lastSyncedIndex
        self.lastUpdated = Date()
    }
}
