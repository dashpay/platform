import Foundation
import SwiftData

/// SwiftData row for one derived shielded-activity entry — a user-facing
/// record of a shielded operation (shield, send, unshield, withdrawal,
/// identity-create, …).
///
/// Mirrors `platform_wallet::wallet::shielded::ShieldedActivityEntry`
/// from the Rust side. Written two ways, both scoped by
/// `(walletId, accountIndex, entryId)`:
/// - **live** — each shielded operation records a rich entry at execution
///   time (exact kind, amount, fee, counterparty, memo, identity id);
/// - **scan-derived** — on a restored wallet the sync pass reconstructs
///   entries best-effort from persisted notes / outgoing notes.
///
/// The persister callback upserts by that tuple — `entryId` alone is not
/// globally unique across accounts (an intra-wallet transfer writes a
/// Sent row on the sending account and a Received row on the receiving
/// account sharing one `entryId`). Re-persisting the same tuple flips a
/// `Pending` row to `Confirmed`/`Failed` in place, and a coarse
/// scan-derived `ShieldedSpend` can be refined to a specific kind when a
/// richer entry re-emits the same id (the id = sha256 of the visible
/// output cmxs is identical across both paths by construction).
///
/// On cold start the matching `loadShieldedActivity` callback streams
/// every row back to Rust so the scan deriver's dedupe set includes it
/// and a re-derivation never clobbers a rich live entry.
@Model
public final class PersistentShieldedActivity {
    /// Composite uniqueness on `(walletId, accountIndex, entryId)` — at
    /// most one activity row per operation PER subwallet. The subwallet
    /// scope is required, not cosmetic: `entryId` is a content hash that
    /// two accounts of one wallet legitimately share (intra-wallet
    /// transfer → Sent on the sender, Received on the receiver).
    #Unique<PersistentShieldedActivity>([\.walletId, \.accountIndex, \.entryId])
    /// Index `(walletId, accountIndex)` so per-subwallet history scans
    /// hit an index instead of the full table.
    #Index<PersistentShieldedActivity>([\.walletId, \.accountIndex])

    /// 32-byte wallet identifier (matches `PersistentWallet.walletId`).
    public var walletId: Data
    /// ZIP-32 account index inside the wallet.
    public var accountIndex: UInt32
    /// Entry id (sha256 of sorted visible output cmxs, 32 bytes). Natural key.
    public var entryId: Data

    /// Kind discriminant (`ShieldedActivityKind::tag`): 0 Shield,
    /// 1 ShieldFromAssetLock, 2 Received, 3 Sent, 4 Unshield,
    /// 5 Withdrawal, 6 IdentityCreate, 7 ShieldedSpend.
    public var kindTag: Int
    /// Direction: 0 In, 1 Out, 2 Self.
    public var direction: Int
    /// Status: 0 Pending, 1 Confirmed, 2 Failed.
    public var status: Int

    /// Display amount in credits (principal; excludes self-change /
    /// zero-value fillers).
    public var amount: UInt64
    /// Exact fee in credits when `hasFee == true`; otherwise unknown.
    public var fee: UInt64
    public var hasFee: Bool
    /// Block height when `hasBlockHeight == true` (confirmed); otherwise
    /// pending. Canonical sort key (desc) with pendings floated up.
    public var blockHeight: UInt64
    public var hasBlockHeight: Bool
    /// Record time in ms since the Unix epoch (display-only / sort
    /// tiebreak).
    public var createdAtMs: UInt64

    /// Created identity id (32 bytes) when `kindTag == 6`
    /// (IdentityCreate); empty otherwise.
    public var identityId: Data
    /// Counterparty bytes (43B Orchard / 21B PlatformAddress / Core
    /// script) when present; empty otherwise.
    public var counterparty: Data
    /// 36-byte memo when present and non-zero; empty otherwise.
    public var memo: Data
    /// Visible output cmxs that fed `entryId` (`count` × 32 bytes,
    /// concatenated). Linkage for confirmation / dedupe.
    public var noteCmxs: Data
    /// Spent nullifiers (`count` × 32 bytes, concatenated). Empty for
    /// inbound kinds.
    public var spentNullifiers: Data

    /// Insertion timestamps.
    public var createdAt: Date
    public var lastUpdated: Date

    public init(
        walletId: Data,
        accountIndex: UInt32,
        entryId: Data,
        kindTag: Int,
        direction: Int,
        status: Int,
        amount: UInt64,
        fee: UInt64,
        hasFee: Bool,
        blockHeight: UInt64,
        hasBlockHeight: Bool,
        createdAtMs: UInt64,
        identityId: Data,
        counterparty: Data,
        memo: Data,
        noteCmxs: Data,
        spentNullifiers: Data
    ) {
        self.walletId = walletId
        self.accountIndex = accountIndex
        self.entryId = entryId
        self.kindTag = kindTag
        self.direction = direction
        self.status = status
        self.amount = amount
        self.fee = fee
        self.hasFee = hasFee
        self.blockHeight = blockHeight
        self.hasBlockHeight = hasBlockHeight
        self.createdAtMs = createdAtMs
        self.identityId = identityId
        self.counterparty = counterparty
        self.memo = memo
        self.noteCmxs = noteCmxs
        self.spentNullifiers = spentNullifiers
        let now = Date()
        self.createdAt = now
        self.lastUpdated = now
    }
}
