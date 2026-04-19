import Foundation
import SwiftData

/// SwiftData model for persisting BLAST address sync state.
///
/// Stores the network-scoped incremental-sync watermark so the engine
/// can resume from where it left off on app restart instead of doing
/// a full trunk/branch/compact rescan.
@Model
public final class PersistentSyncState {
    /// Stable 32-byte scope key for this sync-state row.
    ///
    /// Kept as `walletId` for schema compatibility, but the persisted
    /// value is now a network-scoped key (one row per network) rather
    /// than a concrete wallet id.
    @Attribute(.unique) public var walletId: Data
    /// Network this sync checkpoint belongs to.
    public var network: String
    /// Highest block height covered by the last sync.
    public var syncHeight: UInt64
    /// Latest timestamp covered by the last sync (Unix seconds).
    public var syncTimestamp: UInt64
    /// Last block height with recent address changes (compaction marker).
    public var lastKnownRecentBlock: UInt64
    /// Last time this record was updated.
    public var lastUpdated: Date

    public init(
        walletId: Data,
        network: String,
        syncHeight: UInt64,
        syncTimestamp: UInt64,
        lastKnownRecentBlock: UInt64
    ) {
        self.walletId = walletId
        self.network = network
        self.syncHeight = syncHeight
        self.syncTimestamp = syncTimestamp
        self.lastKnownRecentBlock = lastKnownRecentBlock
        self.lastUpdated = Date()
    }
}
