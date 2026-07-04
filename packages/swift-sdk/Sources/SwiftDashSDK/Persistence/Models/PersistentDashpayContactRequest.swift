import Foundation
import SwiftData

/// SwiftData row for one DashPay `contactRequest` document — the
/// directional, encrypted-key payload the sender publishes when
/// initiating contact with another identity. Populated by the
/// platform-wallet persister callback whenever a `ContactChangeSet`
/// rides on the FFI changeset (`on_persist_contacts_fn`).
///
/// One row per `(network, owner, contact, isOutgoing)` quad. The
/// outgoing and incoming directions for the same `(owner, contact)`
/// pair coexist as **distinct rows** because the encrypted payload
/// differs per direction (each side's `encryptedPublicKey` is sealed
/// to the other party's identity key), so the unique constraint
/// includes the direction bit.
///
/// Cascade-deleted from `PersistentIdentity.contactRequests` — losing
/// the owner identity drops every contact-request row that named it
/// as `owner`.
@Model
public final class PersistentDashpayContactRequest {
    /// Compound uniqueness on `(networkRaw, ownerIdentityId,
    /// contactIdentityId, isOutgoing)`. Mirrors the per-direction
    /// keying the Rust changeset uses on
    /// `ContactChangeSet::sent_requests` /
    /// `incoming_requests`, scoped by network so two networks don't
    /// collide in a shared local store.
    #Unique<PersistentDashpayContactRequest>([
        \.networkRaw, \.ownerIdentityId, \.contactIdentityId, \.isOutgoing
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
    /// through the optional `owner` join. Always equal to
    /// `owner.identityId` — kept in sync by the persister.
    public var ownerIdentityId: Data

    /// Other party's 32-byte identity id. For outgoing rows this is
    /// the recipient (`ContactRequest::recipient_id`); for incoming
    /// rows this is the sender (`ContactRequest::sender_id`). The
    /// `isOutgoing` bit disambiguates which direction this row
    /// represents.
    public var contactIdentityId: Data

    /// Direction bit. `true` ⇒ owner sent this request to contact;
    /// `false` ⇒ contact sent this request to owner. Same shape as
    /// the Rust `ContactRequestFFI::is_outgoing` field.
    public var isOutgoing: Bool

    // MARK: - Payload — round-trips `ContactRequest` verbatim

    /// `ContactRequest::sender_key_index` — index of the sender's
    /// identity public key used for the ECDH that encrypted the
    /// payload.
    public var senderKeyIndex: UInt32

    /// `ContactRequest::recipient_key_index`.
    public var recipientKeyIndex: UInt32

    /// `ContactRequest::account_reference` — DashPay account derivation
    /// hint the sender encoded in the request.
    public var accountReference: UInt32

    /// `ContactRequest::encrypted_public_key` bytes. Always non-empty
    /// — every contact-request document carries an encrypted key.
    public var encryptedPublicKey: Data

    /// `ContactRequest::encrypted_account_label` bytes, when present.
    /// `nil` mirrors the source `Option` being `None`.
    public var encryptedAccountLabel: Data?

    /// `ContactRequest::auto_accept_proof` bytes, when present. `nil`
    /// mirrors the source `Option` being `None`.
    public var autoAcceptProof: Data?

    /// `ContactRequest::core_height_created_at` — the Core block
    /// height at which the request landed on Platform.
    public var coreHeightCreatedAt: UInt32

    /// `ContactRequest::created_at` — Unix-millis timestamp the
    /// request document was created.
    public var createdAtMillis: UInt64

    /// Whether the established relationship this row belongs to has a
    /// **permanently broken** payment channel. Mirrors
    /// `ContactRequestFFI::payment_channel_broken`: only meaningful
    /// for rows projected from the `established` map — both
    /// directions of an established pair carry the same flag (it's a
    /// property of the relationship, not of one direction). Always
    /// `false` for pending rows. The UI reads it to disable "Send
    /// Dash" and surface "payment channel broken — ask the contact to
    /// send a new request".
    ///
    /// Defaulted so existing rows ride SwiftData's lightweight
    /// migration (additive column, non-destructive).
    public var paymentChannelBroken: Bool = false

    /// Owner-private alias for the contact — `contactInfo`-backed,
    /// synced across devices via Platform. Mirrors
    /// `ContactRequestFFI::alias`; established rows only, replicated
    /// onto both directions like `paymentChannelBroken`. Optional so
    /// existing rows ride the lightweight migration.
    public var contactAlias: String?

    /// Owner-private note — same conventions as `contactAlias`.
    public var contactNote: String?

    /// `contactInfo.displayHidden` — whether the owner hid this
    /// contact from the list. Defaulted for lightweight migration.
    public var contactHidden: Bool = false

    /// The contact's decrypted DIP-15 `encryptedAccountLabel` — the label
    /// the contact chose for the account they shared (a payment-routing
    /// hint, e.g. "Main wallet"). **System-derived and read-only**, unlike
    /// the owner-private `contactAlias`/`contactNote`: it is decrypted in
    /// Rust from the contact's incoming request, so it is populated only on
    /// the incoming-direction row (the outgoing row carries a label *we*
    /// sent, which is not surfaced). Optional so existing rows ride the
    /// lightweight migration.
    public var contactAccountLabel: String?

    /// `EstablishedContact::accepted_accounts` — the DIP-15
    /// rotated-account acceptances for this relationship. Mirrors
    /// `ContactRequestFFI::accepted_accounts`: a property of the
    /// relationship (not one direction), so it is replicated onto
    /// both directions like `paymentChannelBroken`; always empty for
    /// pending rows. Defaulted to an empty array so existing rows
    /// ride SwiftData's lightweight migration.
    public var contactAcceptedAccounts: [UInt32] = []

    // MARK: - Relationships

    /// Owning identity — the wallet-managed identity this row's
    /// `ownerIdentityId` denormalizes. Non-optional: every
    /// contact-request row exists *because of* an owner identity.
    /// Cascade-deleted from `PersistentIdentity.contactRequests`.
    public var owner: PersistentIdentity

    // MARK: - Timestamps

    public var createdAt: Date
    public var lastUpdated: Date

    // MARK: - Initialization

    public init(
        owner: PersistentIdentity,
        contactIdentityId: Data,
        isOutgoing: Bool,
        senderKeyIndex: UInt32,
        recipientKeyIndex: UInt32,
        accountReference: UInt32,
        encryptedPublicKey: Data,
        encryptedAccountLabel: Data? = nil,
        autoAcceptProof: Data? = nil,
        coreHeightCreatedAt: UInt32,
        createdAtMillis: UInt64,
        paymentChannelBroken: Bool = false
    ) {
        self.owner = owner
        self.networkRaw = owner.networkRaw
        self.ownerIdentityId = owner.identityId
        self.contactIdentityId = contactIdentityId
        self.isOutgoing = isOutgoing
        self.senderKeyIndex = senderKeyIndex
        self.recipientKeyIndex = recipientKeyIndex
        self.accountReference = accountReference
        self.encryptedPublicKey = encryptedPublicKey
        self.encryptedAccountLabel = encryptedAccountLabel
        self.autoAcceptProof = autoAcceptProof
        self.coreHeightCreatedAt = coreHeightCreatedAt
        self.createdAtMillis = createdAtMillis
        self.paymentChannelBroken = paymentChannelBroken
        self.createdAt = Date()
        self.lastUpdated = Date()
    }
}

// MARK: - Queries

extension PersistentDashpayContactRequest {
    /// Predicate filtering all contact-request rows that belong to a
    /// specific owner identity. Filters on the denormalized
    /// `ownerIdentityId` scalar so SwiftData's predicate engine
    /// doesn't have to traverse the `owner` relationship — same shape
    /// as the dpns-name predicate.
    public static func predicate(
        ownerIdentityId: Data
    ) -> Predicate<PersistentDashpayContactRequest> {
        let target = ownerIdentityId
        return #Predicate<PersistentDashpayContactRequest> { row in
            row.ownerIdentityId == target
        }
    }

    /// Direction-scoped variant of [`predicate(ownerIdentityId:)`].
    /// Used by the DashPay views that show only outgoing requests
    /// (sent), only incoming requests (received), or only established
    /// contacts (read both directions and join on `contactIdentityId`).
    public static func predicate(
        ownerIdentityId: Data,
        isOutgoing: Bool
    ) -> Predicate<PersistentDashpayContactRequest> {
        let target = ownerIdentityId
        let direction = isOutgoing
        return #Predicate<PersistentDashpayContactRequest> { row in
            row.ownerIdentityId == target && row.isOutgoing == direction
        }
    }
}
