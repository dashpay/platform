import Foundation
import SwiftData

// MARK: - Frozen model definitions for already-released schema versions
//
// A `VersionedSchema` identifies a store by the CHECKSUM of the entities it
// declares, not by the identity of the Swift type list. So a registered
// schema version may only ever reference model types whose *shape* is frozen
// at the moment that version shipped. Pointing `DashSchemaVN.models` at a
// live `@Model` type means the next property added to that type silently
// mutates version N's checksum in place: a store written by the previously
// released binary then matches no schema in `DashMigrationPlan.schemas`, and
// `ModelContainer(for:migrationPlan:configurations:)` fails to open it with
// Cocoa error 134504 ("Cannot use staged migration with an unknown model
// version") instead of migrating it.
//
// This file holds the frozen copies. A frozen copy must be a *nested* type,
// because SwiftData derives the entity name from the unqualified type name —
// `DashSchemaV1.PersistentAssetLock` and the top-level `PersistentAssetLock`
// are two distinct Swift types that both describe the entity named
// "PersistentAssetLock", which is exactly what lets a migration stage map one
// onto the other. (`DashModelMigrationTests` asserts that entity naming, so a
// future SwiftData change to the derivation would fail loudly rather than
// silently renaming an entity.)
//
// ## Scope of the freeze
//
// Only `PersistentAssetLock` is frozen today, because it is the only model
// this file's callers have changed since V2 shipped. Every other model in
// `DashSchemaV1` / `DashSchemaV2` is still referenced live and therefore
// still carries the same latent defect. Freezing them is a mechanical but
// wide change (34 models) and is deliberately left out of the change that
// introduced this file; when the next model gains a property, freeze that
// one here too and add the matching stage.
//
// V1's own checksum has already drifted from what actually shipped as V1
// (see the `DashSchemaV1` doc comment: several models were changed in place
// while V1 was the only registered version, and dev stores at V1 are
// knowingly expected to fail open and be rebuilt). The frozen copy below is
// therefore the shape as of the V2 release, shared by V1 and V2 — which is
// what makes V1 -> V2 continue to be "add `PersistentTrackedMasternode`"
// and nothing else, exactly as before.

extension DashSchemaV1 {
    /// `PersistentAssetLock` frozen at the shape it had when schema V2
    /// shipped — i.e. everything the live model has today EXCEPT
    /// `recipientIsExternal`, which is what V3 adds.
    ///
    /// Referenced by both `DashSchemaV1.models` and `DashSchemaV2.models`.
    /// Do not add properties here and do not "fix" its doc comments to
    /// match the live model: every attribute, its optionality, its default
    /// value, the `@Attribute(.unique)` marker and the `#Index` are all
    /// inputs to the V2 checksum, and changing any of them re-breaks the
    /// V2 stores this type exists to keep openable. Doc comments are not
    /// inputs to the checksum, but keeping them minimal here keeps the
    /// live model the single place worth reading.
    ///
    /// See the live ``SwiftDashSDK/PersistentAssetLock`` for what each
    /// column means.
    @Model
    final class PersistentAssetLock {
        #Index<PersistentAssetLock>([\.walletId])

        @Attribute(.unique) var outPointHex: String
        var walletId: Data
        var transactionBytes: Data
        var fundingTypeRaw: Int
        var identityIndexRaw: Int32
        var accountIndexRaw: Int32 = 0
        var amountDuffs: Int64
        var statusRaw: Int
        var proofBytes: Data?
        var recipientPlatformAddressHash: Data?
        var recipientPlatformAddressType: UInt8?
        var createdAt: Date
        var updatedAt: Date

        init(
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
}

// MARK: - The rest of the relationship component, frozen at the V3 shape
//
// The four models the sweep persistence changes — `PersistentTransaction`,
// `PersistentTxo`, `PersistentPendingInput` and `PersistentWallet` — each
// gain a property, so each needs a frozen copy for the same reason
// `PersistentAssetLock` did. Freezing them alone is not possible: a frozen
// model must declare its relationships against frozen counterparts (an
// `inverse:` key path is typed on the destination model), and following
// those relationships in both directions closes over 24 of the 35 models.
// Registering a frozen copy beside a live one for the SAME entity name is
// what the schema cannot express, so the whole component travels together.
//
// These copies are the shape as of V3 — i.e. everything the live models had
// before the sweep columns — and are shared by V1, V2 and V3, none of which
// changed any model in this component. The eleven models outside the
// component (shielded storage, invitations, masternodes, the asset-lock
// pair above, wallet-manager metadata) are still referenced live and still
// carry the latent defect this file exists to fix; freezing them is the
// same mechanical exercise, for whichever change next touches one.
//
// Do not edit these copies to match the live models. Every attribute, its
// optionality, its default, each `@Attribute` marker, each `#Index` and
// each relationship is an input to the V1/V2/V3 checksums, and changing one
// re-breaks the stores these types exist to keep openable.

extension DashSchemaV1 {
    @Model
    final class PersistentAccount {
        /// Compound uniqueness on the full account-identity tuple:
        /// `(wallet, accountType, accountIndex, standardTag,
        /// registrationIndex, keyClass, userIdentityId,
        /// friendIdentityId)`. Mirrors the persister's match logic
        /// exactly — the variant disambiguators (`standardTag` for
        /// BIP44 vs BIP32, `registrationIndex` for top-ups, `keyClass`
        /// for PlatformPayment) are part of the key so legitimate
        /// sibling accounts can coexist (e.g. BIP44 #0 and BIP32 #0,
        /// or multiple top-up accounts on the same identity).
        #Unique<PersistentAccount>([
            \.wallet,
            \.accountType,
            \.accountIndex,
            \.standardTag,
            \.registrationIndex,
            \.keyClass,
            \.userIdentityId,
            \.friendIdentityId,
        ])

        /// Account type identifier — matches the `AccountTypeTagFFI`
        /// discriminant from the Rust side (0 = Standard, 1 = CoinJoin,
        /// … 14 = PlatformPayment, 15 = IdentityAuthenticationEcdsa,
        /// 16 = IdentityAuthenticationBls). Stable across releases.
        var accountType: UInt32
        /// Account index within the type (for indexed account types). For
        /// `PlatformPayment` this is the `account` field; for
        /// `DashpayReceivingFunds` / `DashpayExternalAccount` it's the
        /// account-level selector; for
        /// `IdentityAuthentication{Ecdsa,Bls}` it's the identity index.
        var accountIndex: UInt32
        /// Human-readable account type name.
        var accountTypeName: String
        /// Per-account confirmed balance in duffs.
        var balanceConfirmed: UInt64
        /// Per-account unconfirmed balance in duffs.
        var balanceUnconfirmed: UInt64
        /// External address pool: highest used index (-1 = none).
        var externalHighestUsed: Int32
        /// Internal (change) address pool: highest used index.
        var internalHighestUsed: Int32
        /// `StandardAccountTypeTagFFI` value. Meaningful only when
        /// `accountType == 0` (Standard): 0 = BIP44, 1 = BIP32.
        var standardTag: UInt8
        /// `IdentityTopUp.registration_index`. Zero for other variants.
        var registrationIndex: UInt32
        /// `PlatformPayment.key_class`. Zero for other variants.
        var keyClass: UInt32
        /// `Dashpay*`.user_identity_id (32 bytes). Empty `Data` for other
        /// variants.
        var userIdentityId: Data
        /// `Dashpay*`.friend_identity_id (32 bytes). Empty `Data` for
        /// other variants.
        var friendIdentityId: Data
        /// Bincode-encoded extended public key for this account. For ECDSA
        /// accounts it's an `ExtendedPubKey`; for the two provider
        /// key-material accounts (`accountType == 10` operator = BLS,
        /// `accountType == 11` platform node = Ed25519) it's the extended
        /// BLS / Ed25519 public key instead. Populated by
        /// `on_persist_account_registrations_fn`, consumed by
        /// `on_load_wallet_list_fn` to reconstruct a watch-only account
        /// (`Account::from_xpub` for ECDSA, `BLSAccount`/`EdDSAAccount` for
        /// the provider accounts). `nil` means "not yet persisted" —
        /// account cannot be restored silently. Unique because two
        /// accounts can't legitimately share an xpub (would imply a key
        /// reuse / derivation collision); SQL UNIQUE allows multiple
        /// `nil` values, so freshly-inserted unhydrated rows don't
        /// conflict.
        @Attribute(.unique) var accountExtendedPubKeyBytes: Data?
        /// Record timestamps.
        var createdAt: Date
        var lastUpdated: Date

        /// Parent wallet. Every account currently belongs to a wallet. If
        /// standalone non-wallet accounts are introduced later, this
        /// becomes optional again.
        ///
        /// Kept non-optional. SwiftData would otherwise fatal during
        /// the `save()` phase of a wallet delete
        /// (`Cannot remove PersistentWallet from relationship wallet on
        /// PersistentAccount because an appropriate default value is
        /// not configured`); the workaround is in
        /// `PlatformWalletPersistenceHandler.deleteWalletData`, which
        /// deletes all of the wallet's accounts in a separate
        /// `save()` BEFORE deleting the wallet itself. By the time the
        /// wallet row is deleted, its `accounts` collection is empty
        /// and SwiftData has no inverse to null out. This costs
        /// atomicity (two saves instead of one) — acceptable for a
        /// user-initiated wipe.
        var wallet: PersistentWallet

        /// Addresses from this account's address pools (external +
        /// internal, or a single Absent pool for degenerate types). Holds
        /// Core-chain (base58check) addresses only — PlatformPayment
        /// accounts keep their addresses in `platformAddresses`.
        /// Per-account TXOs flow through this collection
        /// (`coreAddresses.flatMap(\.txos)`).
        @Relationship(deleteRule: .cascade, inverse: \PersistentCoreAddress.account)
        var coreAddresses: [PersistentCoreAddress]

        /// DIP-17 Platform Payment addresses for this account, keyed on
        /// DIP-0018 bech32m encoding. Populated only when
        /// `accountType == 14` (PlatformPayment).
        @Relationship(deleteRule: .cascade, inverse: \PersistentPlatformAddress.account)
        var platformAddresses: [PersistentPlatformAddress]

        /// Transactions this account participates in that the TXO graph
        /// cannot recover — the payload-only involvement described in the
        /// type doc above. Populated by the persistence handler, which
        /// appends this account whenever it upserts a tx record the
        /// changeset bucketed under this account, even when the record
        /// produced no TXO here (special-tx payloads matching provider
        /// owner / voting key addresses).
        ///
        /// A superset that overlaps the TXO-derived set for ordinary funded
        /// txs (the handler appends there too), so consumers computing a
        /// per-account transaction list must **union** this with the
        /// TXO-derived txids and de-dup — see `AccountDetailView`.
        ///
        /// The `inverse:` for this many-to-many lives on
        /// `PersistentTransaction.involvedAccounts`; this side carries the
        /// plain declaration. Default `.nullify` delete rule — deleting
        /// this account detaches it from each tx without removing the
        /// (shared) tx rows. That matters for the wallet-wipe path
        /// (`deleteWalletData`), which deletes accounts before the wallet:
        /// `.nullify` on a to-many inverse has no "default value" fatal
        /// (unlike the non-optional `wallet` back-reference), so no extra
        /// pre-delete pass is needed.
        var involvedTransactions: [PersistentTransaction] = []

        init(
            wallet: PersistentWallet,
            accountType: UInt32,
            accountIndex: UInt32,
            accountTypeName: String
        ) {
            self.wallet = wallet
            self.accountType = accountType
            self.accountIndex = accountIndex
            self.accountTypeName = accountTypeName
            self.balanceConfirmed = 0
            self.balanceUnconfirmed = 0
            self.externalHighestUsed = -1
            self.internalHighestUsed = -1
            self.standardTag = 0
            self.registrationIndex = 0
            self.keyClass = 0
            self.userIdentityId = Data()
            self.friendIdentityId = Data()
            self.accountExtendedPubKeyBytes = nil
            self.createdAt = Date()
            self.lastUpdated = Date()
            self.coreAddresses = []
            self.platformAddresses = []
            self.involvedTransactions = []
        }
    }

    @Model
    final class PersistentCoreAddress {
        /// Base58check-encoded address. Unique across the SwiftData store
        /// because the same address can't validly exist under two accounts
        /// (collision would imply a wallet-id hash collision).
        @Attribute(.unique) var address: String
        /// Typed public key bytes, or empty Data when the Rust side couldn't
        /// produce one (e.g. a pool entry that stored only a script). The
        /// curve is given by `keyType`: 33-byte compressed secp256k1 (ECDSA),
        /// 48-byte BLS operator key, or 32-byte Ed25519 platform-node key.
        var publicKey: Data
        /// `KeyTypeTagFFI` raw value identifying the curve of `publicKey`:
        /// 0 ECDSA / 1 BLS / 2 EdDSA. Meaningful only when `publicKey` is
        /// non-empty. The stored default (NOT just the init-parameter
        /// default, which SwiftData migration never consults) keeps
        /// pre-column stores openable: without it, lightweight migration
        /// fails with "missing attribute values on mandatory destination
        /// attribute" and the container refuses to load — a launch crash on
        /// every device that has existing rows. Defaulted legacy rows read
        /// as ECDSA with an empty `publicKey` until the next Rust
        /// address-pool persist pulse (pool extension / address-used /
        /// registration — NOT plain load, which only reads the snapshot)
        /// re-emits them with typed keys. On load, Rust's
        /// `restore_address_pool` keeps the pre-derived typed key when a
        /// legacy row arrives key-less, so in-memory BLS operator matching
        /// is unaffected; legacy Ed25519 platform-node keys are hardened-only
        /// and re-derivable only via delete+re-import (pre-release
        /// convention).
        var keyType: UInt8 = 0
        /// `AddressPoolTypeTagFFI` raw value — 0 External, 1 Internal,
        /// 2 Absent, 3 AbsentHardened.
        var poolTypeTag: UInt8
        /// Derivation index within this pool.
        var addressIndex: UInt32
        /// BIP32 derivation path (e.g. `"m/44'/1'/0'/0/3"`).
        var derivationPath: String
        /// Marked used by the Rust address pool (first-seen tx or explicit
        /// `mark_used`).
        var isUsed: Bool
        /// SPV height where this address first appeared in a transaction.
        /// Zero until the address is seen on-chain.
        var firstSeenHeight: UInt32
        /// SPV height of the most recent transaction touching this address.
        var lastSeenHeight: UInt32
        /// Cached balance in duffs from `AddressInfo.balance`. Updated by
        /// subsequent `on_persist_account_address_pools_fn` pulses.
        var balance: UInt64
        /// Record timestamps.
        var createdAt: Date
        var lastUpdated: Date

        /// Parent account.
        var account: PersistentAccount?

        /// TXOs paid to this address. Cascade-delete: dropping the
        /// address row takes its TXOs with it. The address is the
        /// canonical owning record — no meaningful render path for an
        /// address-less TXO. Pool rebuilds therefore need to reuse
        /// existing rows (the persister upserts by Base58Check string,
        /// which it already does) rather than wholesale-replace, or
        /// the historical TXO chain gets wiped.
        @Relationship(deleteRule: .cascade, inverse: \PersistentTxo.coreAddress)
        var txos: [PersistentTxo] = []

        init(
            address: String,
            publicKey: Data = Data(),
            keyType: UInt8 = 0,
            poolTypeTag: UInt8,
            addressIndex: UInt32,
            derivationPath: String,
            isUsed: Bool = false,
            balance: UInt64 = 0
        ) {
            self.address = address
            self.publicKey = publicKey
            self.keyType = keyType
            self.poolTypeTag = poolTypeTag
            self.addressIndex = addressIndex
            self.derivationPath = derivationPath
            self.isUsed = isUsed
            self.firstSeenHeight = 0
            self.lastSeenHeight = 0
            self.balance = balance
            self.createdAt = Date()
            self.lastUpdated = Date()
        }
    }

    @Model
    final class PersistentDPNSName {
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
        var networkRaw: UInt32

        /// Type-safe accessor over `networkRaw`. Falls back to `.testnet`
        /// if the stored raw value drifts — matches
        /// `PersistentIdentity.network`.
        var network: Network {
            get { Network(rawValue: networkRaw) ?? .testnet }
            set { networkRaw = newValue.rawValue }
        }

        /// Display label — the original case-and-letters form the user
        /// registered, e.g. "Alice". Maps to the DPNS document's
        /// `label` property.
        var label: String

        /// Homograph-safe lowercase form of `label` used for lookups
        /// (e.g. "Alice" → "a11ce"; `o`/`O`→`0`, `i`/`I`→`1`,
        /// `l`/`L`→`1`, everything else lowercased). Maps to the DPNS
        /// document's `normalizedLabel` property and participates in the
        /// per-domain uniqueness above. Computed once on insert from
        /// `label` via `Self.normalize(_:)`.
        var normalizedLabel: String

        /// Display parent domain — e.g. "dash". Maps to the DPNS
        /// document's `parentDomainName` property. DPNS today only
        /// supports the single top-level domain "dash", so the persister
        /// stamps that as the default; the field exists so subdomain
        /// support (when/if DPNS gains it) lands without a schema bump.
        var parentDomainName: String

        /// Homograph-safe form of `parentDomainName` used for lookups.
        /// Maps to the DPNS document's `normalizedParentDomainName`
        /// property and participates in the per-domain uniqueness above.
        var normalizedParentDomainName: String

        /// Unix-millis timestamp when the wallet first observed this
        /// label belonging to the identity. Mirrors
        /// `DpnsNameInfo.acquired_at`. `0` when unknown.
        var acquiredAt: UInt64

        /// Whether the latest canonical identity snapshot still includes this
        /// name. Marketplace callbacks never overwrite this value. A name that
        /// leaves the wallet keeps its row on the departed identity with `false`;
        /// a same-wallet transfer rebinds the unique row to the current identity
        /// with `true`.
        var isOwned: Bool = true

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
        var documentIdBase58: String?

        /// Listed sale price in **credits** (1 duff = 1000 credits), stored
        /// as `Int64(bitPattern:)` like `PersistentIdentity.balance` because
        /// SwiftData has no unsigned 64-bit column. `nil` = the name is not
        /// listed for sale, which is distinct from a 0-credit listing.
        var priceCredits: Int64?

        /// Raw ``DpnsNameSaleStatus`` discriminant: 0 = owned, 1 = sold,
        /// 2 = transferred. Defaults to 0 so existing rows migrate, so read
        /// it through ``saleStatus`` rather than directly.
        var saleStatusRaw: Int16 = 0

        /// Base58 id of the counterparty a departed name went to — the buyer
        /// when `saleStatusRaw == 1`, the recipient when it is 2. `nil` while
        /// the name is still owned (or the counterparty is unknown).
        var counterpartyIdBase58: String?

        /// Domain document `$createdAt` in Unix milliseconds. `nil` when
        /// Platform did not carry the timestamp.
        var documentCreatedAtMs: UInt64?

        /// Domain document `$updatedAt` in Unix milliseconds. `nil` when
        /// Platform did not carry the timestamp.
        var documentUpdatedAtMs: UInt64?

        /// Domain document `$transferredAt` in Unix milliseconds. `nil` when
        /// Platform did not carry the timestamp.
        var documentTransferredAtMs: UInt64?

        /// Unix-millis timestamp of the sync pass / confirmed transition
        /// that last wrote the marketplace fields. `0` = never written.
        var marketplaceUpdatedAt: UInt64 = 0

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
        var identity: PersistentIdentity

        // MARK: - Timestamps

        var createdAt: Date
        var lastUpdated: Date

        // MARK: - Initialization

        init(
            identity: PersistentIdentity,
            label: String,
            parentDomainName: String = "dash",
            acquiredAt: UInt64 = 0,
            isOwned: Bool = true
        ) {
            self.identity = identity
            self.networkRaw = identity.networkRaw
            self.label = label
            self.normalizedLabel = label.lowercased()
            self.parentDomainName = parentDomainName
            self.normalizedParentDomainName = parentDomainName.lowercased()
            self.acquiredAt = acquiredAt
            self.isOwned = isOwned
            // A freshly inserted row carries no marketplace state until the
            // marketplace persister callback writes it — hence a nil document
            // id, which is the "not tracked" signal the read contract above
            // documents.
            self.documentIdBase58 = nil
            self.priceCredits = nil
            self.saleStatusRaw = 0
            self.counterpartyIdBase58 = nil
            self.documentCreatedAtMs = nil
            self.documentUpdatedAtMs = nil
            self.documentTransferredAtMs = nil
            self.marketplaceUpdatedAt = 0
            self.createdAt = Date()
            self.lastUpdated = Date()
        }
    }

    @Model
    final class PersistentDashpayContactProfile {
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
        var networkRaw: UInt32

        /// Type-safe accessor over `networkRaw`. Falls back to `.testnet`
        /// if the stored raw value drifts.
        var network: Network {
            get { Network(rawValue: networkRaw) ?? .testnet }
            set { networkRaw = newValue.rawValue }
        }

        /// Owning (wallet-managed) identity's 32-byte id, denormalized so
        /// `#Predicate` filters can match without a relationship traversal
        /// through the `owner` join. Always equal to `owner.identityId` —
        /// kept in sync by the persister.
        var ownerIdentityId: Data

        /// The contact's 32-byte identity id — the `contact_profiles` map
        /// key. Part of the compound unique key above.
        var contactIdentityId: Data

        // MARK: - Profile fields
        //
        // All optional — every `dashpay.profile` document field is optional
        // in the contract schema except the implicit `$ownerId`. We mirror
        // that so partial profiles (only an `avatarUrl` set, only a
        // `displayName` set, etc.) round-trip without forcing placeholders.

        /// `displayName` field on the contact's DashPay `profile` document.
        var displayName: String?

        /// `publicMessage` field on the contact's `profile` document.
        var publicMessage: String?

        /// `bio` field. Carried for forwards-compat with future contract
        /// revisions; reserved here so adding it later doesn't trigger a
        /// destructive schema change.
        var bio: String?

        /// `avatarUrl` field — URL the consumer fetches + caches locally.
        /// The binary asset itself is never persisted. Treated as untrusted
        /// (attacker-controlled public data): the Rust side caches and
        /// restores it only when it is a bounded `https://` URL.
        var avatarUrl: String?

        /// `avatarHash` field — 32-byte hash of the avatar binary, so
        /// consumers can verify a fetched asset matches what the contact
        /// published. `nil` when the underlying `avatar_hash` was absent.
        var avatarHash: Data?

        /// `avatarFingerprint` field — 8-byte perceptual hash for quick
        /// equality checks on cached avatars. `nil` when absent.
        var avatarFingerprint: Data?

        /// Wall-clock ms of the last fetch attempt on the Rust side
        /// (`ContactProfileEntry.checked_at_ms`) — drives the self-heal
        /// backoff. Round-tripped verbatim so the restored cache keeps the
        /// same re-query schedule it had before relaunch. Stored as the
        /// scalar so the predicate engine compares it directly.
        var checkedAtMs: UInt64

        // MARK: - Relationships

        /// Owning identity — the wallet-managed identity whose cached
        /// contact profiles this row belongs to. Non-optional: every contact
        /// profile exists *because of* an owner identity. Cascade-deleted
        /// from `PersistentIdentity.contactProfiles`.
        var owner: PersistentIdentity

        // MARK: - Timestamps (local row bookkeeping)

        var createdAt: Date
        var lastUpdated: Date

        // MARK: - Initialization

        init(
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

    @Model
    final class PersistentDashpayContactRequest {
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
        var networkRaw: UInt32

        /// Type-safe accessor over `networkRaw`. Falls back to `.testnet`
        /// if the stored raw value drifts.
        var network: Network {
            get { Network(rawValue: networkRaw) ?? .testnet }
            set { networkRaw = newValue.rawValue }
        }

        /// Owning (wallet-managed) identity's 32-byte id, denormalized so
        /// `#Predicate` filters can match without a relationship traversal
        /// through the optional `owner` join. Always equal to
        /// `owner.identityId` — kept in sync by the persister.
        var ownerIdentityId: Data

        /// Other party's 32-byte identity id. For outgoing rows this is
        /// the recipient (`ContactRequest::recipient_id`); for incoming
        /// rows this is the sender (`ContactRequest::sender_id`). The
        /// `isOutgoing` bit disambiguates which direction this row
        /// represents.
        var contactIdentityId: Data

        /// Direction bit. `true` ⇒ owner sent this request to contact;
        /// `false` ⇒ contact sent this request to owner. Same shape as
        /// the Rust `ContactRequestFFI::is_outgoing` field.
        var isOutgoing: Bool

        // MARK: - Payload — round-trips `ContactRequest` verbatim

        /// `ContactRequest::sender_key_index` — index of the sender's
        /// identity public key used for the ECDH that encrypted the
        /// payload.
        var senderKeyIndex: UInt32

        /// `ContactRequest::recipient_key_index`.
        var recipientKeyIndex: UInt32

        /// `ContactRequest::account_reference` — DashPay account derivation
        /// hint the sender encoded in the request.
        var accountReference: UInt32

        /// `ContactRequest::encrypted_public_key` bytes. Always non-empty
        /// — every contact-request document carries an encrypted key.
        var encryptedPublicKey: Data

        /// `ContactRequest::encrypted_account_label` bytes, when present.
        /// `nil` mirrors the source `Option` being `None`.
        var encryptedAccountLabel: Data?

        /// `ContactRequest::auto_accept_proof` bytes, when present. `nil`
        /// mirrors the source `Option` being `None`.
        var autoAcceptProof: Data?

        /// `ContactRequest::core_height_created_at` — the Core block
        /// height at which the request landed on Platform.
        var coreHeightCreatedAt: UInt32

        /// `ContactRequest::created_at` — Unix-millis timestamp the
        /// request document was created.
        var createdAtMillis: UInt64

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
        var paymentChannelBroken: Bool = false

        /// Owner-private alias for the contact — `contactInfo`-backed,
        /// synced across devices via Platform. Mirrors
        /// `ContactRequestFFI::alias`; established rows only, replicated
        /// onto both directions like `paymentChannelBroken`. Optional so
        /// existing rows ride the lightweight migration.
        var contactAlias: String?

        /// Owner-private note — same conventions as `contactAlias`.
        var contactNote: String?

        /// `contactInfo.displayHidden` — whether the owner hid this
        /// contact from the list. Defaulted for lightweight migration.
        var contactHidden: Bool = false

        /// The contact's decrypted DIP-15 `encryptedAccountLabel` — the label
        /// the contact chose for the account they shared (a payment-routing
        /// hint, e.g. "Main wallet"). **System-derived and read-only**, unlike
        /// the owner-private `contactAlias`/`contactNote`: it is decrypted in
        /// Rust from the contact's incoming request, so it is populated only on
        /// the incoming-direction row (the outgoing row carries a label *we*
        /// sent, which is not surfaced). Optional so existing rows ride the
        /// lightweight migration.
        var contactAccountLabel: String?

        /// `EstablishedContact::accepted_accounts` — the DIP-15
        /// rotated-account acceptances for this relationship. Mirrors
        /// `ContactRequestFFI::accepted_accounts`: a property of the
        /// relationship (not one direction), so it is replicated onto
        /// both directions like `paymentChannelBroken`; always empty for
        /// pending rows. Defaulted to an empty array so existing rows
        /// ride SwiftData's lightweight migration.
        var contactAcceptedAccounts: [UInt32] = []

        // MARK: - Relationships

        /// Owning identity — the wallet-managed identity this row's
        /// `ownerIdentityId` denormalizes. Non-optional: every
        /// contact-request row exists *because of* an owner identity.
        /// Cascade-deleted from `PersistentIdentity.contactRequests`.
        var owner: PersistentIdentity

        // MARK: - Timestamps

        var createdAt: Date
        var lastUpdated: Date

        // MARK: - Initialization

        init(
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

    @Model
    final class PersistentDashpayIgnoredSender {
        /// Compound uniqueness on `(networkRaw, ownerIdentityId,
        /// ignoredSenderId)` — the Rust per-sender suppression key, scoped by
        /// network so two networks don't collide in a shared store.
        #Unique<PersistentDashpayIgnoredSender>([
            \.networkRaw, \.ownerIdentityId, \.ignoredSenderId
        ])

        /// Network discriminant. `UInt32` mirror of `Network.rawValue`, kept
        /// in sync with `owner.networkRaw` by the init.
        var networkRaw: UInt32

        /// Type-safe accessor over `networkRaw`. Falls back to `.testnet` if
        /// the stored raw value drifts.
        var network: Network {
            get { Network(rawValue: networkRaw) ?? .testnet }
            set { networkRaw = newValue.rawValue }
        }

        /// Owning (wallet-managed) identity's 32-byte id — the recipient that
        /// ignored the sender. Denormalized so `#Predicate` filters match
        /// without a relationship traversal. Always equal to
        /// `owner.identityId`.
        var ownerIdentityId: Data

        /// The 32-byte id of the ignored sender. The per-sender suppression
        /// key — no `accountReference`, so ALL of this sender's requests are
        /// suppressed.
        var ignoredSenderId: Data

        // MARK: - Relationships

        /// Owning identity — the wallet-managed identity that ignored the
        /// sender. Non-optional: an ignore exists *because of* an owner
        /// identity. Cascade-deleted from
        /// `PersistentIdentity.dashpayIgnoredSenders`.
        var owner: PersistentIdentity

        // MARK: - Timestamps (local row bookkeeping)

        var ignoredAt: Date

        // MARK: - Initialization

        init(
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

    @Model
    final class PersistentDashpayPayment {
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
        var networkRaw: UInt32

        /// Type-safe accessor over `networkRaw`. Falls back to `.testnet`
        /// if the stored raw value drifts.
        var network: Network {
            get { Network(rawValue: networkRaw) ?? .testnet }
            set { networkRaw = newValue.rawValue }
        }

        /// Owning (wallet-managed) identity's 32-byte id, denormalized so
        /// `#Predicate` filters can match without a relationship traversal
        /// through the `owner` join. Always equal to `owner.identityId` —
        /// kept in sync by the refresh path.
        var ownerIdentityId: Data

        /// The other identity in this payment
        /// (`DashpayPaymentFFI::counterparty_id`). Whether they are the
        /// sender or the receiver is encoded in `directionRaw`.
        var counterpartyIdentityId: Data

        /// Amount in duffs. Always positive; `directionRaw` carries the
        /// sign.
        var amountDuffs: UInt64

        /// Raw `DashPayPaymentDirection` value. Stored as the scalar so
        /// the predicate engine compares it directly.
        var directionRaw: UInt8

        /// Type-safe accessor over `directionRaw`. Falls back to `.sent`
        /// if the stored raw value drifts.
        var direction: DashPayPaymentDirection {
            get { DashPayPaymentDirection(rawValue: directionRaw) ?? .sent }
            set { directionRaw = newValue.rawValue }
        }

        /// Raw `DashPayPaymentStatus` value.
        var statusRaw: UInt8

        /// Type-safe accessor over `statusRaw`. Falls back to `.pending`
        /// if the stored raw value drifts.
        var status: DashPayPaymentStatus {
            get { DashPayPaymentStatus(rawValue: statusRaw) ?? .pending }
            set { statusRaw = newValue.rawValue }
        }

        /// Transaction id (hex), the Rust `dashpay_payments` map key.
        /// Part of the compound unique key above.
        var txid: String

        /// Sender memo, when present. `nil` mirrors the source `Option`
        /// being `None`.
        var memo: String?

        // MARK: - Relationships

        /// Owning identity — the wallet-managed identity whose payment
        /// history this row belongs to. Non-optional: every payment row
        /// exists *because of* an owner identity. Cascade-deleted from
        /// `PersistentIdentity.dashpayPayments`.
        var owner: PersistentIdentity

        // MARK: - Timestamps (local row bookkeeping, not payment dates)

        var createdAt: Date
        var lastUpdated: Date

        // MARK: - Initialization

        init(
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

    @Model
    final class PersistentDashpayProfile {
        /// Compound uniqueness on `(networkRaw, identity)`. Mirrors the
        /// DashPay contract's per-`ownerId` uniqueness on the `profile`
        /// document, scoped by network so two networks don't collide in a
        /// shared local store.
        #Unique<PersistentDashpayProfile>([\.networkRaw, \.identity])

        /// Network discriminant. `UInt32` mirror of `Network.rawValue` —
        /// Foundation's predicate engine compares it directly without a
        /// custom converter. Stays in sync with `identity.networkRaw`
        /// (set by the init); identities don't migrate between networks.
        var networkRaw: UInt32

        /// Type-safe accessor over `networkRaw`. Falls back to `.testnet`
        /// if the stored raw value drifts — matches
        /// `PersistentIdentity.network`.
        var network: Network {
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
        var displayName: String?

        /// `publicMessage` field on the DashPay `profile` document. Up to
        /// 140 chars per the contract schema.
        var publicMessage: String?

        /// `bio` field. Not part of the v3 DashPay contract today; the
        /// FFI carries the slot for forwards-compat with future contract
        /// revisions and the column is reserved here so adding it doesn't
        /// trigger a destructive schema change.
        var bio: String?

        /// `avatarUrl` field. URL string the consumer is expected to
        /// fetch + cache locally; the binary asset itself is never
        /// persisted on this row.
        var avatarUrl: String?

        /// `avatarHash` field — 32-byte hash of the avatar binary,
        /// stored alongside the URL so consumers can verify the fetched
        /// asset matches what the profile author published. `nil` when
        /// the underlying `avatar_hash` was `None`.
        var avatarHash: Data?

        /// `avatarFingerprint` field — 8-byte perceptual hash for
        /// quick equality checks on cached avatars without rehashing the
        /// full asset. `nil` when the underlying `avatar_fingerprint`
        /// was `None`.
        var avatarFingerprint: Data?

        // MARK: - Relationships

        /// Owning identity. Non-optional — a profile only exists in the
        /// context of an identity. Cascade-deleted from the parent's
        /// `dashpayProfile` relationship; the persister wires this up at
        /// construction time.
        var identity: PersistentIdentity

        // MARK: - Timestamps

        var createdAt: Date
        var lastUpdated: Date

        // MARK: - Initialization

        init(
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

    @Model
    final class PersistentDataContract {
        /// Index `networkRaw` so the static `predicate(networkRaw:)` and
        /// `tokensPredicate(networkRaw:)` helpers — plus every per-network
        /// list view — can index-scan instead of table-scan.
        #Index<PersistentDataContract>([\.networkRaw])

        @Attribute(.unique) var id: Data
        var name: String
        var serializedContract: Data
        var createdAt: Date
        var lastAccessedAt: Date

        // Binary serialization (CBOR format)
        var binarySerialization: Data?

        // Version info
        var version: Int?
        var ownerId: Data?

        // Keywords and description
        @Relationship(deleteRule: .cascade, inverse: \PersistentKeyword.dataContract)
        var keywordRelations: [PersistentKeyword]
        var contractDescription: String?

        // Schema and document types storage
        var schemaData: Data
        var documentTypesData: Data

        // Groups
        var groupsData: Data?

        // Network
        /// Stored as the `Network.rawValue` `UInt32` so SwiftData
        /// `#Predicate` expressions can evaluate it directly. See
        /// `PersistentIdentity.networkRaw` for the full rationale.
        var networkRaw: UInt32

        /// Type-safe accessor over `networkRaw`. Setter writes through.
        var network: Network {
            get { Network(rawValue: networkRaw) ?? .testnet }
            set { networkRaw = newValue.rawValue }
        }

        // Timestamps
        var lastUpdated: Date
        var lastSyncedAt: Date?

        // Contract configuration
        var canBeDeleted: Bool
        var readonly: Bool
        var keepsHistory: Bool
        var schemaDefs: Int?

        // Document defaults
        var documentsKeepHistoryContractDefault: Bool
        var documentsMutableContractDefault: Bool
        var documentsCanBeDeletedContractDefault: Bool

        // Relationships with cascade delete
        @Relationship(deleteRule: .cascade, inverse: \PersistentToken.dataContract)
        var tokens: [PersistentToken]?

        @Relationship(deleteRule: .cascade, inverse: \PersistentDocumentType.dataContract)
        var documentTypes: [PersistentDocumentType]?

        @Relationship(deleteRule: .cascade, inverse: \PersistentDocument.dataContract)
        var documents: [PersistentDocument]

        // Owner identity — populated when the owner happens to also live in
        // the local store. May be nil even when `ownerId` is set, because
        // most contracts in the local cache will be owned by identities the
        // user doesn't hold. Back-filled lazily by
        // `ContractIdentityLinker.linkContractToOwner` when either side is
        // inserted.
        @Relationship(deleteRule: .nullify, inverse: \PersistentIdentity.ownedDataContracts)
        var ownerIdentity: PersistentIdentity?

        // Token support tracking
        var hasTokens: Bool
        var tokensData: Data?

        // Computed properties
        var idBase58: String {
            id.toBase58String()
        }

        var ownerIdBase58: String? {
            ownerId?.toBase58String()
        }

        var parsedContract: [String: Any]? {
            try? JSONSerialization.jsonObject(with: serializedContract, options: []) as? [String: Any]
        }

        var binarySerializationHex: String? {
            binarySerialization?.toHexString()
        }

        var keywords: [String] {
            keywordRelations.map { $0.keyword }
        }

        var schema: [String: Any] {
            get {
                guard let json = try? JSONSerialization.jsonObject(with: schemaData),
                      let dict = json as? [String: Any] else {
                    return [:]
                }
                return dict
            }
            set {
                schemaData = (try? JSONSerialization.data(withJSONObject: newValue)) ?? Data()
                lastUpdated = Date()
            }
        }

        var documentTypesList: [String] {
            get {
                guard let json = try? JSONSerialization.jsonObject(with: documentTypesData),
                      let array = json as? [String] else {
                    return []
                }
                return array
            }
            set {
                documentTypesData = (try? JSONSerialization.data(withJSONObject: newValue)) ?? Data()
                lastUpdated = Date()
            }
        }

        var tokenConfigurations: [String: Any]? {
            get {
                guard let data = tokensData,
                      let json = try? JSONSerialization.jsonObject(with: data),
                      let dict = json as? [String: Any] else {
                    return nil
                }
                return dict
            }
            set {
                if let newValue = newValue {
                    tokensData = try? JSONSerialization.data(withJSONObject: newValue)
                    hasTokens = true
                } else {
                    tokensData = nil
                    hasTokens = false
                }
                lastUpdated = Date()
            }
        }

        var groups: [String: Any]? {
            get {
                guard let data = groupsData,
                      let json = try? JSONSerialization.jsonObject(with: data),
                      let dict = json as? [String: Any] else {
                    return nil
                }
                return dict
            }
            set {
                if let newValue = newValue {
                    groupsData = try? JSONSerialization.data(withJSONObject: newValue)
                } else {
                    groupsData = nil
                }
                lastUpdated = Date()
            }
        }

        init(
            id: Data,
            name: String,
            serializedContract: Data,
            version: Int? = 1,
            ownerId: Data? = nil,
            schema: [String: Any] = [:],
            documentTypesList: [String] = [],
            keywords: [String] = [],
            description: String? = nil,
            hasTokens: Bool = false,
            network: Network
        ) {
            self.id = id
            self.name = name
            self.serializedContract = serializedContract
            self.createdAt = Date()
            self.lastAccessedAt = Date()
            self.version = version
            self.ownerId = ownerId

            // Schema and document types
            self.schemaData = (try? JSONSerialization.data(withJSONObject: schema)) ?? Data()
            self.documentTypesData = (try? JSONSerialization.data(withJSONObject: documentTypesList)) ?? Data()

            // Keywords
            self.keywordRelations = keywords.map { PersistentKeyword(keyword: $0, contractId: id.toBase58String()) }
            self.contractDescription = description

            // Tokens
            self.hasTokens = hasTokens
            self.tokensData = nil

            // Groups
            self.groupsData = nil

            // Documents
            self.documents = []

            // Owner identity link is back-filled later by
            // `ContractIdentityLinker`. Initialise explicitly because
            // SwiftData's auto-init of optional relationships has
            // historically been flaky enough in this codebase to be
            // worth the line.
            self.ownerIdentity = nil

            // Network and timestamps
            self.networkRaw = network.rawValue
            self.lastUpdated = Date()
            self.lastSyncedAt = nil

            // Default values for contract configuration
            self.canBeDeleted = false
            self.readonly = false
            self.keepsHistory = false
            self.documentsKeepHistoryContractDefault = false
            self.documentsMutableContractDefault = true
            self.documentsCanBeDeletedContractDefault = true
        }

        func updateLastAccessed() {
            self.lastAccessedAt = Date()
        }

        func updateVersion(_ newVersion: Int) {
            self.version = newVersion
            self.lastUpdated = Date()
        }

        func markAsSynced() {
            self.lastSyncedAt = Date()
        }

        func addDocument(_ document: PersistentDocument) {
            documents.append(document)
            lastUpdated = Date()
        }

        func removeDocument(withId documentId: String) {
            if let docIdData = Data.identifier(fromBase58: documentId) {
                documents.removeAll { $0.id == docIdData }
            }
            lastUpdated = Date()
        }
    }

    @Model
    final class PersistentDocument {
        /// Index `networkRaw` to keep per-network document scans
        /// index-served. The static `predicate(contractId:network:)` helper
        /// and every UI list view filter by the active network.
        #Index<PersistentDocument>([\.networkRaw])

        // Primary key
        @Attribute(.unique) var documentId: String

        // Core document properties
        var documentType: String
        var revision: Int32
        var data: Data

        // References (stored as strings for queries)
        var contractId: String
        var ownerId: String

        // Binary data for efficient operations
        var contractIdData: Data
        var ownerIdData: Data

        // Timestamps
        var createdAt: Date
        var updatedAt: Date
        var transferredAt: Date?

        // Block heights
        var createdAtBlockHeight: Int64?
        var updatedAtBlockHeight: Int64?
        var transferredAtBlockHeight: Int64?

        // Core block heights
        var createdAtCoreBlockHeight: Int64?
        var updatedAtCoreBlockHeight: Int64?
        var transferredAtCoreBlockHeight: Int64?

        // Network
        /// Stored as the `Network.rawValue` `UInt32` so SwiftData
        /// `#Predicate` expressions can evaluate it directly. See
        /// `PersistentIdentity.networkRaw` for the full rationale.
        var networkRaw: UInt32

        /// Type-safe accessor over `networkRaw`. Setter writes through.
        var network: Network {
            get { Network(rawValue: networkRaw) ?? .testnet }
            set { networkRaw = newValue.rawValue }
        }

        // Deletion flag
        var isDeleted: Bool = false

        // Local tracking
        var localCreatedAt: Date
        var localUpdatedAt: Date

        // Relationships
        var documentType_relation: PersistentDocumentType?
        var dataContract: PersistentDataContract?

        // Optional reference to local identity (if owner is local)
        var ownerIdentity: PersistentIdentity?

        // Computed properties
        var id: Data {
            Data.identifier(fromBase58: documentId) ?? Data()
        }

        var idBase58: String {
            documentId
        }

        var ownerIdBase58: String {
            ownerId
        }

        var contractIdBase58: String {
            contractId
        }

        var properties: [String: Any]? {
            try? JSONSerialization.jsonObject(with: data, options: []) as? [String: Any]
        }

        var displayTitle: String {
            guard let props = properties else { return "Document" }

            if let title = props["title"] as? String { return title }
            if let name = props["name"] as? String { return name }
            if let label = props["label"] as? String { return label }
            if let normalizedLabel = props["normalizedLabel"] as? String { return normalizedLabel }

            return documentType
        }

        var summary: String {
            var parts: [String] = []

            parts.append("Type: \(documentType)")
            parts.append("Rev: \(revision)")

            // Pin to Gregorian so the `createdAt` year stays CE even
            // when the device is configured for a non-Gregorian
            // calendar (e.g. Thai region → Buddhist era). The SDK
            // doesn't depend on the app's `AppDate` helper, so we
            // configure the formatter inline.
            let formatter = DateFormatter()
            formatter.calendar = Calendar(identifier: .gregorian)
            formatter.dateStyle = .short
            parts.append("Created: \(formatter.string(from: createdAt))")

            return parts.joined(separator: " • ")
        }

        init(
            documentId: String,
            documentType: String,
            revision: Int32,
            data: Data,
            contractId: String,
            ownerId: String,
            network: Network
        ) {
            self.documentId = documentId
            self.documentType = documentType
            self.revision = revision
            self.data = data
            self.contractId = contractId
            self.ownerId = ownerId
            self.contractIdData = Data.identifier(fromBase58: contractId) ?? Data()
            self.ownerIdData = Data.identifier(fromBase58: ownerId) ?? Data()
            self.networkRaw = network.rawValue
            self.createdAt = Date()
            self.updatedAt = Date()
            self.localCreatedAt = Date()
            self.localUpdatedAt = Date()
        }

        // MARK: - Methods
        func updateProperties(_ newData: Data) {
            self.data = newData
            self.updatedAt = Date()
        }

        func updateRevision(_ newRevision: Int64) {
            self.revision = Int32(newRevision)
            self.updatedAt = Date()
        }

        func markAsDeleted() {
            self.isDeleted = true
            self.updatedAt = Date()
        }

        // MARK: - Static Methods
        static func predicate(documentId: String) -> Predicate<PersistentDocument> {
            #Predicate<PersistentDocument> { doc in
                doc.documentId == documentId && doc.isDeleted == false
            }
        }

        static func predicate(contractId: String, network: Network) -> Predicate<PersistentDocument> {
            // See `PersistentIdentity.predicate(network:)` — Foundation's
            // predicate engine can't capture `Network`, so we filter on
            // the UInt32-backed `networkRaw` shadow field.
            let target = network.rawValue
            return #Predicate<PersistentDocument> { doc in
                doc.contractId == contractId && doc.networkRaw == target && doc.isDeleted == false
            }
        }

        static func predicate(ownerId: Data) -> Predicate<PersistentDocument> {
            let ownerIdString = ownerId.toBase58String()
            return #Predicate<PersistentDocument> { doc in
                doc.ownerId == ownerIdString && doc.isDeleted == false
            }
        }

        // MARK: - Identity Linking
        func linkToLocalIdentityIfNeeded(in modelContext: ModelContext) {
            guard ownerIdentity == nil else { return }

            let ownerIdToMatch = self.ownerIdData
            let identityPredicate = #Predicate<PersistentIdentity> { identity in
                identity.identityId == ownerIdToMatch && identity.isLocal == true
            }

            let descriptor = FetchDescriptor<PersistentIdentity>(predicate: identityPredicate)

            do {
                if let localIdentity = try modelContext.fetch(descriptor).first {
                    self.ownerIdentity = localIdentity
                    self.localUpdatedAt = Date()
                }
            } catch {
                print("Failed to link document to local identity: \(error)")
            }
        }
    }

    @Model
    final class PersistentDocumentType {
        @Attribute(.unique) var id: Data
        var contractId: Data
        var name: String

        // Schema stored as JSON
        var schemaJSON: Data
        var propertiesJSON: Data

        // Document behavior settings
        var documentsKeepHistory: Bool
        var documentsMutable: Bool
        var documentsCanBeDeleted: Bool
        var documentsTransferable: Bool

        // indexOnly storage mode (meta-schema v3, protocol version 14): no
        // stored rows — the index entries ARE the documents
        var indexOnly: Bool = false

        // Required fields
        var requiredFieldsJSON: Data?

        // Security
        var securityLevel: Int

        // Trade and creation restrictions
        var tradeMode: Int
        var creationRestrictionMode: Int

        // Identity encryption keys
        var requiresIdentityEncryptionBoundedKey: Bool
        var requiresIdentityDecryptionBoundedKey: Bool

        // Timestamps
        var createdAt: Date
        var lastAccessedAt: Date

        // Relationship to data contract
        var dataContract: PersistentDataContract?

        // Relationship to documents
        @Relationship(deleteRule: .cascade, inverse: \PersistentDocument.documentType_relation)
        var documents: [PersistentDocument]?

        // Relationship to indices
        @Relationship(deleteRule: .cascade, inverse: \PersistentIndex.documentType)
        var indices: [PersistentIndex]?

        // Relationship to properties
        @Relationship(deleteRule: .cascade, inverse: \PersistentProperty.documentType)
        var propertiesList: [PersistentProperty]?

        init(contractId: Data, name: String, schemaJSON: Data, propertiesJSON: Data) {
            // Create unique ID by combining contract ID and name
            var idData = contractId
            idData.append(name.data(using: .utf8) ?? Data())
            self.id = idData

            self.contractId = contractId
            self.name = name
            self.schemaJSON = schemaJSON
            self.propertiesJSON = propertiesJSON
            self.documentsKeepHistory = false
            self.documentsMutable = true
            self.documentsCanBeDeleted = true
            self.documentsTransferable = false
            self.securityLevel = 0
            self.tradeMode = 0
            self.creationRestrictionMode = 0
            self.requiresIdentityEncryptionBoundedKey = false
            self.requiresIdentityDecryptionBoundedKey = false
            self.createdAt = Date()
            self.lastAccessedAt = Date()
        }
    }

    @Model
    final class PersistentIdentity {
        /// Index `networkRaw` so per-network scans (`#Predicate { $0.networkRaw == raw }`)
        /// don't degrade to a table scan. Every UI surface that lists
        /// identities filters by the active network.
        #Index<PersistentIdentity>([\.networkRaw])

        // MARK: - Core Properties
        @Attribute(.unique) var identityId: Data
        var balance: Int64
        var revision: Int64
        /// `true` iff this identity is YOURS or deliberately tracked on
        /// this device, two ways in:
        /// - wallet-derived: identities of a wallet on this device are
        ///   ALWAYS local — the persister promotes the flag when it
        ///   attaches the `wallet` relationship, and the startup heal
        ///   repairs rows persisted before that rule existed;
        /// - manually added: the user loaded/watched the identity via a
        ///   UI flow (LoadIdentityView by id/name), which marks its own
        ///   row (the initializer default `true` matches — a directly
        ///   constructed row is a manual add).
        ///
        /// `false` only for incidental rows — observed foreign
        /// identities materialized by sync that nobody asked to track.
        /// The flag is PROMOTE-ONLY: no sync path ever writes `false`
        /// over a `true` (a manual mark must survive Platform data
        /// flowing over the row, and losing a wallet link doesn't
        /// un-track an identity).
        ///
        /// It makes no claim about signing capability — compute that
        /// live where needed; wallet-owned filtering has
        /// `walletOwnedIdentitiesPredicate`.
        var isLocal: Bool
        var alias: String?
        /// User's chosen primary display label (the one rendered on
        /// list rows and avatars). Populated only when the user selects a
        /// main name from `mainDpnsName` selection or as the fallback set
        /// during initial registration. The full label collection lives on
        /// the `dpnsNames` relationship below; this scalar is just the
        /// "show this one in the cell" hint.
        var dpnsName: String?
        var mainDpnsName: String?
        var identityType: String

        // MARK: - Special Key Storage (stored in keychain)
        var votingPrivateKeyIdentifier: String?
        var ownerPrivateKeyIdentifier: String?
        var payoutPrivateKeyIdentifier: String?

        // MARK: - Public Keys
        @Relationship(deleteRule: .cascade) var publicKeys: [PersistentPublicKey]

        // MARK: - Timestamps
        var createdAt: Date
        var lastUpdated: Date
        var lastSyncedAt: Date?

        // MARK: - Network
        /// Stored as the `Network.rawValue` `UInt32` so SwiftData
        /// `#Predicate` expressions can evaluate it directly. Foundation's
        /// predicate engine rejects captured non-primitive types — even
        /// Codable raw-value enums crash at evaluation with
        /// "Unsupported Predicate: Captured/constant values of type
        /// 'Network' are not supported". The `network` computed
        /// accessor below keeps the public API type-safe; only predicates
        /// that need to filter by network reach for `networkRaw`.
        var networkRaw: UInt32

        /// Type-safe accessor over `networkRaw`. Reads fall back to
        /// `.testnet` if the stored raw value ever drifts out of the
        /// `Network` range (shouldn't happen — writers only go through
        /// this setter which uses `Network.rawValue`).
        var network: Network {
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
        var wallet: PersistentWallet?
        /// DIP-9 identity index within the owning wallet. Mirrors the
        /// `identity_index` carried on `IdentityEntryFFI` from Rust.
        /// Only meaningful when `wallet != nil`; defaults to 0
        /// otherwise. Used to stable-sort identities within a wallet
        /// (e.g. when grouping public keys by identity).
        var identityIndex: UInt32 = 0

        // MARK: - Relationships
        @Relationship(deleteRule: .cascade, inverse: \PersistentDocument.ownerIdentity) var documents: [PersistentDocument]
        @Relationship(deleteRule: .nullify) var tokenBalances: [PersistentTokenBalance]

        /// Confirmed DPNS labels observed for this identity. Cascade-deleted from
        /// the parent — losing the identity row drops the label cache and retained
        /// marketplace history too. A name that leaves this wallet remains related
        /// to its departed identity for history with
        /// `PersistentDPNSName.isOwned == false`. A transfer to another identity in
        /// the same wallet instead rebinds the schema's single unique-name row to
        /// the current owner. Owned-name surfaces use
        /// `PersistentDPNSName.predicate(identityId:)`.
        @Relationship(deleteRule: .cascade, inverse: \PersistentDPNSName.identity)
        var dpnsNames: [PersistentDPNSName] = []

        /// DashPay profile cache for this identity — at most one row per
        /// (network, identity) per the contract's per-`ownerId`
        /// uniqueness on the `profile` document. Cascade-deleted from the
        /// parent. Optional because not every identity has published a
        /// profile (and the FFI changeset's `dashpay_profile: None`
        /// semantics mean "no update", not "delete" — the persister never
        /// nils this out from a flush). Inserted / refreshed by
        /// `PlatformWalletPersistenceHandler.upsertDashpayProfile(...)`.
        @Relationship(deleteRule: .cascade, inverse: \PersistentDashpayProfile.identity)
        var dashpayProfile: PersistentDashpayProfile?

        /// DashPay contact-request rows owned by this identity (both
        /// outgoing and incoming). Cascade-deleted from the parent. Same
        /// query-by-denormalized-id pattern as `dpnsNames`: filters use
        /// `PersistentDashpayContactRequest.predicate(ownerIdentityId:)`
        /// rather than walking this collection from a SwiftUI view.
        /// Append / overwrite / delete on the write path: the persister
        /// callback applies upserts (per `(owner, contact, isOutgoing)`)
        /// and tombstones (`removed_sent` / `removed_incoming`) directly.
        @Relationship(deleteRule: .cascade, inverse: \PersistentDashpayContactRequest.owner)
        var contactRequests: [PersistentDashpayContactRequest] = []

        /// DashPay payment-history rows owned by this identity.
        /// Cascade-deleted from the parent. Same
        /// query-by-denormalized-id pattern as `contactRequests`: filters
        /// use `PersistentDashpayPayment.predicate(ownerIdentityId:)`
        /// rather than walking this collection from a SwiftUI view.
        /// Populated by `PlatformWalletManager.refreshDashPayPayments`
        /// (FFI getter → upsert), not by the persister callback.
        @Relationship(deleteRule: .cascade, inverse: \PersistentDashpayPayment.owner)
        var dashpayPayments: [PersistentDashpayPayment] = []

        /// DashPay ignored senders (per-sender mute, = block, reversible,
        /// local-only) owned by this identity. Cascade-deleted from the parent.
        /// Persisted from the `ignored` changeset array by `persistContacts`
        /// and read back at load to rebuild the Rust `ignored_senders` set —
        /// without them an ignored sender resurfaces on relaunch. Filters use
        /// `PersistentDashpayIgnoredSender.predicate(ownerIdentityId:)`.
        @Relationship(deleteRule: .cascade, inverse: \PersistentDashpayIgnoredSender.owner)
        var dashpayIgnoredSenders: [PersistentDashpayIgnoredSender] = []

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
        var contactProfiles: [PersistentDashpayContactProfile] = []

        // Contracts in the local store that name this identity as their
        // owner. `.nullify` so deleting the identity leaves the contract
        // rows alive (with `ownerIdentity` nulled) — matches the user's
        // intent that contracts persist independently of whether the owner
        // identity happens to be loaded.
        // The `@Relationship` macro is declared on the contract side
        // (`PersistentDataContract.ownerIdentity`) so this is a plain
        // stored property — see `wallet` above for the same pattern.
        var ownedDataContracts: [PersistentDataContract]

        // MARK: - Initialization
        init(
            identityId: Data,
            balance: Int64 = 0,
            revision: Int64 = 0,
            isLocal: Bool = true,
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
        var identityIdString: String {
            identityId.toHexString()
        }

        var identityIdBase58: String {
            identityId.toBase58String()
        }

        var formattedBalance: String {
            let dashAmount = Double(balance) / 100_000_000_000
            return String(format: "%.8f DASH", dashAmount)
        }

        /// User-facing short name. Priority: `alias` → `mainDpnsName`
        /// → `dpnsName` → truncated hex id. Mirrors the old
        /// `IdentityModel.displayName` extension so views that read
        /// this don't change behavior post-migration.
        var displayName: String {
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

        var identityTypeEnum: IdentityType {
            IdentityType(rawValue: identityType) ?? .user
        }

        // MARK: - Methods
        func updateBalance(_ newBalance: Int64) {
            self.balance = newBalance
            self.lastUpdated = Date()
        }

        func updateRevision(_ newRevision: Int64) {
            self.revision = newRevision
            self.lastUpdated = Date()
        }

        func markAsSynced() {
            self.lastSyncedAt = Date()
        }

        func updateDPNSName(_ name: String?) {
            self.dpnsName = name
            self.lastUpdated = Date()
        }

        func addPublicKey(_ key: PersistentPublicKey) {
            publicKeys.append(key)
            lastUpdated = Date()
        }

        func removePublicKey(withId keyId: Int32) {
            publicKeys.removeAll { $0.keyId == keyId }
            lastUpdated = Date()
        }
    }

    @Model
    final class PersistentIndex {
        @Attribute(.unique) var id: Data
        var contractId: Data
        var documentTypeName: String
        var name: String

        // Index configuration
        var unique: Bool
        var nullSearchable: Bool
        var contested: Bool

        // Count / sum axes (meta-schema v3, protocol version 14). Every
        // keyword is persisted VERBATIM as authored in the contract JSON —
        // `countable` keeps its boolean-or-string spelling ("true" /
        // "countable" / "countableAllowingOffset"), and the `averageable` /
        // `rangeAverageable` sugar is stored as-is rather than desugared.
        // Interpreting the spellings (DPP's normalization rules) is protocol
        // logic and stays out of the SDK; display layers map them for
        // presentation.
        var countable: String?
        var rangeCountable: Bool = false
        var summable: String?
        var rangeSummable: Bool = false
        var averageable: String?
        var rangeAverageable: Bool = false

        // Ranking axes (each adds one ordered secondary tree)
        var rankedCountable: Bool = false
        var rankedSummable: Bool = false
        var rankedAverageable: Bool = false

        // indexOnly member key (the property whose value keys each entry).
        // Persisted only when declared; an omitted terminal on an indexOnly
        // type means $ownerId per DPP, a default display layers apply.
        var terminal: String?

        // Preallocation: creating the refersTo-referenced document also
        // creates this index's trees, and deleting the last entry keeps them
        var preallocated: Bool = false

        // Time-range bucketing transform ({on, range, step, phase}), if any
        var timeRangeJSON: Data?

        // Properties in the index with sorting
        var propertiesJSON: Data

        // Contested details (if contested)
        var contestedDetailsJSON: Data?

        // Timestamps
        var createdAt: Date

        // Relationship to document type
        var documentType: PersistentDocumentType?

        init(contractId: Data, documentTypeName: String, name: String, properties: [String]) {
            // Create unique ID by combining contract ID, document type name, and index name
            var idData = contractId
            idData.append(documentTypeName.data(using: .utf8) ?? Data())
            idData.append(name.data(using: .utf8) ?? Data())
            self.id = idData

            self.contractId = contractId
            self.documentTypeName = documentTypeName
            self.name = name
            self.unique = false
            self.nullSearchable = false
            self.contested = false

            // Store properties as JSON array
            if let jsonData = try? JSONSerialization.data(withJSONObject: properties, options: []) {
                self.propertiesJSON = jsonData
            } else {
                self.propertiesJSON = Data()
            }

            self.createdAt = Date()
        }
    }

    @Model
    final class PersistentKeyword {
        @Attribute(.unique) var id: String
        var keyword: String
        var contractId: String

        // Relationship
        var dataContract: PersistentDataContract?

        init(keyword: String, contractId: String) {
            self.id = "\(contractId)_\(keyword)"
            self.keyword = keyword
            self.contractId = contractId
        }
    }

    @Model
    final class PersistentPendingInput {
        /// Two single-column indexes:
        ///   * `outpoint` — the per-outpoint reconciliation lookup that
        ///     runs on every `upsertUtxo`.
        ///   * `walletId` — per-wallet pending-input scans (cleanup when
        ///     a wallet is removed, the storage explorer's network
        ///     scope, "long-lived non-zero pending count" diagnostics).
        ///
        /// SwiftData allows only a single `#Index` macro per model;
        /// passing multiple key-path arrays declares multiple separate
        /// indexes from one macro call.
        #Index<PersistentPendingInput>([\.outpoint], [\.walletId])
        var outpoint: Data

        /// Position of this input in the spending transaction's input
        /// list. Carried so a future UI surface can render the input
        /// index correctly without re-deriving from the raw tx bytes;
        /// the resolution flow itself only uses `outpoint`.
        var inputIndex: UInt32

        /// 32-byte canonical txid of the spending transaction. Stored
        /// in addition to the relationship below so the entry remains
        /// usable if the parent `PersistentTransaction` isn't yet in the
        /// background context (re-upsert ordering, fault-in lag, …).
        var spendingTxid: Data

        /// The transaction this input belongs to. Cascade-deleted from
        /// the parent side via `PersistentTransaction.pendingInputs` so
        /// removing a tx doesn't leave dangling pending rows.
        var spendingTransaction: PersistentTransaction?

        /// Wallet id (`PersistentTxo.walletId` denorm) so cleanup /
        /// per-wallet diagnostics can scope without joining through the
        /// transaction relationship.
        var walletId: Data

        /// Insertion timestamp — useful for spotting stale entries that
        /// never resolved (orphans whose previous output isn't ours).
        var createdAt: Date

        init(
            outpoint: Data,
            inputIndex: UInt32,
            spendingTxid: Data,
            spendingTransaction: PersistentTransaction?,
            walletId: Data
        ) {
            self.outpoint = outpoint
            self.inputIndex = inputIndex
            self.spendingTxid = spendingTxid
            self.spendingTransaction = spendingTransaction
            self.walletId = walletId
            self.createdAt = Date()
        }
    }

    @Model
    final class PersistentPlatformAddress {
        /// Index `walletId` so per-wallet platform-address scans —
        /// `predicate(walletId:)`, the storage explorer's network scope
        /// fallback, BLAST-sync re-upsert paths — hit an index instead
        /// of scanning the whole table.
        #Index<PersistentPlatformAddress>([\.walletId])

        /// DIP-0018 bech32m-encoded address (`dash1…` / `tdash1…`). Unique
        /// across the SwiftData store — a collision would imply a wallet-
        /// id / derivation path collision.
        @Attribute(.unique) var address: String
        /// `PlatformAddress` type byte: 0 = P2PKH, 1 = P2SH. Matches the
        /// discriminant emitted by the Rust-side FFI.
        var addressType: UInt8
        /// 20-byte address hash. Kept denormalized so the BLAST balance
        /// callback (which gets hashes, not full addresses) can upsert in
        /// one fetch.
        @Attribute(.unique) var addressHash: Data
        /// 33-byte compressed secp256k1 public key, or empty Data if the
        /// Rust side couldn't produce one (pool entries that stored only
        /// a script, etc.).
        var publicKey: Data
        /// DIP-17 account index (field `account` in `PlatformPayment`).
        var accountIndex: UInt32
        /// DIP-17 derivation index within the account.
        var addressIndex: UInt32
        /// BIP32 derivation path (e.g. `"m/9'/5'/17'/0'/0'/0"`).
        var derivationPath: String
        /// Marked used by the Rust address pool (first-seen tx or explicit
        /// `mark_used`), or auto-flipped by BLAST when a non-zero
        /// balance / nonce first arrives.
        var isUsed: Bool
        /// Credit balance in credits (1e11 credits per DASH).
        var balance: UInt64
        /// Current anti-replay nonce.
        var nonce: UInt32
        /// Platform block height where this address first appeared in a
        /// balance changeset. Zero until the address is seen on-chain.
        var firstSeenHeight: UInt32
        /// Platform block height this row's `balance` is current **as of**
        /// — the balance height pin (`AddressFunds::as_of_height` in Rust).
        /// Round-tripped verbatim through the persistence callbacks so the
        /// sync's delta-replay gate survives restarts. Zero means "unknown
        /// provenance" (rows persisted before the pin existed).
        var lastSeenHeight: UInt64
        /// 32-byte wallet ID that owns this address. Denormalized from
        /// `account.wallet.walletId` so per-wallet `@Query` filters don't
        /// have to traverse two optional relationships.
        var walletId: Data
        /// Record timestamps.
        var createdAt: Date
        var lastUpdated: Date

        /// Parent account (PlatformPayment, type tag 14).
        var account: PersistentAccount?

        init(
            address: String,
            addressType: UInt8,
            addressHash: Data,
            publicKey: Data = Data(),
            accountIndex: UInt32,
            addressIndex: UInt32,
            derivationPath: String,
            isUsed: Bool = false,
            balance: UInt64 = 0,
            nonce: UInt32 = 0,
            walletId: Data
        ) {
            self.address = address
            self.addressType = addressType
            self.addressHash = addressHash
            self.publicKey = publicKey
            self.accountIndex = accountIndex
            self.addressIndex = addressIndex
            self.derivationPath = derivationPath
            self.isUsed = isUsed
            self.balance = balance
            self.nonce = nonce
            self.firstSeenHeight = 0
            self.lastSeenHeight = 0
            self.walletId = walletId
            self.createdAt = Date()
            self.lastUpdated = Date()
        }
    }

    @Model
    final class PersistentProperty {
        @Attribute(.unique) var id: Data
        var contractId: Data
        var documentTypeName: String
        var name: String

        // Property type and constraints
        var type: String
        var format: String?
        var contentMediaType: String?
        var byteArray: Bool
        var minItems: Int?
        var maxItems: Int?
        var pattern: String?
        var minLength: Int?
        var maxLength: Int?
        var minValue: Int?
        var maxValue: Int?
        var fieldDescription: String?

        // Property attributes
        var transient: Bool
        var isRequired: Bool

        // Timestamps
        var createdAt: Date

        // Relationship to document type
        var documentType: PersistentDocumentType?

        init(contractId: Data, documentTypeName: String, name: String, type: String) {
            // Create unique ID by combining contract ID, document type name, and property name
            var idData = contractId
            idData.append(documentTypeName.data(using: .utf8) ?? Data())
            idData.append(name.data(using: .utf8) ?? Data())
            self.id = idData

            self.contractId = contractId
            self.documentTypeName = documentTypeName
            self.name = name
            self.type = type
            self.byteArray = false
            self.transient = false
            self.isRequired = false
            self.createdAt = Date()
        }
    }

    @Model
    final class PersistentPublicKey {
        // MARK: - Core Properties
        var keyId: Int32
        var purpose: String
        var securityLevel: String
        var keyType: String
        var readOnly: Bool
        var disabledAt: Int64?

        // MARK: - Key Data
        var publicKeyData: Data

        // MARK: - Contract Bounds
        /// JSON-encoded `[base64(contractId)]` — legacy storage shape
        /// that only retains the contract id, never the document-type
        /// name. New code paths still write here for the id portion;
        /// `contractBoundsDocumentTypeName` carries the doc-type so
        /// the `SingleContractDocumentType` variant round-trips
        /// faithfully. Keeping the field shape lets old SwiftData
        /// stores that predate the doc-type column continue to load
        /// without migration (the doc-type column is just `nil`).
        var contractBoundsData: Data?

        /// When set, the key's bounds are
        /// `.singleContractDocumentType(id: contractBoundsData[0],
        /// documentTypeName: contractBoundsDocumentTypeName)`. When
        /// `nil`, the key is either unbounded (when `contractBoundsData`
        /// is also nil) or bounded to a whole contract via
        /// `.singleContract(id:)`. Optional so old stores load cleanly.
        var contractBoundsDocumentTypeName: String?

        // MARK: - Private Key Reference (optional)
        var privateKeyKeychainIdentifier: String?

        // MARK: - Derivation breadcrumb (derive-sign-destroy)
        /// 32-byte wallet id that owns this identity key, denormalized from the
        /// discovery breadcrumb. Paired with `identityDerivationPath`, it lets the
        /// signer derive this key on demand from the Keychain-held seed instead of
        /// reading a stored scalar. `nil` for rows persisted before this column
        /// existed and for keys with no wallet association; such rows fall back to
        /// the stored scalar until the backfill populates them. Additive optional
        /// column => SwiftData lightweight migration.
        var walletId: Data?

        /// Full DIP-9 identity-authentication path
        /// `m/9'/coin'/5'/0'/ECDSA'/identityIndex'/keyIndex'` the signer feeds to
        /// the mnemonic resolver to derive this key's private scalar at sign time.
        /// The authoritative breadcrumb; `nil` until written on persist or
        /// backfilled from the key's Keychain metadata.
        var identityDerivationPath: String?

        // MARK: - Metadata
        var identityId: String
        var createdAt: Date
        var lastAccessed: Date?

        // MARK: - Relationships
        @Relationship(inverse: \PersistentIdentity.publicKeys)
        var identity: PersistentIdentity?

        // MARK: - Initialization
        init(
            keyId: Int32,
            purpose: KeyPurpose,
            securityLevel: SecurityLevel,
            keyType: KeyType,
            publicKeyData: Data,
            readOnly: Bool = false,
            disabledAt: Int64? = nil,
            contractBounds: [Data]? = nil,
            contractBoundsDocumentTypeName: String? = nil,
            identityId: String
        ) {
            self.keyId = keyId
            self.purpose = String(purpose.rawValue)
            self.securityLevel = String(securityLevel.rawValue)
            self.keyType = String(keyType.rawValue)
            self.publicKeyData = publicKeyData
            self.readOnly = readOnly
            self.disabledAt = disabledAt
            if let contractBounds = contractBounds {
                self.contractBoundsData = try? JSONSerialization.data(withJSONObject: contractBounds.map { $0.base64EncodedString() })
            } else {
                self.contractBoundsData = nil
            }
            self.contractBoundsDocumentTypeName = contractBoundsDocumentTypeName
            self.identityId = identityId
            self.createdAt = Date()
        }

        // MARK: - Computed Properties
        var contractBounds: [Data]? {
            get {
                guard let data = contractBoundsData,
                      let json = try? JSONSerialization.jsonObject(with: data),
                      let strings = json as? [String] else {
                    return nil
                }
                return strings.compactMap { Data(base64Encoded: $0) }
            }
            set {
                // Always clear the doc-type column when the contract-
                // bounds ids change through this setter. The
                // `documentTypeName` is paired with a SPECIFIC id, so
                // mutating ids without explicitly carrying the doc-
                // type would leave the columns inconsistent and make
                // `toIdentityPublicKey()` reconstruct a stale variant.
                // Callers that want the full `.singleContractDocumentType`
                // round-trip should write `contractBoundsDocumentTypeName`
                // explicitly after this setter, or go through
                // `PersistentPublicKey.from(IdentityPublicKey, identityId:)`
                // which sets both columns atomically.
                contractBoundsDocumentTypeName = nil
                if let newValue = newValue {
                    contractBoundsData = try? JSONSerialization.data(withJSONObject: newValue.map { $0.base64EncodedString() })
                } else {
                    contractBoundsData = nil
                }
            }
        }

        var purposeEnum: KeyPurpose? {
            guard let purposeInt = UInt8(purpose) else { return nil }
            return KeyPurpose(rawValue: purposeInt)
        }

        var securityLevelEnum: SecurityLevel? {
            guard let levelInt = UInt8(securityLevel) else { return nil }
            return SecurityLevel(rawValue: levelInt)
        }

        var keyTypeEnum: KeyType? {
            guard let typeInt = UInt8(keyType) else { return nil }
            return KeyType(rawValue: typeInt)
        }

        var isDisabled: Bool {
            disabledAt != nil
        }

        /// Check if this public key has an associated private key identifier
        var hasPrivateKeyIdentifier: Bool {
            privateKeyKeychainIdentifier != nil
        }
    }

    @Model
    final class PersistentToken {
        @Attribute(.unique) var id: Data
        var contractId: Data
        var position: Int
        var name: String

        // Basic token supply info
        var baseSupply: String
        var maxSupply: String?
        var decimals: Int

        // Token conventions
        var localizations: [String: TokenLocalization]?

        // Status flags
        var isPaused: Bool
        var allowTransferToFrozenBalance: Bool

        // History keeping rules
        var keepsTransferHistory: Bool
        var keepsFreezingHistory: Bool
        var keepsMintingHistory: Bool
        var keepsBurningHistory: Bool
        var keepsDirectPricingHistory: Bool
        var keepsDirectPurchaseHistory: Bool

        // Control rules
        var conventionsChangeRules: ChangeControlRules?
        var maxSupplyChangeRules: ChangeControlRules?
        var manualMintingRules: ChangeControlRules?
        var manualBurningRules: ChangeControlRules?
        var freezeRules: ChangeControlRules?
        var unfreezeRules: ChangeControlRules?
        var destroyFrozenFundsRules: ChangeControlRules?
        var emergencyActionRules: ChangeControlRules?

        // Distribution rules
        var perpetualDistribution: TokenPerpetualDistribution?
        var preProgrammedDistribution: TokenPreProgrammedDistribution?
        var newTokensDestinationIdentity: Data?
        var mintingAllowChoosingDestination: Bool
        var distributionChangeRules: TokenDistributionChangeRules?

        // Marketplace rules
        var tradeMode: TokenTradeMode
        var tradeModeChangeRules: ChangeControlRules?

        // Main control group
        var mainControlGroupPosition: Int?
        var mainControlGroupCanBeModified: String?

        // Description
        var tokenDescription: String?

        // Timestamps
        var createdAt: Date
        var lastUpdatedAt: Date

        // Relationships
        var dataContract: PersistentDataContract?

        @Relationship(deleteRule: .cascade)
        var balances: [PersistentTokenBalance]?

        @Relationship(deleteRule: .cascade)
        var historyEvents: [PersistentTokenHistoryEvent]?

        init(contractId: Data, position: Int, name: String, baseSupply: String, decimals: Int = 8) {
            // Create unique ID by combining contract ID and position
            var idData = contractId
            withUnsafeBytes(of: position.bigEndian) { bytes in
                idData.append(contentsOf: bytes)
            }
            self.id = idData

            self.contractId = contractId
            self.position = position
            self.name = name
            self.baseSupply = baseSupply
            self.decimals = decimals

            // Default values
            self.isPaused = false
            self.allowTransferToFrozenBalance = true
            self.keepsTransferHistory = true
            self.keepsFreezingHistory = true
            self.keepsMintingHistory = true
            self.keepsBurningHistory = true
            self.keepsDirectPricingHistory = true
            self.keepsDirectPurchaseHistory = true
            self.mintingAllowChoosingDestination = true
            self.tradeMode = TokenTradeMode.notTradeable

            self.createdAt = Date()
            self.lastUpdatedAt = Date()
        }
    }

    @Model
    final class PersistentTokenBalance {
        /// Index `networkRaw` for per-network balance scans. Token-balance
        /// rows are aggregated per-identity per-token; UI surfaces always
        /// scope to the active network.
        #Index<PersistentTokenBalance>([\.networkRaw])

        // MARK: - Core Properties
        var tokenId: String
        var identityId: Data
        /// Schema-stable signed carrier for the protocol's unsigned balance.
        /// SwiftData/SQLite keep the original `balance` Int64 column unchanged;
        /// interpret its bits through `unsignedBalance` at every API boundary.
        var balance: Int64
        var frozen: Bool

        // MARK: - Timestamps
        var createdAt: Date
        var lastUpdated: Date
        var lastSyncedAt: Date?

        // MARK: - Token Info (Cached)
        var tokenName: String?
        var tokenSymbol: String?
        var tokenDecimals: Int32?

        // MARK: - Network
        /// Stored as the `Network.rawValue` `UInt32` so SwiftData
        /// `#Predicate` expressions can evaluate it directly. See
        /// `PersistentIdentity.networkRaw` for the full rationale.
        var networkRaw: UInt32

        /// Type-safe accessor over `networkRaw`. Setter writes through.
        var network: Network {
            get { Network(rawValue: networkRaw) ?? .testnet }
            set { networkRaw = newValue.rawValue }
        }

        // MARK: - Relationships
        @Relationship(deleteRule: .nullify) var identity: PersistentIdentity?
        @Relationship(inverse: \PersistentToken.balances) var token: PersistentToken?

        // MARK: - Initialization
        init(
            tokenId: String,
            identityId: Data,
            balance: Int64 = 0,
            frozen: Bool = false,
            tokenName: String? = nil,
            tokenSymbol: String? = nil,
            tokenDecimals: Int32? = nil,
            network: Network
        ) {
            self.tokenId = tokenId
            self.identityId = identityId
            self.balance = balance
            self.frozen = frozen
            self.tokenName = tokenName
            self.tokenSymbol = tokenSymbol
            self.tokenDecimals = tokenDecimals
            self.createdAt = Date()
            self.lastUpdated = Date()
            self.lastSyncedAt = nil
            self.networkRaw = network.rawValue
        }

        /// Full-domain unsigned initializer. The distinct argument label preserves
        /// the original public `balance: Int64` source API without making integer
        /// literals ambiguous between signed and unsigned overloads.
        public convenience init(
            tokenId: String,
            identityId: Data,
            unsignedBalance: UInt64,
            frozen: Bool = false,
            tokenName: String? = nil,
            tokenSymbol: String? = nil,
            tokenDecimals: Int32? = nil,
            network: Network
        ) {
            self.init(
                tokenId: tokenId,
                identityId: identityId,
                balance: Int64(bitPattern: unsignedBalance),
                frozen: frozen,
                tokenName: tokenName,
                tokenSymbol: tokenSymbol,
                tokenDecimals: tokenDecimals,
                network: network
            )
        }

        // MARK: - Computed Properties
        /// Lossless full-domain view over the schema-stable signed carrier.
        var unsignedBalance: UInt64 {
            get { UInt64(bitPattern: balance) }
            set { balance = Int64(bitPattern: newValue) }
        }

        var formattedBalance: String {
            let decimals: Int
            if let tokenDecimals {
                decimals = Int(tokenDecimals)
            } else if let tokenDecimals = token?.decimals {
                decimals = tokenDecimals
            } else {
                return "\(unsignedBalance)"
            }

            guard decimals > 0 else { return String(unsignedBalance) }

            // Place the decimal point in the exact integer string. A Double
            // conversion loses low digits well before UInt64.max.
            let digits = String(unsignedBalance)
            let scale = decimals
            if digits.count <= scale {
                return "0." + String(repeating: "0", count: scale - digits.count) + digits
            }
            let split = digits.index(digits.endIndex, offsetBy: -scale)
            return String(digits[..<split]) + "." + String(digits[split...])
        }

        var displayBalance: String {
            if let symbol = tokenSymbol ?? token?.name {
                return "\(formattedBalance) \(symbol)"
            }
            return formattedBalance
        }

        // MARK: - Methods
        /// Original signed-carrier API retained for source compatibility.
        func updateBalance(_ newBalance: Int64) {
            self.balance = newBalance
            self.lastUpdated = Date()
        }

        /// Full-domain unsigned update API.
        func updateUnsignedBalance(_ newBalance: UInt64) {
            self.unsignedBalance = newBalance
            self.lastUpdated = Date()
        }

        func freeze() {
            self.frozen = true
            self.lastUpdated = Date()
        }

        func unfreeze() {
            self.frozen = false
            self.lastUpdated = Date()
        }

        func markAsSynced() {
            self.lastSyncedAt = Date()
        }

        func updateTokenInfo(name: String?, symbol: String?, decimals: Int32?) {
            if let name = name {
                self.tokenName = name
            }
            if let symbol = symbol {
                self.tokenSymbol = symbol
            }
            if let decimals = decimals {
                self.tokenDecimals = decimals
            }
            self.lastUpdated = Date()
        }
    }

    @Model
    final class PersistentTokenHistoryEvent {
        @Attribute(.unique) var id: UUID

        // Event details
        var eventType: String
        var transactionId: Data?
        var blockHeight: Int64?
        var coreBlockHeight: Int64?

        // Participants
        var fromIdentity: Data?
        var toIdentity: Data?
        var performedByIdentity: Data

        // Amounts
        var amount: String?
        var balanceBefore: String?
        var balanceAfter: String?

        // Additional data stored as JSON
        var additionalDataJSON: Data?

        // Description
        var eventDescription: String?

        // Timestamps
        var createdAt: Date
        var eventTimestamp: Date

        // Relationship to token
        @Relationship(inverse: \PersistentToken.historyEvents)
        var token: PersistentToken?

        init(
            eventType: TokenEventType,
            performedByIdentity: Data,
            eventTimestamp: Date = Date()
        ) {
            self.id = UUID()
            self.eventType = eventType.rawValue
            self.performedByIdentity = performedByIdentity
            self.eventTimestamp = eventTimestamp
            self.createdAt = Date()
        }

        // MARK: - Computed Properties
        var eventTypeEnum: TokenEventType {
            TokenEventType(rawValue: eventType) ?? .unknown
        }

        var fromIdentityBase58: String? {
            fromIdentity?.toBase58String()
        }

        var toIdentityBase58: String? {
            toIdentity?.toBase58String()
        }

        var performedByIdentityBase58: String {
            performedByIdentity.toBase58String()
        }

        var displayTitle: String {
            switch eventTypeEnum {
            case .mint:
                return "Minted \(formattedAmount)"
            case .burn:
                return "Burned \(formattedAmount)"
            case .transfer:
                return "Transfer \(formattedAmount)"
            case .freeze:
                return "Frozen \(formattedAmount)"
            case .unfreeze:
                return "Unfrozen \(formattedAmount)"
            case .destroyFrozenFunds:
                return "Destroyed Frozen Funds \(formattedAmount)"
            case .configUpdate:
                return "Configuration Updated"
            case .emergencyAction:
                return "Emergency Action"
            case .perpetualDistribution:
                return "Perpetual Distribution \(formattedAmount)"
            case .preProgrammedRelease:
                return "Pre-programmed Release \(formattedAmount)"
            case .directPricing:
                return "Direct Pricing Updated"
            case .directPurchase:
                return "Direct Purchase \(formattedAmount)"
            case .unknown:
                return "Unknown Event"
            }
        }

        private var formattedAmount: String {
            guard let amount = amount else { return "" }
            return amount
        }

        // MARK: - Additional Data Methods
        func setAdditionalData(_ data: [String: Any]) {
            additionalDataJSON = try? JSONSerialization.data(withJSONObject: data)
        }

        func getAdditionalData() -> [String: Any]? {
            guard let data = additionalDataJSON else { return nil }
            return try? JSONSerialization.jsonObject(with: data) as? [String: Any]
        }
    }

    @Model
    final class PersistentTransaction {
        /// Index on `firstSeen` so per-wallet queries — which fetch
        /// `PersistentTxo` rows by `walletId` then sort their parent
        /// transactions by `firstSeen` — get a sorted scan instead of
        /// an in-memory O(N log N) pass. The unique `txid` index covers
        /// point-lookups; this one covers the timeline.
        #Index<PersistentTransaction>([\.firstSeen])

        /// Transaction ID (32-byte hash, raw little-endian wire bytes —
        /// the same orientation Rust hands us via the FFI `[u8; 32]`).
        /// Stored as raw `Data` so the unique index covers 32 bytes
        /// instead of a 64-char hex string, and the persistence
        /// handler avoids a hex round-trip on every write.
        @Attribute(.unique) var txid: Data
        /// Raw transaction bytes (consensus-encoded — the same wire
        /// format `dashcore::consensus::encode::serialize` produces and
        /// `Transaction::consensus_decode` round-trips). The FFI write
        /// path always populates this; the persister-fallback read path
        /// (`PlatformWalletPersistence::get_core_tx_record`) hands it
        /// back over FFI so Rust can decode a real `Transaction`
        /// without a placeholder body.
        var transactionData: Data
        /// Context: 0=mempool, 1=instantSend, 2=inBlock, 3=inChainLockedBlock.
        var context: UInt32
        /// Block height (0 for mempool).
        var blockHeight: UInt32
        /// Block hash (nil for mempool).
        var blockHash: Data?
        /// Block timestamp.
        var blockTimestamp: UInt32
        /// The transaction's index within its block (`block.vtx` order),
        /// meaningful only when [`hasBlockPosition`]. Pure storage of the
        /// Rust-stamped value (rust-dashcore#891): restored provider special
        /// transactions hand it back so the masternode aggregation keeps
        /// Core's same-block apply order across restarts. `false` on rows
        /// persisted before the field existed and on unconfirmed contexts.
        var blockPosition: UInt32 = 0
        var hasBlockPosition: Bool = false
        /// Direction: 0=incoming, 1=outgoing, 2=internal, 3=coinJoin.
        var direction: UInt32
        /// Transaction type name (Standard, CoinJoin, etc.). Sourced
        /// from Rust's `Debug` repr of `TransactionType` for human
        /// display only — DO NOT use this string as a discriminant;
        /// match on [`transactionTypeKind`] instead. The string is
        /// not a stable wire contract (a `#[derive(Debug)]` rename on
        /// the Rust side would silently change it).
        var transactionType: String
        /// Typed discriminant of Rust's
        /// `key_wallet::transaction_checking::transaction_router::TransactionType`,
        /// kept in lockstep with [`TransactionTypeKind`]. Use this byte
        /// (via [`typedKind`] / [`isAssetLock`] / [`isAssetUnlock`]) to
        /// branch on transaction kind in UI code; the parallel
        /// [`transactionType`] string is human-readable only and not
        /// stable.
        ///
        /// Sentinel `0xFF` means "pre-feature row whose discriminant
        /// hasn't been populated yet" — SPV's next upsert round
        /// replaces it with the real discriminant on touch. Accessors
        /// treat the sentinel as unknown (no branch fires).
        var transactionTypeKind: UInt8 = 0xFF
        /// Net amount in duffs (signed: positive=received, negative=sent).
        var netAmount: Int64
        /// Fee in duffs (nil if unknown).
        var fee: UInt64?
        /// User-assigned label.
        var label: String
        /// Timestamp when first observed (Unix seconds).
        var firstSeen: UInt64

        // MARK: - Provider (masternode) special-transaction payload

        /// Fields lifted by the Rust FFI from a ProRegTx / ProUpServTx
        /// DIP-3 payload (see `provider_payload_fields` in
        /// `rs-platform-wallet-ffi`). All optional — populated only when
        /// [`typedKind`] is `.providerRegistration` / `.providerUpdateService`.
        /// The Swift side never decodes the payload; these are pure storage.
        ///
        /// Masternode service endpoint as `"ip:port"`.
        var providerServiceAddress: String? = nil
        /// ProUpServTx `proTxHash` (32 raw wire bytes) linking the update to
        /// its registration. `nil` for ProRegTx (whose own txid is the
        /// proTxHash).
        var providerProTxHash: Data? = nil
        /// ProRegTx collateral outpoint txid (32 raw wire bytes); pair with
        /// [`providerCollateralVout`]. `nil` when not a ProRegTx.
        var providerCollateralTxid: Data? = nil
        var providerCollateralVout: UInt32 = 0
        /// ProRegTx owner / voting key hashes (hash160, 20 bytes each).
        var providerOwnerKeyHash: Data? = nil
        var providerVotingKeyHash: Data? = nil

        /// Record timestamps.
        var createdAt: Date
        var lastUpdated: Date

        /// Transaction outputs created by this transaction.
        ///
        /// Cascade-deletes the matching `PersistentTxo` rows when the
        /// transaction is removed — outputs cannot meaningfully exist
        /// without their containing transaction (the outpoint, script,
        /// amount, and address are all derived from it).
        @Relationship(deleteRule: .cascade, inverse: \PersistentTxo.transaction)
        var outputs: [PersistentTxo] = []

        /// Transaction outputs spent *by* this transaction.
        ///
        /// Inverse of `PersistentTxo.spendingTransaction`. Default
        /// `.nullify` delete rule (do not pass `.cascade`!) — those TXOs
        /// are owned by their *creating* transaction, not this one.
        /// Cascading from the spending side would let a recent tx wipe
        /// outputs of an older tx on delete: a data-loss bug. Removing
        /// this transaction merely detaches the spend-link and the TXOs
        /// flip back to "unspent" until something else claims them.
        @Relationship(inverse: \PersistentTxo.spendingTransaction)
        var inputs: [PersistentTxo] = []

        /// Pending input outpoints — entries this transaction's input
        /// list references but for which no `PersistentTxo` has been
        /// upserted yet. Filled by `PlatformWalletPersistenceHandler.
        /// upsertTransaction` via the FFI's `input_outpoints` slice;
        /// each entry is consumed (deleted) by `upsertUtxo` when the
        /// matching previous-output finally arrives. See
        /// `PersistentPendingInput` for the full reconciliation flow.
        /// Cascade-delete: removing the spending tx drops every pending
        /// row that hasn't resolved yet.
        @Relationship(deleteRule: .cascade, inverse: \PersistentPendingInput.spendingTransaction)
        var pendingInputs: [PersistentPendingInput] = []

        /// Every account whose changeset bucket carried this tx record.
        ///
        /// This is a **superset** of the TXO-derived membership: it
        /// includes payload-only involvement (special-tx payloads whose
        /// Provider Owner / Voting key addresses matched an account) where
        /// no `PersistentTxo` exists in the account, so the TXO join can
        /// never surface it. The persistence handler appends the matched
        /// account here for every record it upserts, mirroring how
        /// `WalletChangeSetFFI::from_changeset` buckets `cs.records` by
        /// `record.account_type` on the Rust side.
        ///
        /// The TXO join (`outputs` / `inputs` → `PersistentTxo.account`)
        /// remains the canonical path for **funds** — balances, spend
        /// tracking, per-address history all flow through it. This join
        /// exists only so payload-only involvement is representable at
        /// all; treat it as "account participation," not "account owns
        /// value in this tx."
        ///
        /// Inverse of `PersistentAccount.involvedTransactions`, declared
        /// on this side only (SwiftData needs the `inverse:` on exactly
        /// one end of a many-to-many pair). Default `.nullify` delete rule
        /// on both sides — deleting an account merely detaches it from the
        /// tx (and vice versa); neither end cascades, since the tx row is
        /// shared across accounts / wallets and the account outlives any
        /// single tx.
        @Relationship(inverse: \PersistentAccount.involvedTransactions)
        var involvedAccounts: [PersistentAccount] = []

        init(
            txid: Data,
            transactionData: Data,
            context: UInt32 = 0,
            blockHeight: UInt32 = 0,
            direction: UInt32 = 0,
            transactionType: String = "Standard",
            netAmount: Int64 = 0,
            firstSeen: UInt64 = 0
        ) {
            self.txid = txid
            self.transactionData = transactionData
            self.context = context
            self.blockHeight = blockHeight
            self.blockTimestamp = 0
            self.direction = direction
            self.transactionType = transactionType
            self.netAmount = netAmount
            self.firstSeen = firstSeen
            self.label = ""
            self.createdAt = Date()
            self.lastUpdated = Date()
        }

        // MARK: - Display Helpers

        /// Hex-encoded txid for UI / log sites. The on-disk row stores
        /// the raw 32 bytes in wire/internal order (matches what
        /// `dashcore::Txid::as_ref()` hands the FFI). The canonical
        /// Bitcoin/Dash display convention is the *reverse* of those
        /// bytes (the `Txid: Display` impl in dashcore-rust does the
        /// same flip), so block-explorer hex matches what users see
        /// here. Storage stays unflipped — predicate fetches compare
        /// wire-order `Data` directly without re-encoding.
        var txidHex: String {
            txid.reversed().map { String(format: "%02x", $0) }.joined()
        }

        var contextName: String {
            switch context {
            case 0: return "Mempool"
            case 1: return "InstantSend"
            case 2: return "In Block"
            case 3: return "Chain Locked"
            default: return "Unknown"
            }
        }

        var directionName: String {
            switch direction {
            case 0: return "Incoming"
            case 1: return "Outgoing"
            case 2: return "Internal"
            case 3: return "CoinJoin"
            default: return "Unknown"
            }
        }

        /// Typed view onto [`transactionTypeKind`]. `nil` only for the
        /// `0xFF` sentinel (pre-feature row not yet re-persisted by SPV)
        /// or for a future Rust-side variant addition Swift hasn't
        /// learned about yet — both treated as "unknown" by the
        /// `isAssetLock` / `isAssetUnlock` accessors so an unexpected
        /// byte never silently fires the wrong branch.
        var typedKind: TransactionTypeKind? {
            TransactionTypeKind(rawValue: transactionTypeKind)
        }

        /// `true` when this transaction is a Dash Platform asset-lock
        /// funding tx — a Layer-1 burn that mints Layer-2 credits. The
        /// wallet's `direction` classifier reports `Internal` because the
        /// credit output is derived from this wallet's identity-funding
        /// account, but the *intent* is conversion to L2 credits, not
        /// "transaction to myself."
        var isAssetLock: Bool {
            typedKind == .assetLock
        }

        /// Companion to [`isAssetLock`] — withdrawal back to L1.
        var isAssetUnlock: Bool {
            typedKind == .assetUnlock
        }

        /// `true` for a masternode provider-registration (ProRegTx).
        var isProviderRegistration: Bool {
            typedKind == .providerRegistration
        }

        /// `true` for a masternode provider-update-service (ProUpServTx).
        var isProviderUpdateService: Bool {
            typedKind == .providerUpdateService
        }

        /// ProUpServTx proTxHash in block-explorer (reversed) hex, or `nil`.
        /// Matches [`txidHex`]'s display-order convention.
        var providerProTxHashHex: String? {
            providerProTxHash.map { $0.reversed().map { String(format: "%02x", $0) }.joined() }
        }

        /// ProRegTx collateral outpoint as `"txidHex:vout"` in display order,
        /// or `nil` when there's no collateral field.
        var providerCollateralDisplay: String? {
            guard let txid = providerCollateralTxid else { return nil }
            let hex = txid.reversed().map { String(format: "%02x", $0) }.joined()
            return "\(hex):\(providerCollateralVout)"
        }

        /// ProRegTx owner key hash (hash160) in hex — key hashes are shown
        /// in their natural forward byte order, unlike txids.
        var providerOwnerKeyHashHex: String? {
            providerOwnerKeyHash.map { $0.map { String(format: "%02x", $0) }.joined() }
        }

        /// ProRegTx voting key hash (hash160) in forward-order hex.
        var providerVotingKeyHashHex: String? {
            providerVotingKeyHash.map { $0.map { String(format: "%02x", $0) }.joined() }
        }

        /// `true` for masternode provider special transactions (ProRegTx
        /// and the three ProUp*Tx kinds). Like asset locks, these get
        /// classified `Internal` by the wallet's direction logic (the
        /// wallet only sees its own owner/voting/payout keys referenced
        /// in the payload), so direction-derived labels like
        /// "Self-Transfer" are misleading for them.
        var isProviderSpecial: Bool {
            providerSpecialName != nil
        }

        /// Human-readable name for provider special transactions, `nil`
        /// for every other kind.
        var providerSpecialName: String? {
            switch typedKind {
            case .providerRegistration: return "Provider Registration"
            case .providerUpdateRegistrar: return "Provider Update Registrar"
            case .providerUpdateService: return "Provider Update Service"
            case .providerUpdateRevocation: return "Provider Update Revocation"
            default: return nil
            }
        }

        /// Direction text for UI surfaces, overridden for asset-lock /
        /// asset-unlock txs (the L1 DASH isn't going "to myself" — it's
        /// being converted to / from L2 platform credits) and for
        /// provider special txs (the payload references our keys but no
        /// value moves "to myself").
        ///
        /// Use this anywhere a human-readable "what happened" label is
        /// needed; fall back to [`directionName`] only when the consumer
        /// genuinely needs the raw direction (e.g. the filter dropdown).
        var displayDirection: String {
            if isAssetLock { return "Asset Lock" }
            if isAssetUnlock { return "Asset Unlock" }
            if let name = providerSpecialName { return name }
            return directionName
        }

        var formattedAmount: String {
            let dash = Double(abs(netAmount)) / 100_000_000.0
            let sign = netAmount >= 0 ? "+" : "-"
            return String(format: "%@%.8f DASH", sign, dash)
        }
    }

    @Model
    final class PersistentTxo {
        /// Index `walletId` so per-wallet TXO scans — the canonical
        /// "show every TXO (and, by union of `transaction` +
        /// `spendingTransaction`, every transaction) that touches wallet
        /// W" path — hit an index instead of scanning the entire TXO
        /// table. The denorm is what makes the predicate translatable
        /// to SQL in the first place; this just makes the resulting
        /// query fast at scale.
        #Index<PersistentTxo>([\.walletId])

        /// Outpoint: 36 raw bytes (32-byte txid in wire orientation +
        /// 4-byte vout little-endian) — the standard Bitcoin outpoint
        /// serialization. Unique identifier stored explicitly so
        /// SwiftData predicate fetches can hit a single column without
        /// traversing the `transaction` relationship. Always equals
        /// `PersistentTxo.makeOutpoint(txid: transaction.txid, vout: vout)`.
        @Attribute(.unique) var outpoint: Data
        /// Output index within the transaction.
        var vout: UInt32
        /// Value in duffs.
        var amount: UInt64
        /// Owning address (Base58Check).
        var address: String
        /// Script pubkey bytes.
        var scriptPubKey: Data
        /// Block height where created.
        var height: UInt32
        /// Whether this is a coinbase output.
        var isCoinbase: Bool
        /// Whether confirmed in a block.
        var isConfirmed: Bool
        /// Whether locked by InstantSend.
        var isInstantLocked: Bool
        /// Whether reserved/locked for a specific purpose.
        var isLocked: Bool
        /// Whether this TXO has been spent.
        ///
        /// Denormalized: should track `spendingTransaction != nil`. Kept
        /// as an explicit column because per-row spent/unspent filters
        /// are a hot query path, and chasing the optional relationship
        /// in a predicate drops SwiftData onto the same nested-optional
        /// codepath that crashes elsewhere. The persistence handler is
        /// responsible for keeping the two in sync; do not enforce
        /// invariants here.
        var isSpent: Bool
        /// Record timestamps.
        var createdAt: Date
        var lastUpdated: Date

        /// 32-byte wallet ID this TXO belongs to. Denormalized from
        /// `account?.wallet.walletId` so per-wallet `@Query` predicates
        /// can filter with a single equality check instead of chaining
        /// through the optional `account` relationship — SwiftData's
        /// predicate compiler can't translate that chain into SQLite and
        /// crashes with `Unsupported function expression TERNARY(...).walletId`.
        /// This is the single column callers filter on for "show every
        /// TXO (and, by union of `transaction` + `spendingTransaction`,
        /// every transaction) that touches wallet W". Empty `Data()` for
        /// rows migrated from older schema; the next sync pass will
        /// populate it.
        var walletId: Data = Data()

        /// Containing transaction (the one that *created* this output).
        /// Cascade-deleted from the parent side (see
        /// `PersistentTransaction.outputs`). Optional only because the
        /// underlying SwiftData inverse must allow nil during the brief
        /// window between row insert and relationship attachment; in
        /// steady state every TXO has a non-nil `transaction`.
        var transaction: PersistentTransaction?

        /// The transaction that *spent* this output, or nil if the TXO
        /// is unspent. Inverse of `PersistentTransaction.inputs`. Uses
        /// the default `.nullify` delete rule from that side — deleting
        /// the spending tx must not cascade-delete this row.
        var spendingTransaction: PersistentTransaction?

        /// Position of this output within `spendingTransaction.input`
        /// (i.e. the canonical "vin index"). Captured at the moment the
        /// spend is reconciled — sourced from
        /// `TransactionRecordFFI.input_outpoints` index, which itself
        /// comes from `tx.input.iter()` on the Rust side, so the value
        /// matches the serialized transaction's input ordering exactly.
        /// `nil` when the TXO is unspent (no spending tx, no vin index)
        /// or when migrated from an older row that predates the column.
        /// Surfaced by `TransactionStorageDetailView` so input rows
        /// render in serialized vin order with their real positions
        /// rather than being re-sorted by outpoint hex (which loses
        /// the relationship between row and serialized index).
        var spendingInputIndex: UInt32? = nil

        /// Parent account. No longer paired with an inverse on the
        /// account side — the canonical account path is
        /// `coreAddress?.account`. This field is the fallback when the
        /// address row isn't yet linked (out-of-order flush, address
        /// pool rebuild, etc.).
        var account: PersistentAccount?

        /// Owning `PersistentCoreAddress` row, if it exists in the
        /// account's address pool. Linked alongside `address` (the
        /// Base58Check string) — the string is the authoritative
        /// identifier and survives even when the address pool is rebuilt
        /// or the TXO was paid to an address never in our pool (e.g. an
        /// outgoing recipient). The relationship is the convenient
        /// pointer for navigating to derivation metadata, balance, and
        /// pool tag without a separate fetch. Inverse of
        /// `PersistentCoreAddress.txos`; `.cascade` on that side so
        /// account / wallet teardown drops TXOs cleanly.
        var coreAddress: PersistentCoreAddress?

        init(
            transaction: PersistentTransaction,
            vout: UInt32,
            amount: UInt64,
            address: String,
            scriptPubKey: Data = Data(),
            height: UInt32 = 0
        ) {
            self.outpoint = Self.makeOutpoint(txid: transaction.txid, vout: vout)
            self.vout = vout
            self.amount = amount
            self.address = address
            self.scriptPubKey = scriptPubKey
            self.height = height
            self.isCoinbase = false
            self.isConfirmed = false
            self.isInstantLocked = false
            self.isLocked = false
            self.isSpent = false
            self.createdAt = Date()
            self.lastUpdated = Date()
            self.transaction = transaction
        }

        /// Build the 36-byte outpoint key (32-byte txid raw bytes +
        /// 4-byte vout little-endian). Exposed so the persistence
        /// handler can compose predicates / lookups directly from the
        /// FFI's `[u8; 32]` + `u32` without going through string
        /// formatting.
        static func makeOutpoint(txid: Data, vout: UInt32) -> Data {
            var data = Data(capacity: 36)
            data.append(txid)
            var v = vout.littleEndian
            withUnsafeBytes(of: &v) { data.append(contentsOf: $0) }
            return data
        }

        /// Convenience accessor for the containing transaction's txid
        /// as raw 32-byte `Data`. Prefers the `transaction` relationship;
        /// falls back to the first 32 bytes of `outpoint` when the
        /// inverse is briefly nil during insert (so storage-explorer
        /// rows still render a stable identifier rather than collapsing
        /// to empty).
        var txid: Data {
            if let transaction {
                return transaction.txid
            }
            return outpoint.count >= 32 ? Data(outpoint.prefix(32)) : Data()
        }

        /// Hex-encoded txid for UI / log sites. Reverses bytes to match
        /// the canonical block-explorer display (same flip as
        /// `dashcore::Txid: Display`). Mirrors
        /// `PersistentTransaction.txidHex` directly so the two stay in
        /// sync; can't simply forward to it because we want the same
        /// hex even when `transaction` is briefly unattached.
        var txidHex: String {
            let rawTxid = txid
            guard rawTxid.count == 32 else { return "" }
            return rawTxid.reversed().map { String(format: "%02x", $0) }.joined()
        }

        /// Human-readable outpoint (`<txid hex>:<vout>`) for UI / log
        /// sites. Reconstructs from `txidHex` so the byte-flip stays
        /// consistent across all display surfaces.
        var outpointHex: String {
            let hex = txidHex
            return hex.isEmpty ? "" : "\(hex):\(vout)"
        }

        var formattedAmount: String {
            let dash = Double(amount) / 100_000_000.0
            return String(format: "%.8f DASH", dash)
        }
    }

    @Model
    final class PersistentWallet {
        /// Index `networkRaw` so per-network wallet scans (used everywhere
        /// from the network-scoped storage explorer to the per-network
        /// "is there a wallet on this chain yet" lookups) don't degrade
        /// to a table scan. Also index `walletGroupId` so the Wallet Info
        /// "Networks" lookup — which fetches every sibling-network row for
        /// a seed by its group id — stays a keyed scan.
        #Index<PersistentWallet>([\.networkRaw], [\.walletGroupId])
        #Unique<PersistentWallet>([\.walletId])

        /// 32-byte NETWORK-SCOPED wallet ID, and the row's primary
        /// uniqueness key. Since the network-scoping change the same seed
        /// yields a DISTINCT `walletId` per network (a domain-tagged network
        /// byte is folded into the digest), so a wallet that exists on
        /// multiple chains has one row per network, each with its own id —
        /// the network is already baked into the id, so `walletId` alone is
        /// globally unique (an earlier `(walletId, networkRaw)` composite
        /// was a leftover from the pre-scoping model, where one seed shared
        /// a single id across networks and `networkRaw` was the only
        /// distinguishing column). To gather a seed's sibling-network rows,
        /// group by `walletGroupId` (which is the same across networks),
        /// not by this id.
        var walletId: Data
        /// 32-byte NETWORK-INDEPENDENT group id shared by every network's
        /// wallet derived from the same seed (Rust computes it as the
        /// no-network digest of the root key). Distinct from `walletId`,
        /// which is network-scoped. Used to group a seed's sibling-network
        /// rows in the Wallet Info "Networks" section. Defaults to empty
        /// for rows written before this column existed (pre-release, no
        /// migration); consumers treat empty as "legacy — this single row
        /// only".
        var walletGroupId: Data = Data()
        /// Network this wallet belongs to. `nil` means "not yet known" —
        /// the row was created by a changeset before `persistWalletMetadata`
        /// filled the network in. Views treat `nil` as unknown.
        ///
        /// Stored as the `Network.rawValue` `UInt32?` so SwiftData
        /// `#Predicate` expressions can evaluate it directly. See
        /// `PersistentIdentity.networkRaw` for the full rationale.
        var networkRaw: UInt32?

        /// Type-safe accessor over `networkRaw`. `nil` round-trips as
        /// `nil`; non-nil reads fall back to `.testnet` if the stored
        /// raw value ever drifts out of the `Network` range.
        var network: Network? {
            get {
                guard let raw = networkRaw else { return nil }
                return Network(rawValue: raw) ?? .testnet
            }
            set { networkRaw = newValue?.rawValue }
        }
        /// Optional wallet name.
        var name: String?
        /// Optional free-form user-supplied description. Mirrored into
        /// the keychain metadata blob (see `WalletKeychainMetadata`) so
        /// it survives a SwiftData wipe / reinstall via the
        /// orphan-mnemonic recovery flow. No UI surfaces this yet, but
        /// the column is wired so existing rows roll forward without a
        /// schema migration when it lands.
        var walletDescription: String?
        /// Birth height — block height when the wallet was created.
        var birthHeight: UInt32
        /// Last synced core block height.
        var syncedHeight: UInt32
        /// Timestamp of last sync (Unix seconds).
        var lastSynced: UInt64
        /// Bincode-serialised
        /// `dashcore::ephemerealdata::chain_lock::ChainLock` carrying the
        /// wallet's `WalletMetadata::last_applied_chain_lock` from the
        /// previous session. Roundtripped across app launches so the
        /// asset-lock-resume CL-from-metadata fallback in Rust's
        /// `proof.rs` can fire on catch-up at launch without waiting
        /// for SPV to re-apply a fresh ChainLock. `nil` when no
        /// ChainLock has ever been observed for this wallet (fresh
        /// wallet, or pre-feature row).
        var lastAppliedChainLockBytes: Data?
        /// User imported this wallet from an existing mnemonic (as
        /// opposed to generating a fresh one). Cosmetic flag that
        /// drives the "📥 Imported" badge; defaulted to `false` for
        /// rows that predate the column.
        var isImported: Bool = false
        /// Verified seed-binding marker: the BIP44 account-0 xpub that the
        /// Keychain-resolved seed was proven to derive, bound to the mnemonic
        /// Keychain item's identity stamp, written after one successful
        /// `platform_wallet_verify_seed_binds_to_wallet_cached` run. On later
        /// launches the unlock path hands this back to Rust (with the item's
        /// current stamp), which skips the mnemonic-resolving derivation when
        /// it still matches — and re-verifies when the xpub OR the Keychain
        /// item changed. Opaque to Swift — Rust decides match-vs-verify; this
        /// column only stores and returns it. `nil` (rows predating the
        /// column, or never verified) means the full check runs at the next
        /// unlock.
        var seedBindingVerifiedMarker: String?
        /// Record timestamps.
        var createdAt: Date
        var lastUpdated: Date

        /// Accounts belonging to this wallet.
        @Relationship(deleteRule: .cascade, inverse: \PersistentAccount.wallet)
        var accounts: [PersistentAccount]

        /// Identities registered against this wallet. Cardinality is
        /// 0..N — a wallet may have zero identities (freshly created)
        /// or many. Deletion semantics: `.nullify` so an identity
        /// survives a wallet delete as an orphaned row (useful for
        /// post-mortem inspection and possible re-association if the
        /// wallet is re-imported from the same seed).
        ///
        /// Paired with `PersistentIdentity.wallet` (plain stored
        /// property; the inverse key lives on this side).
        @Relationship(deleteRule: .nullify, inverse: \PersistentIdentity.wallet)
        var identities: [PersistentIdentity]

        init(
            walletId: Data,
            walletGroupId: Data = Data(),
            network: Network? = nil,
            name: String? = nil,
            walletDescription: String? = nil,
            birthHeight: UInt32 = 0,
            syncedHeight: UInt32 = 0,
            isImported: Bool = false
        ) {
            self.walletId = walletId
            self.walletGroupId = walletGroupId
            self.networkRaw = network?.rawValue
            self.name = name
            self.walletDescription = walletDescription
            self.birthHeight = birthHeight
            self.syncedHeight = syncedHeight
            self.lastSynced = 0
            self.isImported = isImported
            self.createdAt = Date()
            self.lastUpdated = Date()
            self.accounts = []
            self.identities = []
        }
    }
}
