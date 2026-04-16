import Foundation
import SwiftData

/// SwiftData model for persisting BLAST address sync state.
///
/// Stores the incremental-sync watermark so the engine can resume
/// from where it left off on app restart instead of doing a full
/// trunk/branch/compact rescan.
@Model
public final class PersistentSyncState {
    /// 32-byte wallet ID.
    @Attribute(.unique) public var walletId: Data
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
        syncHeight: UInt64,
        syncTimestamp: UInt64,
        lastKnownRecentBlock: UInt64
    ) {
        self.walletId = walletId
        self.syncHeight = syncHeight
        self.syncTimestamp = syncTimestamp
        self.lastKnownRecentBlock = lastKnownRecentBlock
        self.lastUpdated = Date()
    }
}
