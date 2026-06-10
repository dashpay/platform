import Foundation
import SwiftData

/// SwiftData row for one outgoing (sent) shielded (Orchard) note,
/// recovered via the wallet's outgoing viewing key (OVK) during a
/// scan.
///
/// Mirrors `platform_wallet::changeset::ShieldedChangeSet::outgoing_notes`
/// from the Rust side. Distinct from `PersistentShieldedNote` (a
/// received, spendable note): this is append-only *send* history with
/// no nullifier / position / spend state. The persister callback
/// writes one row per `(walletId, accountIndex, cmx)`; re-persisting
/// the same `cmx` (a re-scan) is an idempotent upsert.
///
/// On cold start the matching `loadShieldedOutgoingNotes` callback
/// streams every row back to Rust so
/// `ShieldedWallet::restore_from_snapshot` can rehydrate
/// `SubwalletState.outgoing_notes` before the first sync runs.
@Model
public final class PersistentShieldedOutgoingNote {
    /// Composite uniqueness on `(walletId, accountIndex, cmx)` — at
    /// most one send-history row per sent note. `cmx` alone is the
    /// natural key (note commitments are unique), but scoping the
    /// constraint by subwallet matches how the row is addressed on
    /// both the persist and load paths.
    #Unique<PersistentShieldedOutgoingNote>([\.walletId, \.accountIndex, \.cmx])
    /// Index `(walletId, accountIndex)` so per-subwallet history
    /// scans hit an index instead of the full table.
    #Index<PersistentShieldedOutgoingNote>([\.walletId, \.accountIndex])

    /// 32-byte wallet identifier (matches `PersistentWallet.walletId`).
    public var walletId: Data
    /// ZIP-32 account index inside the wallet.
    public var accountIndex: UInt32
    /// Note commitment (cmx) of the sent note (32 bytes). Natural key.
    public var cmx: Data
    /// Recipient's raw Orchard address (43 bytes).
    public var recipient: Data
    /// Value sent, in credits.
    public var value: UInt64
    /// Raw Dash memo bytes (up to 36 bytes).
    public var memo: Data
    /// Block height the sent note appeared at.
    public var blockHeight: UInt64

    /// Insertion timestamps.
    public var createdAt: Date
    public var lastUpdated: Date

    public init(
        walletId: Data,
        accountIndex: UInt32,
        cmx: Data,
        recipient: Data,
        value: UInt64,
        memo: Data,
        blockHeight: UInt64
    ) {
        self.walletId = walletId
        self.accountIndex = accountIndex
        self.cmx = cmx
        self.recipient = recipient
        self.value = value
        self.memo = memo
        self.blockHeight = blockHeight
        let now = Date()
        self.createdAt = now
        self.lastUpdated = now
    }
}
