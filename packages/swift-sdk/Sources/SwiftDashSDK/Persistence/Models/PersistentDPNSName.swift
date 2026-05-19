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
        self.createdAt = Date()
        self.lastUpdated = Date()
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
