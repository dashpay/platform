import Foundation
import SwiftData

/// SwiftData row for per-subwallet shielded sync watermarks.
///
/// Mirrors `platform_wallet::changeset::ShieldedChangeSet::synced_indices`
/// + `nullifier_checkpoints` from the Rust side. One row per
/// `(walletId, accountIndex)`. Updated via the
/// `on_persist_shielded_synced_indices_fn` and
/// `on_persist_shielded_nullifier_checkpoints_fn` FFI callbacks;
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
    /// Whether the optional `(height, timestamp)` nullifier
    /// checkpoint is populated. SwiftData predicate compilation is
    /// finicky around chained optionals; an explicit `Bool` flag
    /// keeps the watermark-restore query simple.
    public var hasNullifierCheckpoint: Bool
    /// Block height of the most recent nullifier sync pass.
    /// Meaningful iff `hasNullifierCheckpoint == true`.
    public var nullifierCheckpointHeight: UInt64
    /// Block timestamp (Unix seconds) of the most recent pass.
    /// Meaningful iff `hasNullifierCheckpoint == true`.
    public var nullifierCheckpointTimestamp: UInt64

    public var lastUpdated: Date

    public init(
        walletId: Data,
        accountIndex: UInt32,
        lastSyncedIndex: UInt64 = 0,
        hasNullifierCheckpoint: Bool = false,
        nullifierCheckpointHeight: UInt64 = 0,
        nullifierCheckpointTimestamp: UInt64 = 0
    ) {
        self.walletId = walletId
        self.accountIndex = accountIndex
        self.lastSyncedIndex = lastSyncedIndex
        self.hasNullifierCheckpoint = hasNullifierCheckpoint
        self.nullifierCheckpointHeight = nullifierCheckpointHeight
        self.nullifierCheckpointTimestamp = nullifierCheckpointTimestamp
        self.lastUpdated = Date()
    }
}
