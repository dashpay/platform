import Foundation
import SwiftData

/// SwiftData row for one DashPay payment-history entry — a mirror of
/// the Rust-side `PaymentEntry` map on a `ManagedIdentity`
/// (`dashpay_payments`, keyed by txid).
///
/// Unlike the contact-request rows this model is **not** populated by
/// the persister callback. The Rust persister doesn't project payment
/// history; rows are refreshed on demand from the
/// `managed_identity_get_dashpay_payments` FFI getter via
/// `PlatformWalletManager.refreshDashPayPayments(walletId:identityId:)`,
/// which upserts here so the UI can `@Query` payments reactively.
///
/// One row per `(network, owner, txid)` — the Rust map is keyed by
/// txid per identity, scoped by network so two networks don't collide
/// in a shared local store.
///
/// The source `PaymentEntry` carries no timestamp field (the model
/// keys history by txid and records no wall-clock time), so none is
/// persisted — `createdAt` / `lastUpdated` below are local row
/// bookkeeping, not payment dates.
///
/// Cascade-deleted from `PersistentIdentity.dashpayPayments` — losing
/// the owner identity drops its payment history.
@Model
public final class PersistentDashpayPayment {
    /// Compound uniqueness on `(networkRaw, ownerIdentityId, txid)`.
    /// Mirrors the per-identity txid keying of the Rust
    /// `dashpay_payments` map.
    #Unique<PersistentDashpayPayment>([
        \.networkRaw, \.ownerIdentityId, \.txid
    ])

    /// Network discriminant. `UInt32` mirror of `Network.rawValue` —
    /// Foundation's predicate engine compares it directly without a
    /// custom converter. Kept in sync with `owner.networkRaw` by the
    /// init.
    public var networkRaw: UInt32

    /// Type-safe accessor over `networkRaw`. Falls back to `.testnet`
    /// if the stored raw value drifts.
    public var network: Network {
        get { Network(rawValue: networkRaw) ?? .testnet }
        set { networkRaw = newValue.rawValue }
    }

    /// Owning (wallet-managed) identity's 32-byte id, denormalized so
    /// `#Predicate` filters can match without a relationship traversal
    /// through the `owner` join. Always equal to `owner.identityId` —
    /// kept in sync by the refresh path.
    public var ownerIdentityId: Data

    /// The other identity in this payment
    /// (`DashpayPaymentFFI::counterparty_id`). Whether they are the
    /// sender or the receiver is encoded in `directionRaw`.
    public var counterpartyIdentityId: Data

    /// Amount in duffs. Always positive; `directionRaw` carries the
    /// sign.
    public var amountDuffs: UInt64

    /// Raw `DashPayPaymentDirection` value. Stored as the scalar so
    /// the predicate engine compares it directly.
    public var directionRaw: UInt8

    /// Type-safe accessor over `directionRaw`. Falls back to `.sent`
    /// if the stored raw value drifts.
    public var direction: DashPayPaymentDirection {
        get { DashPayPaymentDirection(rawValue: directionRaw) ?? .sent }
        set { directionRaw = newValue.rawValue }
    }

    /// Raw `DashPayPaymentStatus` value.
    public var statusRaw: UInt8

    /// Type-safe accessor over `statusRaw`. Falls back to `.pending`
    /// if the stored raw value drifts.
    public var status: DashPayPaymentStatus {
        get { DashPayPaymentStatus(rawValue: statusRaw) ?? .pending }
        set { statusRaw = newValue.rawValue }
    }

    /// Transaction id (hex), the Rust `dashpay_payments` map key.
    /// Part of the compound unique key above.
    public var txid: String

    /// Sender memo, when present. `nil` mirrors the source `Option`
    /// being `None`.
    public var memo: String?

    // MARK: - Relationships

    /// Owning identity — the wallet-managed identity whose payment
    /// history this row belongs to. Non-optional: every payment row
    /// exists *because of* an owner identity. Cascade-deleted from
    /// `PersistentIdentity.dashpayPayments`.
    public var owner: PersistentIdentity

    // MARK: - Timestamps (local row bookkeeping, not payment dates)

    public var createdAt: Date
    public var lastUpdated: Date

    // MARK: - Initialization

    public init(
        owner: PersistentIdentity,
        counterpartyIdentityId: Data,
        amountDuffs: UInt64,
        direction: DashPayPaymentDirection,
        status: DashPayPaymentStatus,
        txid: String,
        memo: String? = nil
    ) {
        self.owner = owner
        self.networkRaw = owner.networkRaw
        self.ownerIdentityId = owner.identityId
        self.counterpartyIdentityId = counterpartyIdentityId
        self.amountDuffs = amountDuffs
        self.directionRaw = direction.rawValue
        self.statusRaw = status.rawValue
        self.txid = txid
        self.memo = memo
        self.createdAt = Date()
        self.lastUpdated = Date()
    }
}

// MARK: - Queries

extension PersistentDashpayPayment {
    /// Predicate filtering all payment rows that belong to a specific
    /// owner identity. Filters on the denormalized `ownerIdentityId`
    /// scalar so SwiftData's predicate engine doesn't have to traverse
    /// the `owner` relationship — same shape as the contact-request
    /// predicate.
    public static func predicate(
        ownerIdentityId: Data
    ) -> Predicate<PersistentDashpayPayment> {
        let target = ownerIdentityId
        return #Predicate<PersistentDashpayPayment> { row in
            row.ownerIdentityId == target
        }
    }

    /// Counterparty-scoped variant of [`predicate(ownerIdentityId:)`]
    /// — the payment list on a `ContactDetailView` shows only the
    /// history with that one contact.
    public static func predicate(
        ownerIdentityId: Data,
        counterpartyIdentityId: Data
    ) -> Predicate<PersistentDashpayPayment> {
        let target = ownerIdentityId
        let counterparty = counterpartyIdentityId
        return #Predicate<PersistentDashpayPayment> { row in
            row.ownerIdentityId == target
                && row.counterpartyIdentityId == counterparty
        }
    }
}
