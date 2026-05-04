import Foundation
import SwiftData

/// SwiftData row mirroring the DashPay `profile` document for one
/// `PersistentIdentity`. Populated by the platform-wallet persister
/// callback whenever an `IdentityEntry.dashpay_profile` rides on the
/// FFI changeset (`IdentityEntryFFI.dashpay_profile_present == true`).
///
/// One row per (network, identity): the DashPay contract enforces a
/// single `profile` document per `ownerId` (its only `unique` index),
/// so we mirror that with a compound uniqueness on `(networkRaw,
/// identity)` rather than rebinding to `identityId` directly — the
/// `identity` relationship is non-optional and the same uniqueness
/// shape as `PersistentDPNSName`.
///
/// Cascade-deleted from the parent `PersistentIdentity` via the
/// `dashpayProfile` relationship: dropping an identity drops its
/// profile cache.
@Model
public final class PersistentDashpayProfile {
    /// Compound uniqueness on `(networkRaw, identity)`. Mirrors the
    /// DashPay contract's per-`ownerId` uniqueness on the `profile`
    /// document, scoped by network so two networks don't collide in a
    /// shared local store.
    #Unique<PersistentDashpayProfile>([\.networkRaw, \.identity])

    /// Network discriminant. `UInt32` mirror of `Network.rawValue` —
    /// Foundation's predicate engine compares it directly without a
    /// custom converter. Stays in sync with `identity.networkRaw`
    /// (set by the init); identities don't migrate between networks.
    public var networkRaw: UInt32

    /// Type-safe accessor over `networkRaw`. Falls back to `.testnet`
    /// if the stored raw value drifts — matches
    /// `PersistentIdentity.network`.
    public var network: Network {
        get { Network(rawValue: networkRaw) ?? .testnet }
        set { networkRaw = newValue.rawValue }
    }

    // MARK: - Profile fields
    //
    // All optional — every `dashpay.profile` document field is
    // optional in the contract schema except the implicit
    // `$ownerId`. We mirror that on the row so partial profiles
    // (only an `avatarUrl` set, only a `displayName` set, etc.)
    // round-trip without forcing placeholder values.

    /// `displayName` field on the DashPay `profile` document. Up to
    /// 25 chars per the contract schema.
    public var displayName: String?

    /// `publicMessage` field on the DashPay `profile` document. Up to
    /// 140 chars per the contract schema.
    public var publicMessage: String?

    /// `bio` field. Not part of the v3 DashPay contract today; the
    /// FFI carries the slot for forwards-compat with future contract
    /// revisions and the column is reserved here so adding it doesn't
    /// trigger a destructive schema change.
    public var bio: String?

    /// `avatarUrl` field. URL string the consumer is expected to
    /// fetch + cache locally; the binary asset itself is never
    /// persisted on this row.
    public var avatarUrl: String?

    /// `avatarHash` field — 32-byte hash of the avatar binary,
    /// stored alongside the URL so consumers can verify the fetched
    /// asset matches what the profile author published. `nil` when
    /// the underlying `avatar_hash` was `None`.
    public var avatarHash: Data?

    /// `avatarFingerprint` field — 8-byte perceptual hash for
    /// quick equality checks on cached avatars without rehashing the
    /// full asset. `nil` when the underlying `avatar_fingerprint`
    /// was `None`.
    public var avatarFingerprint: Data?

    // MARK: - Relationships

    /// Owning identity. Non-optional — a profile only exists in the
    /// context of an identity. Cascade-deleted from the parent's
    /// `dashpayProfile` relationship; the persister wires this up at
    /// construction time.
    public var identity: PersistentIdentity

    // MARK: - Timestamps

    public var createdAt: Date
    public var lastUpdated: Date

    // MARK: - Initialization

    public init(
        identity: PersistentIdentity,
        displayName: String? = nil,
        publicMessage: String? = nil,
        bio: String? = nil,
        avatarUrl: String? = nil,
        avatarHash: Data? = nil,
        avatarFingerprint: Data? = nil
    ) {
        self.identity = identity
        self.networkRaw = identity.networkRaw
        self.displayName = displayName
        self.publicMessage = publicMessage
        self.bio = bio
        self.avatarUrl = avatarUrl
        self.avatarHash = avatarHash
        self.avatarFingerprint = avatarFingerprint
        self.createdAt = Date()
        self.lastUpdated = Date()
    }
}

// MARK: - Queries

extension PersistentDashpayProfile {
    /// Predicate filtering all rows that belong to a specific
    /// identity. Traverses the non-optional `identity` relationship
    /// to match its `identityId` — same shape as
    /// `PersistentDPNSName.predicate(identityId:)`.
    public static func predicate(identityId: Data) -> Predicate<PersistentDashpayProfile> {
        let target = identityId
        return #Predicate<PersistentDashpayProfile> { profile in
            profile.identity.identityId == target
        }
    }
}
