import Foundation
import SwiftData

/// SwiftData model for persisting a single tracked asset-lock credit
/// output (DIP-0027). Mirrors
/// [`AssetLockEntry`](platform_wallet::changeset::AssetLockEntry) on
/// the Rust side, one row per `(walletId, outpoint)`. Upserted by the
/// `on_persist_asset_locks_fn` callback whenever the asset-lock
/// manager flushes a status transition (Built → Broadcast →
/// InstantSendLocked → ChainLocked) and deleted when the lock is
/// consumed by a successful identity-registration / top-up flow.
///
/// Two consumers:
/// 1. `RegistrationProgressView` reads `statusRaw` to drive the
///    stage progress bar (`@Query` filtered by `walletId +
///    identityIndexRaw`).
/// 2. The wallet load path rebuilds `unused_asset_locks` on the
///    Rust side from these rows so an in-flight registration that
///    was interrupted by an app kill can resume from the latest
///    status without rebroadcasting the asset-lock transaction.
///
/// ## Destination conventions per funding type
///
/// The destination of the asset lock — what was funded — is stored
/// in different fields depending on `fundingTypeRaw`. Each funding
/// type owns one typed field-family rather than sharing a
/// polymorphic `destinationBytes: Data?` blob; the typed shape
/// keeps SwiftData predicates working and makes per-type queries
/// readable.
///
/// - **Identity** (`fundingTypeRaw ∈ {0, 1, 2, 3}` —
///   IdentityRegistration / IdentityTopUp / IdentityTopUpNotBound /
///   IdentityInvitation): `identityIndexRaw` carries the HD slot of
///   the destination identity. The identity row itself can be
///   resolved via `PersistentIdentity` joined on
///   `(walletId, identityIndex)`.
///
/// - **Platform address** (`fundingTypeRaw == 4` —
///   AssetLockAddressTopUp):
///   `recipientPlatformAddressHash` + `recipientPlatformAddressType`
///   identify the destination, and `recipientIsExternal` says whether
///   it belongs to this wallet or to a third party. Set by Swift on
///   the controller's `.completed` phase because the recipient is
///   picked at ST-submit time on the host side; Rust never sees it.
///
/// - **Shielded address** (`fundingTypeRaw == 5` —
///   AssetLockShieldedAddressTopUp, not yet wired): will add a
///   dedicated `recipientShielded*` field family when the
///   shielded-funding flow lands. Keep this convention — one typed
///   field-family per funding type rather than a polymorphic
///   `destinationBytes` blob.
@Model
public final class PersistentAssetLock {
    /// Index `walletId` so per-wallet asset-lock scans (the progress
    /// bar's `@Query`, the storage explorer's wallet-scoped drill-
    /// down, the load-time rehydration path) hit an index instead of
    /// scanning the whole table.
    #Index<PersistentAssetLock>([\.walletId])

    /// 36-byte outpoint encoded as `<txid hex (display order)>:<vout>`.
    /// Matches the formatting used by `PersistentTxo.outpointHex` /
    /// `PersistentPendingInput`'s outpoint surface so the same lock
    /// is identifiable across all three. Unique across the SwiftData
    /// store — a collision would imply two wallets producing the
    /// same outpoint, which is unreachable in practice.
    @Attribute(.unique) public var outPointHex: String

    /// 32-byte wallet id owning this asset lock.
    public var walletId: Data

    /// Consensus-encoded asset-lock transaction (Core special
    /// transaction type 8, `AssetLockPayload`). Carried so the load
    /// path can re-instantiate the `TrackedAssetLock` without
    /// rebroadcasting.
    public var transactionBytes: Data

    /// Discriminant of [`AssetLockFundingType`]:
    /// 0 = IdentityRegistration, 1 = IdentityTopUp, 2 = IdentityTopUpNotBound,
    /// 3 = IdentityInvitation, 4 = AssetLockAddressTopUp,
    /// 5 = AssetLockShieldedAddressTopUp. Stored as `Int` so SwiftData
    /// predicates can compare directly without a cast.
    public var fundingTypeRaw: Int

    /// Identity index slot consumed by this asset lock — the source of
    /// truth for matching against an in-flight registration's
    /// `RegistrationProgressView`. Stored as `Int32` so `#Predicate`
    /// can compare against a Swift-side `UInt32` lossily-cast value
    /// without overflow surprises (identity indices stay well under
    /// `Int32.max`).
    public var identityIndexRaw: Int32

    /// BIP44 account index the asset-lock funding tx was built from.
    /// The Rust restore path uses this value to reinsert the
    /// unresolved funding tx into `standard_bip44_accounts[account_index]`
    /// at load time — a wrong value silently drops the record. The
    /// SDK surface (`ManagedAssetLockManager.buildTransaction` and
    /// friends) already accepts any `accountIndex: UInt32`, so this
    /// column captures the actual value rather than always defaulting
    /// to 0.
    ///
    /// Default `0` on the column makes SwiftData's lightweight
    /// migration safe for rows that pre-date this field — the same
    /// hardcoded value the restore path used before, so behavior is
    /// preserved bit-exactly for the BIP44-account-0 common case.
    /// Wallets that funded from a non-zero account on the old code
    /// have a latent broken row either way; restoring with `0` here
    /// matches the pre-this-commit behavior. New rows always carry
    /// the real value.
    public var accountIndexRaw: Int32 = 0

    /// Locked amount in duffs (1 DASH = 1e8 duffs). Stored as `Int64`
    /// for the same predicate-friendliness reason as
    /// `identityIndexRaw`.
    public var amountDuffs: Int64

    /// Discriminant of [`AssetLockStatus`]:
    /// 0 = Built, 1 = Broadcast, 2 = InstantSendLocked, 3 = ChainLocked,
    /// 4 = Consumed, 5 = RecoveredFromChain. Stored as `Int` so
    /// `#Predicate` can match raw values directly (the progress bar
    /// compares against 0/1/2/3 and the resumable-locks filter against
    /// 4 to hide already-spent rows).
    ///
    /// `5` (RecoveredFromChain) rows are written by restore reconstruction or
    /// live reconciliation of an unauthenticated already-consumed report. The
    /// lock is confirmed on Core but its Platform-side consumption is unknown,
    /// so UIs must treat it as neither pending (1…3) nor done (4).
    public var statusRaw: Int

    /// Bincode-encoded `AssetLockProof` (`dpp::bincode::config::standard()`).
    /// Absent (`nil`) until the lock reaches `InstantSendLocked` /
    /// `ChainLocked`. The load path passes these bytes back over FFI
    /// where Rust decodes them into the live proof.
    public var proofBytes: Data?

    /// 20-byte hash of the recipient platform address for asset
    /// locks consumed by an `AddressFundingFromAssetLockTransition`
    /// (`fundingTypeRaw == 4`). Populated by Swift after a
    /// successful `fundFromAssetLock` call — the recipient is
    /// known on the caller side, not on the Rust side (which only
    /// tracks the credit-output key, not the destination address).
    ///
    /// `nil` for:
    /// - Identity-funding asset locks (the destination is the
    ///   newly-created identity, surfaced via the `identityIndex`
    ///   slot instead).
    /// - Address-funding locks that haven't completed yet (status
    ///   < Consumed).
    /// - Pre-this-commit address-funding locks that completed
    ///   before the field existed.
    ///
    /// Default `nil` on the column makes SwiftData's lightweight
    /// migration safe for rows that pre-date this field.
    public var recipientPlatformAddressHash: Data?

    /// `PlatformAddress` type byte (0 = P2PKH, 1 = P2SH) matching
    /// `recipientPlatformAddressHash`. Stored alongside the hash so
    /// the storage explorer can render a typed bech32m string
    /// without joining against `PersistentPlatformAddress`. `nil`
    /// whenever `recipientPlatformAddressHash` is `nil`.
    public var recipientPlatformAddressType: UInt8?

    /// Whether `recipientPlatformAddressHash` is a THIRD PARTY's
    /// address (`true`) or one of this wallet's own addresses
    /// (`false`) — the own/external discriminator for the funding
    /// type 4 field-family.
    ///
    /// Without it, a populated recipient hash is ambiguous. Consumers
    /// read that hash as "this lock topped up an address of mine" —
    /// `PlatformAddressActivityStore.matchesOwnAssetLockTopUp` in
    /// dashwallet-ios does exactly that — so an outgoing payment to
    /// someone else would be rendered as an incoming credit to the
    /// user. `true` marks the row as an outgoing send whose recipient
    /// hash names a stranger and therefore will NEVER have a matching
    /// `PersistentPlatformAddress` row.
    ///
    /// Written by the caller that picked the recipient, alongside the
    /// hash and type: `false` for `fundFromAssetLock`, `true` for
    /// `fundFromAssetLockExternal`.
    ///
    /// `nil` for:
    /// - Identity-funding asset locks (no platform-address recipient).
    /// - Address-funding locks that haven't completed yet.
    /// - Pre-this-commit address-funding locks that completed before
    ///   the field existed. Treat `nil` alongside a populated
    ///   `recipientPlatformAddressHash` as "own" — that was the only
    ///   flow those rows could have come from.
    ///
    /// ## Migration
    ///
    /// Adding this property is a lightweight-migratable change (a new
    /// optional column backfilled `NULL`), but it is NOT a free one: a
    /// `VersionedSchema` identifies a store by the CHECKSUM of the
    /// entities it declares, so adding a property to the live model
    /// mutates the checksum of every registered schema version that
    /// references it. The model LIST being unchanged is irrelevant.
    ///
    /// Left unaddressed, a store written by the V2 binary would match no
    /// schema in `DashMigrationPlan.schemas` and
    /// `ModelContainer(for:migrationPlan:configurations:)` would fail to
    /// open it with Cocoa error 134504 ("Cannot use staged migration with
    /// an unknown model version"). So V1 and V2 now reference a frozen
    /// copy of this model (`DashSchemaV1.PersistentAssetLock`, in
    /// `DashSchemaFrozenModels.swift`), this property is what schema
    /// `DashSchemaV3` adds, and a lightweight V2 -> V3 stage carries
    /// existing stores across. Do the same for the next property added
    /// here.
    public var recipientIsExternal: Bool?

    /// Record timestamps.
    public var createdAt: Date
    public var updatedAt: Date

    public init(
        outPointHex: String,
        walletId: Data,
        transactionBytes: Data,
        fundingTypeRaw: Int,
        identityIndexRaw: Int32,
        accountIndexRaw: Int32 = 0,
        amountDuffs: Int64,
        statusRaw: Int,
        proofBytes: Data? = nil
    ) {
        self.outPointHex = outPointHex
        self.walletId = walletId
        self.transactionBytes = transactionBytes
        self.fundingTypeRaw = fundingTypeRaw
        self.identityIndexRaw = identityIndexRaw
        self.accountIndexRaw = accountIndexRaw
        self.amountDuffs = amountDuffs
        self.statusRaw = statusRaw
        self.proofBytes = proofBytes
        self.createdAt = Date()
        self.updatedAt = Date()
    }
}

// MARK: - Queries

extension PersistentAssetLock {
    /// Per-wallet predicate. Indexed scan via the `walletId` index.
    public static func predicate(walletId: Data) -> Predicate<PersistentAssetLock> {
        #Predicate<PersistentAssetLock> { entry in
            entry.walletId == walletId
        }
    }

    /// Per-slot predicate keyed by `(walletId, identityIndex)` — used
    /// by `RegistrationProgressView` to find the in-flight lock for
    /// a particular registration slot.
    public static func predicate(
        walletId: Data,
        identityIndex: UInt32
    ) -> Predicate<PersistentAssetLock> {
        let identityIndexRaw = Int32(bitPattern: identityIndex)
        return #Predicate<PersistentAssetLock> { entry in
            entry.walletId == walletId && entry.identityIndexRaw == identityIndexRaw
        }
    }
}

// MARK: - Outpoint encoding helpers

extension PersistentAssetLock {
    /// Encode a 36-byte raw outpoint (`txid_le || vout_le`) — matching
    /// the layout used by the Rust-side `AssetLockEntryFFI.out_point`
    /// — as the canonical display-order hex string
    /// `<txid display hex>:<vout>`.
    ///
    /// The Rust side serializes the txid in raw byte order (little-
    /// endian on the wire); display order is the reverse, same as
    /// `PersistentTxo.outpointHex`.
    public static func encodeOutPoint(rawBytes: Data) -> String {
        precondition(rawBytes.count == 36, "outpoint must be 36 bytes")
        let txid = rawBytes.prefix(32)
        let voutBytes = rawBytes.suffix(4)
        // Byte-copy through a local `UInt32` rather than calling
        // `raw.load(as: UInt32.self)` directly: `Data.suffix(4)`'s
        // `withUnsafeBytes` hands us a pointer into the underlying
        // storage, whose alignment Swift's `Data` does NOT guarantee
        // for `UInt32` (4-byte). On ARM64 a misaligned load traps;
        // copying the four bytes into an aligned local sidesteps the
        // requirement. Matches the same pattern used elsewhere in
        // this SDK (`PlatformWalletManager.runCatchUp`,
        // `ManagedAssetLockManager`) for FFI byte-array decoding.
        let vout = voutBytes.withUnsafeBytes { raw -> UInt32 in
            var value: UInt32 = 0
            withUnsafeMutableBytes(of: &value) { dst in
                dst.copyBytes(from: raw.prefix(MemoryLayout<UInt32>.size))
            }
            return UInt32(littleEndian: value)
        }
        let txidHex = txid.reversed().map { String(format: "%02x", $0) }.joined()
        return "\(txidHex):\(vout)"
    }
}
