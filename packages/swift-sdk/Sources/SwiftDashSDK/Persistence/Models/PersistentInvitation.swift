import Foundation
import SwiftData

/// SwiftData model for a single **created** DashPay invitation (DIP-13),
/// one row per `(walletId, outpoint)`. Upserted by the
/// `on_persist_invitations_fn` persister callback whenever
/// `create_invitation` flushes an `InvitationChangeSet`; the "Sent
/// invitations" list (`InvitationsView`) reads it via `@Query`.
///
/// SwiftData is the UI source of truth — there is **no** Rust→Swift
/// rehydrate path (the `funding_index` re-derives the voucher key), so a
/// store wipe loses only list *visibility*, never funds. **No secret
/// column:** the one-time voucher key is never stored.
///
/// Mirrors the `on_persist_asset_locks_fn` / `PersistentAssetLock`
/// push-callback path, but simpler: `InvitationEntry` is all-POD (no owned
/// byte buffers), so there is no `…Storage` Vec and no pointer-lifetime
/// management on either side.
@Model
public final class PersistentInvitation {
    /// Index `walletId` so the per-wallet "Sent invitations" `@Query` hits an
    /// index instead of scanning the whole table.
    #Index<PersistentInvitation>([\.walletId])

    /// 36-byte outpoint encoded as `<txid display hex>:<vout>` — identical form
    /// to `PersistentAssetLock.outPointHex`, produced by
    /// `PersistentAssetLock.encodeOutPoint`. The unique key (the T1 seam):
    /// the upsert stores this exact string and a removal derives the identical
    /// string from the same function. Globally unique (unscoped by wallet) —
    /// on-chain outpoints are globally unique.
    @Attribute(.unique) public var outPointHex: String

    /// Raw 36-byte outpoint (`txid_le ‖ vout_le`), stored alongside
    /// `outPointHex` so the reclaim flow can rebuild an `OutPointFFI` directly
    /// without a reverse-decode of the display string (the T1 misaligned-load
    /// error class we specifically avoid). The shim already has these bytes
    /// in-hand from `InvitationEntryFFI.out_point`.
    public var rawOutPoint: Data

    /// 32-byte wallet id that created this invitation. Drives the view's
    /// `@Query` filter (set on BOTH the insert and the update branch).
    public var walletId: Data

    /// DIP-13 invitation funding index (`m/9'/coin'/5'/3'/<index>'`) — the
    /// handle that re-derives the voucher key and drives recovery/reclaim.
    /// Stored as `Int` for `#Predicate`-friendliness.
    public var fundingIndexRaw: Int

    /// Amount locked in the voucher (duffs). `Int64` for predicate-friendliness.
    public var amountDuffs: Int64

    /// Advisory expiry (unix seconds); not consensus-enforced.
    public var expiryUnix: Int

    /// Creation time (unix seconds), from the Rust changeset.
    public var createdAtSecs: Int

    /// Whether the link carried inviter info (contact-bootstrap opted in).
    public var hasInviter: Bool

    /// Invitation status discriminant: 0 = Created, 1 = Claimed, 2 = Reclaimed.
    /// Stored as `Int` so `#Predicate` matches raw values directly (the Swift
    /// `Int` side has no compiler exhaustiveness — the view maps an unknown
    /// value to an explicit `.unknown` label).
    public var statusRaw: Int

    /// Set true just before this wallet's own reclaim consume is submitted, and
    /// cleared once the terminal status is saved. It survives a crash between the
    /// on-chain consume and the `statusRaw = 2` save, so a retry that hits
    /// "already consumed" can tell *our own* reclaim (in-flight → Reclaimed) from
    /// a voucher someone else claimed (never in-flight → Claimed). Defaults to
    /// `false`, making it an additive SwiftData migration (the property-level
    /// default lets existing rows migrate without a mapping model).
    public var reclaimInFlight: Bool = false

    /// Record timestamps.
    public var createdAt: Date
    public var updatedAt: Date

    public init(
        outPointHex: String,
        rawOutPoint: Data,
        walletId: Data,
        fundingIndexRaw: Int,
        amountDuffs: Int64,
        expiryUnix: Int,
        createdAtSecs: Int,
        hasInviter: Bool,
        statusRaw: Int,
        reclaimInFlight: Bool = false
    ) {
        self.outPointHex = outPointHex
        self.rawOutPoint = rawOutPoint
        self.walletId = walletId
        self.fundingIndexRaw = fundingIndexRaw
        self.amountDuffs = amountDuffs
        self.expiryUnix = expiryUnix
        self.createdAtSecs = createdAtSecs
        self.hasInviter = hasInviter
        self.statusRaw = statusRaw
        self.reclaimInFlight = reclaimInFlight
        self.createdAt = Date()
        self.updatedAt = Date()
    }
}

// MARK: - Queries

extension PersistentInvitation {
    /// Per-wallet predicate. Indexed scan via the `walletId` index.
    public static func predicate(walletId: Data) -> Predicate<PersistentInvitation> {
        #Predicate<PersistentInvitation> { entry in
            entry.walletId == walletId
        }
    }
}
