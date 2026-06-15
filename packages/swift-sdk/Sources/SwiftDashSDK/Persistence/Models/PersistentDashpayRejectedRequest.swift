import Foundation
import SwiftData

/// SwiftData row for one DashPay rejected-request tombstone (G5 stage 1)
/// — a mirror of one entry in the Rust-side
/// `ManagedIdentity.rejected_contact_requests` map, keyed by
/// `(sender_id, account_reference)`.
///
/// ## Why this row exists
///
/// `contactRequest` documents are immutable and never deleted on-chain,
/// so a rejected sender's request keeps returning on every sync sweep.
/// The Rust side suppresses re-ingest via `is_request_rejected`, but that
/// map is in-memory: on relaunch it starts empty, and without a persisted
/// tombstone to restore it from, the rejected request **re-ingests and the
/// contact resurrects**. This row is that persisted tombstone — the load
/// path rehydrates `rejected_contact_requests` from it (see the `rejected`
/// array on `IdentityRestoreEntryFFI`).
///
/// It is the SwiftData analog of the `rejected_contact_requests` table the
/// Rust-side SQLite persister keeps; the example app uses the SwiftData
/// persister, so it needs its own durable tombstone store.
///
/// ## Keying
///
/// Compound-unique on `(networkRaw, ownerIdentityId, senderIdentityId,
/// accountReference)`. The `accountReference` is part of the key on
/// purpose: a once-rejected sender CAN re-request via a bumped
/// `accountReference` (DIP-15 rotation), and that rotated request must
/// **not** be suppressed — mirrors the Rust suppression key exactly.
///
/// Cascade-deleted from `PersistentIdentity.dashpayRejectedRequests` —
/// losing the owner identity drops its tombstones.
@Model
public final class PersistentDashpayRejectedRequest {
    /// Compound uniqueness on `(networkRaw, ownerIdentityId,
    /// senderIdentityId, accountReference)` — the Rust suppression key,
    /// scoped by network so two networks don't collide in a shared store.
    #Unique<PersistentDashpayRejectedRequest>([
        \.networkRaw, \.ownerIdentityId, \.senderIdentityId, \.accountReference
    ])

    /// Network discriminant. `UInt32` mirror of `Network.rawValue`, kept
    /// in sync with `owner.networkRaw` by the init.
    public var networkRaw: UInt32

    /// Type-safe accessor over `networkRaw`. Falls back to `.testnet` if
    /// the stored raw value drifts.
    public var network: Network {
        get { Network(rawValue: networkRaw) ?? .testnet }
        set { networkRaw = newValue.rawValue }
    }

    /// Owning (wallet-managed) identity's 32-byte id — the recipient that
    /// rejected the request. Denormalized so `#Predicate` filters match
    /// without a relationship traversal. Always equal to
    /// `owner.identityId`.
    public var ownerIdentityId: Data

    /// The 32-byte id of the identity whose request was rejected (the
    /// sender). Part of the suppression key.
    public var senderIdentityId: Data

    /// The `accountReference` of the rejected request — part of the
    /// suppression key. A request from the same sender with a different
    /// `accountReference` (rotation) is NOT suppressed.
    public var accountReference: UInt32

    /// The rejected document's id, for audit / exact-match purposes only.
    /// `nil` mirrors the source `Option<Identifier>` being `None`. Not
    /// part of the suppression key.
    public var documentId: Data?

    // MARK: - Relationships

    /// Owning identity — the wallet-managed identity that rejected the
    /// request. Non-optional: a tombstone exists *because of* an owner
    /// identity. Cascade-deleted from
    /// `PersistentIdentity.dashpayRejectedRequests`.
    public var owner: PersistentIdentity

    // MARK: - Timestamps (local row bookkeeping)

    public var rejectedAt: Date

    // MARK: - Initialization

    public init(
        owner: PersistentIdentity,
        senderIdentityId: Data,
        accountReference: UInt32,
        documentId: Data? = nil
    ) {
        self.owner = owner
        self.networkRaw = owner.networkRaw
        self.ownerIdentityId = owner.identityId
        self.senderIdentityId = senderIdentityId
        self.accountReference = accountReference
        self.documentId = documentId
        self.rejectedAt = Date()
    }
}

// MARK: - Queries

extension PersistentDashpayRejectedRequest {
    /// Predicate filtering all tombstone rows that belong to a specific
    /// owner identity. Filters on the denormalized `ownerIdentityId`
    /// scalar so SwiftData's predicate engine doesn't traverse the
    /// `owner` relationship — same shape as the contact-request and
    /// payment predicates.
    public static func predicate(
        ownerIdentityId: Data
    ) -> Predicate<PersistentDashpayRejectedRequest> {
        let target = ownerIdentityId
        return #Predicate<PersistentDashpayRejectedRequest> { row in
            row.ownerIdentityId == target
        }
    }
}
