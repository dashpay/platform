import Foundation
import SwiftData

/// SwiftData row for one DashPay **ignored sender** — a mirror of one
/// entry in the Rust-side `ManagedIdentity.ignored_senders` set, keyed by
/// the ignored sender's identity id.
///
/// ## Why this row exists
///
/// Ignore is a per-sender mute (= block, reversible, **local-only**): there
/// is no on-chain artifact (syncing it would leak who you ignored via the
/// public contact-request indices). `contactRequest` documents are
/// immutable and never deleted on-chain, so an ignored sender's requests
/// keep returning on every sync sweep. The Rust side suppresses re-ingest
/// via `is_sender_ignored`, but that set is in-memory: on relaunch it
/// starts empty, and without a persisted row to restore it from, the
/// ignored sender's requests **re-ingest and the sender resurfaces**. This
/// row is that persisted state — the load path rehydrates `ignored_senders`
/// from it (see the `ignored_senders` array on `IdentityRestoreEntryFFI`).
///
/// It is the SwiftData analog of the Rust-side ignored-senders store; the
/// example app uses the SwiftData persister, so it needs its own durable
/// ignore store.
///
/// ## Keying
///
/// Compound-unique on `(networkRaw, ownerIdentityId, ignoredSenderId)`.
/// Suppression is **per-sender** — bare sender id, no `accountReference`:
/// an ignored sender's requests are ALL suppressed (including rotated,
/// bumped-`accountReference` ones), matching the Rust set exactly. (This is
/// the deliberate difference from the old per-`(sender, accountReference)`
/// reject this replaces.)
///
/// Cascade-deleted from `PersistentIdentity.dashpayIgnoredSenders` —
/// losing the owner identity drops its ignored senders.
@Model
public final class PersistentDashpayIgnoredSender {
    /// Compound uniqueness on `(networkRaw, ownerIdentityId,
    /// ignoredSenderId)` — the Rust per-sender suppression key, scoped by
    /// network so two networks don't collide in a shared store.
    #Unique<PersistentDashpayIgnoredSender>([
        \.networkRaw, \.ownerIdentityId, \.ignoredSenderId
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
    /// ignored the sender. Denormalized so `#Predicate` filters match
    /// without a relationship traversal. Always equal to
    /// `owner.identityId`.
    public var ownerIdentityId: Data

    /// The 32-byte id of the ignored sender. The per-sender suppression
    /// key — no `accountReference`, so ALL of this sender's requests are
    /// suppressed.
    public var ignoredSenderId: Data

    // MARK: - Relationships

    /// Owning identity — the wallet-managed identity that ignored the
    /// sender. Non-optional: an ignore exists *because of* an owner
    /// identity. Cascade-deleted from
    /// `PersistentIdentity.dashpayIgnoredSenders`.
    public var owner: PersistentIdentity

    // MARK: - Timestamps (local row bookkeeping)

    public var ignoredAt: Date

    // MARK: - Initialization

    public init(
        owner: PersistentIdentity,
        ignoredSenderId: Data
    ) {
        self.owner = owner
        self.networkRaw = owner.networkRaw
        self.ownerIdentityId = owner.identityId
        self.ignoredSenderId = ignoredSenderId
        self.ignoredAt = Date()
    }
}

// MARK: - Queries

extension PersistentDashpayIgnoredSender {
    /// Predicate filtering all ignored-sender rows that belong to a
    /// specific owner identity. Filters on the denormalized
    /// `ownerIdentityId` scalar so SwiftData's predicate engine doesn't
    /// traverse the `owner` relationship — same shape as the
    /// contact-request and contact-profile predicates. Drives the
    /// "Ignored" screen's `@Query`.
    public static func predicate(
        ownerIdentityId: Data
    ) -> Predicate<PersistentDashpayIgnoredSender> {
        let target = ownerIdentityId
        return #Predicate<PersistentDashpayIgnoredSender> { row in
            row.ownerIdentityId == target
        }
    }

    /// Sender-scoped variant — fetch the one row for a single ignored
    /// sender of an owner (used by the persister's upsert/delete path).
    public static func predicate(
        ownerIdentityId: Data,
        ignoredSenderId: Data
    ) -> Predicate<PersistentDashpayIgnoredSender> {
        let target = ownerIdentityId
        let sender = ignoredSenderId
        return #Predicate<PersistentDashpayIgnoredSender> { row in
            row.ownerIdentityId == target
                && row.ignoredSenderId == sender
        }
    }
}
