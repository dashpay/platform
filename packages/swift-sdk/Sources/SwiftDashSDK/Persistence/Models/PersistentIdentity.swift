import Foundation
import SwiftData

/// SwiftData model for persisting Identity data
@Model
public final class PersistentIdentity {
    /// Index `networkRaw` so per-network scans (`#Predicate { $0.networkRaw == raw }`)
    /// don't degrade to a table scan. Every UI surface that lists
    /// identities filters by the active network.
    #Index<PersistentIdentity>([\.networkRaw])

    // MARK: - Core Properties
    @Attribute(.unique) public var identityId: Data
    public var balance: Int64
    public var revision: Int64
    /// `true` iff the user can act as this identity from this device.
    /// Two ways to be local:
    /// - wallet-owned: `wallet != nil`, keys re-derivable from the
    ///   wallet's DIP-9 tree (`wallet != nil` ⟹ `isLocal`);
    /// - imported key material with NO wallet: masternode/evonode
    ///   voting/owner/payout keys or pasted user private keys
    ///   (keychain-backed — see `LoadIdentityView`'s import flow).
    ///
    /// `false` for observed identities (DashPay contacts, DPNS
    /// lookups, payment counterparties).
    ///
    /// Writers promote, never demote: the persister sets `true` when
    /// it attaches the wallet relationship (and `loadWalletList()`
    /// heals wallet-linked stale-`false` rows at startup), import
    /// flows set `true` when key material lands. Nothing flips a
    /// `true` back — the flag outlives what any single writer can
    /// see. For strictly wallet-owned filtering use
    /// `walletOwnedIdentitiesPredicate`, which is narrower.
    public var isLocal: Bool
    public var alias: String?
    /// User's chosen primary display label (the one rendered on
    /// list rows and avatars). Populated only when the user selects a
    /// main name from `mainDpnsName` selection or as the fallback set
    /// during initial registration. The full label collection lives on
    /// the `dpnsNames` relationship below; this scalar is just the
    /// "show this one in the cell" hint.
    public var dpnsName: String?
    public var mainDpnsName: String?
    public var identityType: String

    // MARK: - Special Key Storage (stored in keychain)
    public var votingPrivateKeyIdentifier: String?
    public var ownerPrivateKeyIdentifier: String?
    public var payoutPrivateKeyIdentifier: String?

    // MARK: - Public Keys
    @Relationship(deleteRule: .cascade) public var publicKeys: [PersistentPublicKey]

    // MARK: - Timestamps
    public var createdAt: Date
    public var lastUpdated: Date
    public var lastSyncedAt: Date?

    // MARK: - Network
    /// Stored as the `Network.rawValue` `UInt32` so SwiftData
    /// `#Predicate` expressions can evaluate it directly. Foundation's
    /// predicate engine rejects captured non-primitive types — even
    /// Codable raw-value enums crash at evaluation with
    /// "Unsupported Predicate: Captured/constant values of type
    /// 'Network' are not supported". The `network` computed
    /// accessor below keeps the public API type-safe; only predicates
    /// that need to filter by network reach for `networkRaw`.
    public var networkRaw: UInt32

    /// Type-safe accessor over `networkRaw`. Reads fall back to
    /// `.testnet` if the stored raw value ever drifts out of the
    /// `Network` range (shouldn't happen — writers only go through
    /// this setter which uses `Network.rawValue`).
    public var network: Network {
        get { Network(rawValue: networkRaw) ?? .testnet }
        set { networkRaw = newValue.rawValue }
    }

    // MARK: - Wallet Association
    //
    // Cardinality: an identity belongs to 0 or 1 wallet. A wallet
    // holds N identities (see `PersistentWallet.identities`). When
    // the wallet is deleted, `wallet` nulls out (deleteRule:
    // `.nullify`) and the identity row survives orphaned.
    //
    // The `wallet` reference is the single source of truth — there
    // is no denormalized scalar `walletId`. Callers that want the
    // 32-byte wallet id read `identity.wallet?.walletId`;
    // predicates filter with `$0.wallet?.walletId == target`.
    // `@Relationship` is declared on the `PersistentWallet` side
    // (`identities`, with `inverse: \PersistentIdentity.wallet`),
    // so this is a plain stored property.
    public var wallet: PersistentWallet?
    /// DIP-9 identity index within the owning wallet. Mirrors the
    /// `identity_index` carried on `IdentityEntryFFI` from Rust.
    /// Only meaningful when `wallet != nil`; defaults to 0
    /// otherwise. Used to stable-sort identities within a wallet
    /// (e.g. when grouping public keys by identity).
    public var identityIndex: UInt32 = 0

    // MARK: - Relationships
    @Relationship(deleteRule: .cascade, inverse: \PersistentDocument.ownerIdentity) public var documents: [PersistentDocument]
    @Relationship(deleteRule: .nullify) public var tokenBalances: [PersistentTokenBalance]

    /// Confirmed DPNS labels observed for this identity. Cascade-deleted from
    /// the parent — losing the identity row drops the label cache and retained
    /// marketplace history too. A name that leaves this wallet remains related
    /// to its departed identity for history with
    /// `PersistentDPNSName.isOwned == false`. A transfer to another identity in
    /// the same wallet instead rebinds the schema's single unique-name row to
    /// the current owner. Owned-name surfaces use
    /// `PersistentDPNSName.predicate(identityId:)`.
    @Relationship(deleteRule: .cascade, inverse: \PersistentDPNSName.identity)
    public var dpnsNames: [PersistentDPNSName] = []

    /// DashPay profile cache for this identity — at most one row per
    /// (network, identity) per the contract's per-`ownerId`
    /// uniqueness on the `profile` document. Cascade-deleted from the
    /// parent. Optional because not every identity has published a
    /// profile (and the FFI changeset's `dashpay_profile: None`
    /// semantics mean "no update", not "delete" — the persister never
    /// nils this out from a flush). Inserted / refreshed by
    /// `PlatformWalletPersistenceHandler.upsertDashpayProfile(...)`.
    @Relationship(deleteRule: .cascade, inverse: \PersistentDashpayProfile.identity)
    public var dashpayProfile: PersistentDashpayProfile?

    /// DashPay contact-request rows owned by this identity (both
    /// outgoing and incoming). Cascade-deleted from the parent. Same
    /// query-by-denormalized-id pattern as `dpnsNames`: filters use
    /// `PersistentDashpayContactRequest.predicate(ownerIdentityId:)`
    /// rather than walking this collection from a SwiftUI view.
    /// Append / overwrite / delete on the write path: the persister
    /// callback applies upserts (per `(owner, contact, isOutgoing)`)
    /// and tombstones (`removed_sent` / `removed_incoming`) directly.
    @Relationship(deleteRule: .cascade, inverse: \PersistentDashpayContactRequest.owner)
    public var contactRequests: [PersistentDashpayContactRequest] = []

    /// DashPay payment-history rows owned by this identity.
    /// Cascade-deleted from the parent. Same
    /// query-by-denormalized-id pattern as `contactRequests`: filters
    /// use `PersistentDashpayPayment.predicate(ownerIdentityId:)`
    /// rather than walking this collection from a SwiftUI view.
    /// Populated by `PlatformWalletManager.refreshDashPayPayments`
    /// (FFI getter → upsert), not by the persister callback.
    @Relationship(deleteRule: .cascade, inverse: \PersistentDashpayPayment.owner)
    public var dashpayPayments: [PersistentDashpayPayment] = []

    /// DashPay ignored senders (per-sender mute, = block, reversible,
    /// local-only) owned by this identity. Cascade-deleted from the parent.
    /// Persisted from the `ignored` changeset array by `persistContacts`
    /// and read back at load to rebuild the Rust `ignored_senders` set —
    /// without them an ignored sender resurfaces on relaunch. Filters use
    /// `PersistentDashpayIgnoredSender.predicate(ownerIdentityId:)`.
    @Relationship(deleteRule: .cascade, inverse: \PersistentDashpayIgnoredSender.owner)
    public var dashpayIgnoredSenders: [PersistentDashpayIgnoredSender] = []

    /// Cached DashPay **contact** profiles owned by this identity (one
    /// per contact whose public profile has been fetched). Cascade-deleted
    /// from the parent. Same query-by-denormalized-id pattern as
    /// `contactRequests`: filters use
    /// `PersistentDashpayContactProfile.predicate(ownerIdentityId:)` rather
    /// than walking this collection from a SwiftUI view. Populated by the
    /// persister callback (`IdentityEntryFFI.contact_profiles` rows) and
    /// read back at load to rebuild the Rust `contact_profiles` map.
    /// Distinct from the owner's own `dashpayProfile`.
    @Relationship(deleteRule: .cascade, inverse: \PersistentDashpayContactProfile.owner)
    public var contactProfiles: [PersistentDashpayContactProfile] = []

    // Contracts in the local store that name this identity as their
    // owner. `.nullify` so deleting the identity leaves the contract
    // rows alive (with `ownerIdentity` nulled) — matches the user's
    // intent that contracts persist independently of whether the owner
    // identity happens to be loaded.
    // The `@Relationship` macro is declared on the contract side
    // (`PersistentDataContract.ownerIdentity`) so this is a plain
    // stored property — see `wallet` above for the same pattern.
    public var ownedDataContracts: [PersistentDataContract]

    // MARK: - Initialization
    public init(
        identityId: Data,
        balance: Int64 = 0,
        revision: Int64 = 0,
        isLocal: Bool = false,
        alias: String? = nil,
        dpnsName: String? = nil,
        mainDpnsName: String? = nil,
        identityType: IdentityType = .user,
        votingPrivateKeyIdentifier: String? = nil,
        ownerPrivateKeyIdentifier: String? = nil,
        payoutPrivateKeyIdentifier: String? = nil,
        network: Network,
        identityIndex: UInt32 = 0
    ) {
        self.identityId = identityId
        self.balance = balance
        self.revision = revision
        self.isLocal = isLocal
        self.alias = alias
        self.dpnsName = dpnsName
        self.mainDpnsName = mainDpnsName
        self.identityType = identityType.rawValue
        self.votingPrivateKeyIdentifier = votingPrivateKeyIdentifier
        self.ownerPrivateKeyIdentifier = ownerPrivateKeyIdentifier
        self.payoutPrivateKeyIdentifier = payoutPrivateKeyIdentifier
        self.networkRaw = network.rawValue
        self.identityIndex = identityIndex
        self.publicKeys = []
        self.documents = []
        self.tokenBalances = []
        self.dpnsNames = []
        self.dashpayProfile = nil
        self.contactRequests = []
        self.dashpayPayments = []
        self.dashpayIgnoredSenders = []
        self.contactProfiles = []
        self.ownedDataContracts = []
        self.createdAt = Date()
        self.lastUpdated = Date()
        self.lastSyncedAt = nil
    }

    // MARK: - Computed Properties
    public var identityIdString: String {
        identityId.toHexString()
    }

    public var identityIdBase58: String {
        identityId.toBase58String()
    }

    public var formattedBalance: String {
        let dashAmount = Double(balance) / 100_000_000_000
        return String(format: "%.8f DASH", dashAmount)
    }

    /// Projected DPP `IdentityPublicKey` view of `publicKeys`.
    /// Views that deal in DPP types (key signing, state
    /// transitions, crypto helpers) get their input here without
    /// having to thread `PersistentPublicKey` → DPP conversions
    /// themselves. Recomputed on each access — cheap.
    public var identityPublicKeys: [IdentityPublicKey] {
        publicKeys.compactMap { $0.toIdentityPublicKey() }
    }

    /// User-facing short name. Priority: `alias` → `mainDpnsName`
    /// → `dpnsName` → truncated hex id. Mirrors the old
    /// `IdentityModel.displayName` extension so views that read
    /// this don't change behavior post-migration.
    public var displayName: String {
        if let alias = alias, !alias.isEmpty {
            return alias
        }
        if let mainDpnsName = mainDpnsName, !mainDpnsName.isEmpty {
            return mainDpnsName
        }
        if let dpnsName = dpnsName, !dpnsName.isEmpty {
            return dpnsName
        }
        return String(identityIdString.prefix(12)) + "..."
    }

    public var identityTypeEnum: IdentityType {
        IdentityType(rawValue: identityType) ?? .user
    }

    // MARK: - Methods
    public func updateBalance(_ newBalance: Int64) {
        self.balance = newBalance
        self.lastUpdated = Date()
    }

    public func updateRevision(_ newRevision: Int64) {
        self.revision = newRevision
        self.lastUpdated = Date()
    }

    public func markAsSynced() {
        self.lastSyncedAt = Date()
    }

    public func updateDPNSName(_ name: String?) {
        self.dpnsName = name
        self.lastUpdated = Date()
    }

    public func addPublicKey(_ key: PersistentPublicKey) {
        publicKeys.append(key)
        lastUpdated = Date()
    }

    public func removePublicKey(withId keyId: Int32) {
        publicKeys.removeAll { $0.keyId == keyId }
        lastUpdated = Date()
    }

    /// Replace the persisted public-key rows with `newKeys`, carrying
    /// per-key provenance forward from the rows being replaced: the
    /// keychain identifier AND the wallet-derivation breadcrumb
    /// (`walletId` + `identityDerivationPath`) — the breadcrumb is the
    /// ownership evidence `hasWalletDerivationEvidence(for:)` (and
    /// therefore the restore quarantine and the startup heal) relies
    /// on. Refresh flows MUST use this instead of a bare
    /// `removeAll()` + re-append: dropping the stamps would strip a
    /// genuine wallet identity of its evidence and quarantine it from
    /// the next Rust restore.
    ///
    /// Provenance carries over only when the incoming key matches the
    /// outgoing one on BOTH `keyId` and `publicKeyData` — a breadcrumb
    /// (or keychain reference) describes exactly one key, and gluing
    /// it onto different key material would fabricate evidence.
    /// Incoming keys that already carry their own provenance keep it.
    public func replacePublicKeysPreservingProvenance(
        with newKeys: [PersistentPublicKey]
    ) {
        let outgoingByKeyId = Dictionary(
            publicKeys.map { ($0.keyId, $0) },
            uniquingKeysWith: { first, _ in first }
        )
        publicKeys.removeAll()
        for key in newKeys {
            if let outgoing = outgoingByKeyId[key.keyId],
               outgoing.publicKeyData == key.publicKeyData {
                if key.privateKeyKeychainIdentifier == nil {
                    key.privateKeyKeychainIdentifier = outgoing.privateKeyKeychainIdentifier
                }
                if key.walletId == nil {
                    key.walletId = outgoing.walletId
                    key.identityDerivationPath = outgoing.identityDerivationPath
                }
            }
            addPublicKey(key)
        }
    }

    /// Verified wallet-ownership evidence: at least one persisted
    /// key row carrying THIS wallet's derivation stamp
    /// (`PersistentPublicKey.walletId`), which `persistIdentityKeys`
    /// writes only for keys Rust derived from the wallet's DIP-9
    /// tree.
    ///
    /// Deliberately the ONLY accepted form. An UNSTAMPED key row is
    /// not evidence: `LoadIdentityView` and `IdentityKeyRefresher`
    /// persist unstamped rows for arbitrary (observed) identities, so
    /// a row the old unconditional fallback mislinked can carry them
    /// — and a stamp for a DIFFERENT wallet is evidence of the
    /// opposite. A genuinely-owned row that predates the breadcrumb
    /// columns regains its stamp through the Keychain breadcrumb
    /// backfill (`backfillIdentityKeyBreadcrumbs`) or its next owned
    /// re-emit (both write `walletId`), so treating it as unproven
    /// meanwhile is a recoverable deferral — restoring a mislink as
    /// wallet-owned is not.
    public func hasWalletDerivationEvidence(for walletId: Data) -> Bool {
        publicKeys.contains { $0.walletId == walletId }
    }

    /// Recompute `isLocal` after an explicit key-removal flow from
    /// what this row can still prove: wallet linkage or remaining
    /// imported key material. This is the ONE sanctioned demotion
    /// path — the persister and the startup heal never demote because
    /// they can't see keychain state, but a removal flow just changed
    /// that state and can. Without this, forgetting the last imported
    /// key leaves a walletless identity displaying "Local" (and
    /// passing `isLocal` action gates) while simultaneously showing
    /// "No Keys".
    ///
    /// Identifiers can outlive wiped Keychain items (same reason the
    /// identity list's "No Keys" badge probes the Keychain directly),
    /// so each stored reference is verified against LIVE Keychain
    /// state first and cleared when the backing item is gone — a
    /// stale identifier must not keep the row "Local".
    ///
    /// `@MainActor` because the Keychain probes are — matching the
    /// UI key-removal flows this is built for.
    @MainActor
    public func recomputeIsLocalAfterKeyRemoval() {
        let keychain = KeychainManager.shared
        for key in publicKeys where key.privateKeyKeychainIdentifier != nil {
            let alive = keychain.hasPrivateKey(
                identityId: identityId,
                keyIndex: key.keyId
            ) || keychain.hasIdentityPrivateKey(
                publicKeyHex: key.publicKeyData.toHexString()
            )
            if !alive {
                key.privateKeyKeychainIdentifier = nil
            }
        }
        if votingPrivateKeyIdentifier != nil,
           !keychain.hasSpecialKey(identityId: identityId, keyType: .voting) {
            votingPrivateKeyIdentifier = nil
        }
        if ownerPrivateKeyIdentifier != nil,
           !keychain.hasSpecialKey(identityId: identityId, keyType: .owner) {
            ownerPrivateKeyIdentifier = nil
        }
        if payoutPrivateKeyIdentifier != nil,
           !keychain.hasSpecialKey(identityId: identityId, keyType: .payout) {
            payoutPrivateKeyIdentifier = nil
        }
        // The wallet arm uses the SAME capability resolver as the
        // persister's promotion gate — current signing material, not
        // derivation evidence. A stamped key proves the wallet ONCE
        // derived this identity's keys (ownership), not that it can
        // sign today: a watch-only linked identity whose one imported
        // scalar is being forgotten must come out non-local even
        // though its stamps remain.
        var walletArm = false
        if let ownerWallet = wallet {
            switch WalletSigningCapability.probe(
                walletId: ownerWallet.walletId,
                verifiedBindingMarker: ownerWallet.seedBindingVerifiedMarker
            ) {
            case .some(true):
                walletArm = true
            case .some(false):
                walletArm = false
            case .none:
                // The Keychain couldn't answer (or the binding was
                // never verified) — every arm of this recompute
                // depends on Keychain truth, so deciding against
                // unknowns risks persisting a wrong classification.
                // Leave the flag as it stands; a later pass (or the
                // startup heal) decides.
                return
            }
        }
        isLocal = walletArm
            || publicKeys.contains { $0.privateKeyKeychainIdentifier != nil }
            || votingPrivateKeyIdentifier != nil
            || ownerPrivateKeyIdentifier != nil
            || payoutPrivateKeyIdentifier != nil
        lastUpdated = Date()
    }
}


// MARK: - Queries

extension PersistentIdentity {
    public static func predicate(identityId: Data) -> Predicate<PersistentIdentity> {
        #Predicate<PersistentIdentity> { identity in
            identity.identityId == identityId
        }
    }

    /// Identities owned by *some* wallet on this device — i.e. ones
    /// the persister attached to a `PersistentWallet` via the
    /// `wallet` relationship. Use this for views that should only
    /// surface identities the user can act as / sign for.
    ///
    /// Narrower than `isLocal`: wallet-owned identities are a subset
    /// of local ones (an identity with imported masternode/user keys
    /// is local but has no wallet row). Use this predicate when the
    /// operation needs the wallet's DIP-9 tree specifically (identity
    /// reload, DashPay), and `isLocal` when it merely needs signing
    /// capability. (Historical note: `isLocal` once read as a
    /// "Local Only vs On Network" badge, but no writer ever set it
    /// `true`, so a wallet's own identity persisted as `false`.)
    public static var walletOwnedIdentitiesPredicate: Predicate<PersistentIdentity> {
        #Predicate<PersistentIdentity> { identity in
            identity.wallet != nil
        }
    }

    public static func predicate(type: IdentityType) -> Predicate<PersistentIdentity> {
        let typeString = type.rawValue
        return #Predicate<PersistentIdentity> { identity in
            identity.identityType == typeString
        }
    }

    public static func needsSyncPredicate(olderThan date: Date) -> Predicate<PersistentIdentity> {
        #Predicate<PersistentIdentity> { identity in
            identity.lastSyncedAt == nil || identity.lastSyncedAt! < date
        }
    }

    public static func predicate(network: Network) -> Predicate<PersistentIdentity> {
        // Compare against the UInt32-backed `networkRaw` because Foundation's
        // predicate evaluator can't capture non-primitive types like
        // `Network` (the computed `network` accessor is invisible to
        // SwiftData — it can't see through `\.network.rawValue` either).
        let target = network.rawValue
        return #Predicate<PersistentIdentity> { identity in
            identity.networkRaw == target
        }
    }

    /// Network-scoped variant of [`walletOwnedIdentitiesPredicate`].
    /// Used by the recipient pickers, the "Acting as" picker, and any
    /// view that needs to restrict to identities the user controls on
    /// a specific network.
    public static func walletOwnedIdentitiesPredicate(network: Network) -> Predicate<PersistentIdentity> {
        let target = network.rawValue
        return #Predicate<PersistentIdentity> { identity in
            identity.wallet != nil && identity.networkRaw == target
        }
    }

    /// Fetch a single `PersistentIdentity` by its raw 32-byte id.
    /// Returns `nil` if the row doesn't exist or the fetch throws.
    public static func fetch(
        in context: ModelContext,
        identityId: Data
    ) -> PersistentIdentity? {
        let target = identityId
        let descriptor = FetchDescriptor<PersistentIdentity>(
            predicate: #Predicate { $0.identityId == target }
        )
        return try? context.fetch(descriptor).first
    }
}

// MARK: - Mutation helpers
//
// Deliberately small surface: only the fields views actually
// mutate from inside SwiftUI. Every helper fetches by identityId,
// applies the change, bumps `lastUpdated`, and leaves `save()` to
// the caller (or to the atomic-round bracket on the persister
// handler). `@discardableResult` on all of them because most
// call sites don't care whether the row existed.

extension PersistentIdentity {
    @discardableResult
    public static func updateBalance(
        in context: ModelContext,
        identityId: Data,
        balance: UInt64
    ) -> Bool {
        guard let row = fetch(in: context, identityId: identityId) else { return false }
        row.balance = Int64(bitPattern: balance)
        row.lastUpdated = Date()
        return true
    }

    @discardableResult
    public static func updateDpnsName(
        in context: ModelContext,
        identityId: Data,
        dpnsName: String?
    ) -> Bool {
        guard let row = fetch(in: context, identityId: identityId) else { return false }
        row.dpnsName = dpnsName
        row.lastUpdated = Date()
        return true
    }

    @discardableResult
    public static func updateMainDpnsName(
        in context: ModelContext,
        identityId: Data,
        mainDpnsName: String?
    ) -> Bool {
        guard let row = fetch(in: context, identityId: identityId) else { return false }
        row.mainDpnsName = mainDpnsName
        row.lastUpdated = Date()
        return true
    }

    @discardableResult
    public static func remove(
        in context: ModelContext,
        identityId: Data
    ) -> Bool {
        guard let row = fetch(in: context, identityId: identityId) else { return false }
        context.delete(row)
        return true
    }
}

// `PersistentIdentity` used to round-trip through the legacy
// `IdentityModel` value-type via `from(_:network:)` /
// `toIdentityModel()`. Both sides of that bridge are gone now —
// views read and mutate `PersistentIdentity` rows directly, and the
// DPP projection for key crypto lives under `identityPublicKeys`
// above.
