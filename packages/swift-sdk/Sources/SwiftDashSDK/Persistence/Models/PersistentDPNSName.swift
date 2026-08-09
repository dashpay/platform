import Foundation
import SwiftData

/// SwiftData row for one confirmed DPNS label owned by a
/// `PersistentIdentity`. Mirrors a single
/// `platform_wallet::DpnsNameInfo` after it travels across the FFI on
/// `IdentityEntryFFI.dpns_names` / `dpns_names_acquired_at`.
///
/// Why a dedicated model and not a `[String]` column: identities can
/// hold multiple DPNS labels and SwiftUI views want to observe the
/// list reactively — `@Query` over a row collection beats a `[String]`
/// column that views can only read in bulk on `onAppear`.
///
/// This is purely a label cache. The DPNS document's `normalizedLabel`
/// (homograph-safe form used for the uniqueness lookup) is NOT
/// persisted here — DPNS lookups go through the SDK / platform-wallet,
/// and the local cache only needs to render the display label.
@Model
public final class PersistentDPNSName {
    /// Compound uniqueness on `(networkRaw, normalizedParentDomainName,
    /// normalizedLabel)`. Mirrors the DPNS contract's `domain`
    /// document index `parentNameAndLabel`
    /// (`normalizedParentDomainName + normalizedLabel`, `unique: true`)
    /// and adds the network scope so two networks don't collide in a
    /// shared local store. A label is only unique within a domain
    /// on a given chain.
    #Unique<PersistentDPNSName>([\.networkRaw, \.normalizedParentDomainName, \.normalizedLabel])

    /// Network discriminant. `UInt32` mirror of `Network.rawValue` —
    /// Foundation's predicate engine compares it directly without a
    /// custom converter. Stays in sync with `identity.networkRaw`
    /// via the init; identities don't migrate between networks.
    public var networkRaw: UInt32

    /// Type-safe accessor over `networkRaw`. Falls back to `.testnet`
    /// if the stored raw value drifts — matches
    /// `PersistentIdentity.network`.
    public var network: Network {
        get { Network(rawValue: networkRaw) ?? .testnet }
        set { networkRaw = newValue.rawValue }
    }

    /// Display label — the original case-and-letters form the user
    /// registered, e.g. "Alice". Maps to the DPNS document's
    /// `label` property.
    public var label: String

    /// Homograph-safe lowercase form of `label` used for lookups
    /// (e.g. "Alice" → "a11ce"; `o`/`O`→`0`, `i`/`I`→`1`,
    /// `l`/`L`→`1`, everything else lowercased). Maps to the DPNS
    /// document's `normalizedLabel` property and participates in the
    /// per-domain uniqueness above. Computed once on insert from
    /// `label` via `Self.normalize(_:)`.
    public var normalizedLabel: String

    /// Display parent domain — e.g. "dash". Maps to the DPNS
    /// document's `parentDomainName` property. DPNS today only
    /// supports the single top-level domain "dash", so the persister
    /// stamps that as the default; the field exists so subdomain
    /// support (when/if DPNS gains it) lands without a schema bump.
    public var parentDomainName: String

    /// Homograph-safe form of `parentDomainName` used for lookups.
    /// Maps to the DPNS document's `normalizedParentDomainName`
    /// property and participates in the per-domain uniqueness above.
    public var normalizedParentDomainName: String

    /// Unix-millis timestamp when the wallet first observed this
    /// label belonging to the identity. Mirrors
    /// `DpnsNameInfo.acquired_at`. `0` when unknown.
    public var acquiredAt: UInt64

    // MARK: - Username marketplace
    //
    // Fed by the `on_persist_dpns_name_states_fn` persister callback
    // (`DpnsNameStateFFI`), NOT by the identity label snapshot that
    // populates the fields above. All of them are optional or defaulted
    // so an existing store migrates in place (SwiftData lightweight
    // migration).
    //
    // READ CONTRACT: every field in this section is meaningful only
    // while `documentIdBase58` is non-nil. A nil document id means the
    // wallet is not tracking this name's marketplace state — it does NOT
    // mean the name is owned and unlisted. Gate any marketplace UI on
    // `documentIdBase58 != nil` before reading `saleStatus` or
    // `priceCredits`.

    /// Base58 id of the DPNS `domain` document behind this label — the
    /// handle every trade transition needs, stable across transfers and
    /// purchases. `nil` while no marketplace state has been mirrored (or
    /// after the row was dropped from marketplace tracking).
    public var documentIdBase58: String?

    /// Listed sale price in **credits** (1 duff = 1000 credits), stored
    /// as `Int64(bitPattern:)` like `PersistentIdentity.balance` because
    /// SwiftData has no unsigned 64-bit column. `nil` = the name is not
    /// listed for sale, which is distinct from a 0-credit listing.
    public var priceCredits: Int64?

    /// Raw ``DpnsNameSaleStatus`` discriminant: 0 = owned, 1 = sold,
    /// 2 = transferred. Defaults to 0 so existing rows migrate, so read
    /// it through ``saleStatus`` rather than directly.
    public var saleStatusRaw: Int16

    /// Base58 id of the counterparty a departed name went to — the buyer
    /// when `saleStatusRaw == 1`, the recipient when it is 2. `nil` while
    /// the name is still owned (or the counterparty is unknown).
    public var counterpartyIdBase58: String?

    /// Unix-millis timestamp of the sync pass / confirmed transition
    /// that last wrote the marketplace fields. `0` = never written.
    public var marketplaceUpdatedAt: UInt64

    // MARK: - Relationships

    /// Owning identity. Cascade-deleted from the parent — losing the
    /// identity row should drop its label cache too. The `inverse`
    /// declaration on `PersistentIdentity.dpnsNames` is the source of
    /// truth for this association.
    ///
    /// Non-optional: every DPNS-label row exists *because* of an
    /// identity. The persister wires it at construction time
    /// (before insert) so SwiftData's non-optional relationship
    /// contract is honored.
    public var identity: PersistentIdentity

    // MARK: - Timestamps

    public var createdAt: Date
    public var lastUpdated: Date

    // MARK: - Initialization

    public init(
        identity: PersistentIdentity,
        label: String,
        parentDomainName: String = "dash",
        acquiredAt: UInt64 = 0
    ) {
        self.identity = identity
        self.networkRaw = identity.networkRaw
        self.label = label
        self.normalizedLabel = Self.normalize(label)
        self.parentDomainName = parentDomainName
        self.normalizedParentDomainName = Self.normalize(parentDomainName)
        self.acquiredAt = acquiredAt
        // A freshly inserted row carries no marketplace state until the
        // marketplace persister callback writes it — hence a nil document
        // id, which is the "not tracked" signal the read contract above
        // documents.
        self.documentIdBase58 = nil
        self.priceCredits = nil
        self.saleStatusRaw = 0
        self.counterpartyIdBase58 = nil
        self.marketplaceUpdatedAt = 0
        self.createdAt = Date()
        self.lastUpdated = Date()
    }
}

// MARK: - Marketplace accessors

extension PersistentDPNSName {
    /// Typed view of the marketplace columns as the SDK's
    /// ``DpnsNameSaleStatus``, or `nil` when this row carries no
    /// trustworthy marketplace state.
    ///
    /// Prefer this over reading `saleStatusRaw` directly: it enforces the
    /// read contract, so an untracked row (`documentIdBase58 == nil`) can
    /// never be mistaken for an owned-and-unlisted one. It also returns
    /// `nil` for a departed row whose counterparty id is missing or
    /// undecodable — the wallet always attaches one for a sale or a
    /// transfer, so its absence means the row is unreliable, not that the
    /// name went nowhere.
    ///
    /// An unrecognized discriminant is likewise `nil`, never `.owned`: if
    /// Rust's `DpnsNameSaleStatus` gains a variant, an older Swift build
    /// must report the row as unreadable rather than claim a departed
    /// name is still owned.
    public var saleStatus: DpnsNameSaleStatus? {
        guard documentIdBase58 != nil else { return nil }
        switch saleStatusRaw {
        case 0:
            return .owned
        case 1:
            guard let to = counterpartyId else { return nil }
            return .sold(to: to)
        case 2:
            guard let to = counterpartyId else { return nil }
            return .transferred(to: to)
        default:
            return nil
        }
    }

    /// The departed name's counterparty as a 32-byte identifier, decoded
    /// from `counterpartyIdBase58`. `nil` while the name is still owned,
    /// or if the stored string doesn't decode.
    public var counterpartyId: Data? {
        counterpartyIdBase58.flatMap { Data.identifier(fromBase58: $0) }
    }

    /// Listed sale price in credits, or `nil` when the name is not
    /// listed (or carries no mirrored marketplace state at all).
    public var listedPriceCredits: UInt64? {
        guard documentIdBase58 != nil, let priceCredits else { return nil }
        return UInt64(bitPattern: priceCredits)
    }
}

// MARK: - Normalization

extension PersistentDPNSName {
    /// Homograph-safe lowercasing identical to the DPNS contract's
    /// `normalizedLabel` rule (and to
    /// `dash_sdk::platform::dpns_usernames::convert_to_homograph_safe_chars`):
    /// `o`/`O`→`0`, `i`/`I`→`1`, `l`/`L`→`1`, every other character
    /// ASCII-lowercased. Run on label and parent at insert time so the
    /// persisted row matches what the platform stores in the DPNS
    /// document. We mirror the rule on the Swift side (rather than
    /// routing the bare label through `dash_sdk_dpns_normalize_username`)
    /// to avoid an FFI hop per row — the rule is closed-form and
    /// stable across releases.
    public static func normalize(_ input: String) -> String {
        String(input.map { c -> Character in
            switch c {
            case "o", "O": return "0"
            case "i", "I": return "1"
            case "l", "L": return "1"
            default: return Character(c.lowercased())
            }
        })
    }
}

// MARK: - Queries

extension PersistentDPNSName {
    /// Predicate filtering all DPNS-label rows that belong to a
    /// specific identity. Traverses the `identity` relationship to
    /// match its `identityId` — safe because the relationship is
    /// non-optional and SwiftData's predicate engine handles
    /// non-optional one-hop traversal cleanly.
    public static func predicate(identityId: Data) -> Predicate<PersistentDPNSName> {
        let target = identityId
        return #Predicate<PersistentDPNSName> { name in
            name.identity.identityId == target
        }
    }
}
