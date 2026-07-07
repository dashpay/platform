import Foundation
import SwiftData

/// SwiftData row for one cached DashPay **contact** profile — a mirror
/// of one entry in the Rust-side `contact_profiles` map on a
/// `ManagedIdentity` (keyed by the contact's identity id).
///
/// Distinct from `PersistentDashpayProfile`, which is the owner's *own*
/// profile (one per identity): this row is a *contact's* public profile,
/// cached so the requests / contacts UI can show a display name + avatar
/// without re-fetching on every launch. The cache is
/// relationship-independent — it serves established contacts, pending
/// incoming-request senders, and (later) ignored senders from one table,
/// matching the Rust map. It holds **only the five public profile
/// fields** parsed from the on-chain `profile` document; it must never
/// receive anything derived from the encrypted `contactInfo` path.
///
/// One row per `(network, owner, contact)` — the Rust map is keyed by
/// the contact's identity id per owner, scoped by network so two
/// networks don't collide in a shared local store.
///
/// Populated by the platform-wallet persister callback whenever an
/// `IdentityEntry.contact_profiles` entry rides on the FFI changeset.
/// A present profile (`ContactProfileRowFFI.is_present == true`) upserts
/// this row; a confirmed-absent entry (`is_present == false`) DELETEs it,
/// so a contact who removed their on-chain profile can't leave a stale
/// name/avatar behind. Read back at load to rebuild the Rust
/// `contact_profiles` map (present entries only — the negative cache
/// re-derives on the first sweep) so the cache survives relaunch instead
/// of refetching every contact.
///
/// Cascade-deleted from `PersistentIdentity.contactProfiles` — losing
/// the owner identity drops its cached contact profiles.
@Model
public final class PersistentDashpayContactProfile {
    /// Compound uniqueness on `(networkRaw, ownerIdentityId,
    /// contactIdentityId)`. Mirrors the per-owner, per-contact keying of
    /// the Rust `contact_profiles` map.
    #Unique<PersistentDashpayContactProfile>([
        \.networkRaw, \.ownerIdentityId, \.contactIdentityId
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
    /// kept in sync by the persister.
    public var ownerIdentityId: Data

    /// The contact's 32-byte identity id — the `contact_profiles` map
    /// key. Part of the compound unique key above.
    public var contactIdentityId: Data

    // MARK: - Profile fields
    //
    // All optional — every `dashpay.profile` document field is optional
    // in the contract schema except the implicit `$ownerId`. We mirror
    // that so partial profiles (only an `avatarUrl` set, only a
    // `displayName` set, etc.) round-trip without forcing placeholders.

    /// `displayName` field on the contact's DashPay `profile` document.
    public var displayName: String?

    /// `publicMessage` field on the contact's `profile` document.
    public var publicMessage: String?

    /// `bio` field. Carried for forwards-compat with future contract
    /// revisions; reserved here so adding it later doesn't trigger a
    /// destructive schema change.
    public var bio: String?

    /// `avatarUrl` field — URL the consumer fetches + caches locally.
    /// The binary asset itself is never persisted. Treated as untrusted
    /// (attacker-controlled public data): the Rust side caches and
    /// restores it only when it is a bounded `https://` URL.
    public var avatarUrl: String?

    /// `avatarHash` field — 32-byte hash of the avatar binary, so
    /// consumers can verify a fetched asset matches what the contact
    /// published. `nil` when the underlying `avatar_hash` was absent.
    public var avatarHash: Data?

    /// `avatarFingerprint` field — 8-byte perceptual hash for quick
    /// equality checks on cached avatars. `nil` when absent.
    public var avatarFingerprint: Data?

    /// Wall-clock ms of the last fetch attempt on the Rust side
    /// (`ContactProfileEntry.checked_at_ms`) — drives the self-heal
    /// backoff. Round-tripped verbatim so the restored cache keeps the
    /// same re-query schedule it had before relaunch. Stored as the
    /// scalar so the predicate engine compares it directly.
    public var checkedAtMs: UInt64

    // MARK: - Relationships

    /// Owning identity — the wallet-managed identity whose cached
    /// contact profiles this row belongs to. Non-optional: every contact
    /// profile exists *because of* an owner identity. Cascade-deleted
    /// from `PersistentIdentity.contactProfiles`.
    public var owner: PersistentIdentity

    // MARK: - Timestamps (local row bookkeeping)

    public var createdAt: Date
    public var lastUpdated: Date

    // MARK: - Initialization

    public init(
        owner: PersistentIdentity,
        contactIdentityId: Data,
        checkedAtMs: UInt64,
        displayName: String? = nil,
        publicMessage: String? = nil,
        bio: String? = nil,
        avatarUrl: String? = nil,
        avatarHash: Data? = nil,
        avatarFingerprint: Data? = nil
    ) {
        self.owner = owner
        self.networkRaw = owner.networkRaw
        self.ownerIdentityId = owner.identityId
        self.contactIdentityId = contactIdentityId
        self.checkedAtMs = checkedAtMs
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

extension PersistentDashpayContactProfile {
    /// Predicate filtering all cached contact-profile rows that belong
    /// to a specific owner identity. Filters on the denormalized
    /// `ownerIdentityId` scalar so SwiftData's predicate engine doesn't
    /// traverse the `owner` relationship — same shape as the
    /// contact-request / payment predicates.
    public static func predicate(
        ownerIdentityId: Data
    ) -> Predicate<PersistentDashpayContactProfile> {
        let target = ownerIdentityId
        return #Predicate<PersistentDashpayContactProfile> { row in
            row.ownerIdentityId == target
        }
    }

    /// Contact-scoped variant of [`predicate(ownerIdentityId:)`] — fetch
    /// the one cached profile for a single contact of an owner.
    public static func predicate(
        ownerIdentityId: Data,
        contactIdentityId: Data
    ) -> Predicate<PersistentDashpayContactProfile> {
        let target = ownerIdentityId
        let contact = contactIdentityId
        return #Predicate<PersistentDashpayContactProfile> { row in
            row.ownerIdentityId == target
                && row.contactIdentityId == contact
        }
    }
}
