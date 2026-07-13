import Foundation
import SwiftData

/// One pre-derived platform-node (Ed25519) public key persisted on a
/// `ProviderPlatformKeys` account (`accountType == 11`).
///
/// Ed25519/SLIP-10 is hardened-only, so the wallet can never extend
/// its platform-node pool without the seed — the batch is derived once
/// at wallet registration (while the seed is available) and stored here
/// so the Node Keys detail screen lists it with no keychain prompt. The
/// 20-byte `nodeId` is `hash160(publicKey)`, precomputed on the Rust
/// side (the host needs no RIPEMD-160). The private scalar is never
/// stored — a per-index reveal re-derives it through the resolver.
public struct DerivedPlatformNodeKey: Codable, Equatable, Hashable, Sendable {
    /// Hardened key index within the platform-node pool (`#0..`).
    public var index: UInt32
    /// Raw 32-byte Ed25519 public key.
    public var publicKey: Data
    /// 20-byte platform node id (`hash160` of `publicKey`).
    public var nodeId: Data

    public init(index: UInt32, publicKey: Data, nodeId: Data) {
        self.index = index
        self.publicKey = publicKey
        self.nodeId = nodeId
    }
}

/// SwiftData model for persisting a wallet account.
///
/// Each account represents an HD derivation path (BIP44, CoinJoin,
/// Identity, Platform Payment, etc.) with its own address pools.
///
/// Note: an account does **not** own a list of TXOs directly. A
/// single transaction can produce outputs into several accounts (or
/// even several wallets), and TXOs hang off the per-address
/// `PersistentCoreAddress.txos` collection so the account ↔ TXO link
/// flows naturally through the address pool. Per-account TXOs are
/// derived as `coreAddresses.flatMap(\.txos)`; per-account
/// transactions with funds are the union of those TXOs' `transaction`
/// (creating tx) and `spendingTransaction` (spending tx). Account
/// scope for funds flows through addresses.
///
/// The one thing the address / TXO graph cannot express is
/// **payload-only** involvement — a special tx (e.g. a ProRegTx)
/// that matched this account through a Provider Owner / Voting key
/// address in its payload while creating no TXO here. Those txs are
/// carried by the explicit `involvedTransactions` many-to-many, the
/// only denormalized account ↔ transaction link on this side; see
/// `PersistentTransaction.involvedAccounts` for the full semantics.
@Model
public final class PersistentAccount {
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
    public var accountType: UInt32
    /// Account index within the type (for indexed account types). For
    /// `PlatformPayment` this is the `account` field; for
    /// `DashpayReceivingFunds` / `DashpayExternalAccount` it's the
    /// account-level selector; for
    /// `IdentityAuthentication{Ecdsa,Bls}` it's the identity index.
    public var accountIndex: UInt32
    /// Human-readable account type name.
    public var accountTypeName: String
    /// Per-account confirmed balance in duffs.
    public var balanceConfirmed: UInt64
    /// Per-account unconfirmed balance in duffs.
    public var balanceUnconfirmed: UInt64
    /// External address pool: highest used index (-1 = none).
    public var externalHighestUsed: Int32
    /// Internal (change) address pool: highest used index.
    public var internalHighestUsed: Int32
    /// `StandardAccountTypeTagFFI` value. Meaningful only when
    /// `accountType == 0` (Standard): 0 = BIP44, 1 = BIP32.
    public var standardTag: UInt8
    /// `IdentityTopUp.registration_index`. Zero for other variants.
    public var registrationIndex: UInt32
    /// `PlatformPayment.key_class`. Zero for other variants.
    public var keyClass: UInt32
    /// `Dashpay*`.user_identity_id (32 bytes). Empty `Data` for other
    /// variants.
    public var userIdentityId: Data
    /// `Dashpay*`.friend_identity_id (32 bytes). Empty `Data` for
    /// other variants.
    public var friendIdentityId: Data
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
    @Attribute(.unique) public var accountExtendedPubKeyBytes: Data?
    /// Pre-derived platform-node (Ed25519) public keys for the
    /// `ProviderPlatformKeys` account (`accountType == 11`), captured
    /// at wallet registration while the seed was available. Empty for
    /// every other account type, and for wallets created before this
    /// field existed (the Node Keys screen then falls back to the
    /// resolver-based "Load Keys" path). Populated once by
    /// `on_persist_account_registrations_fn` and read directly by the
    /// UI — never rewritten on the restore / re-persist cycle, so the
    /// batch is durable across relaunches with no keychain prompt. See
    /// [`DerivedPlatformNodeKey`].
    ///
    /// Declared with an inline default so SwiftData's lightweight
    /// migration can add the column to stores created before this
    /// field existed — without it, `ModelContainer` creation fatals
    /// with `loadIssueModelContainer` on first launch after upgrade.
    public var derivedPlatformNodeKeys: [DerivedPlatformNodeKey] = []
    /// Record timestamps.
    public var createdAt: Date
    public var lastUpdated: Date

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
    public var wallet: PersistentWallet

    /// Addresses from this account's address pools (external +
    /// internal, or a single Absent pool for degenerate types). Holds
    /// Core-chain (base58check) addresses only — PlatformPayment
    /// accounts keep their addresses in `platformAddresses`.
    /// Per-account TXOs flow through this collection
    /// (`coreAddresses.flatMap(\.txos)`).
    @Relationship(deleteRule: .cascade, inverse: \PersistentCoreAddress.account)
    public var coreAddresses: [PersistentCoreAddress]

    /// DIP-17 Platform Payment addresses for this account, keyed on
    /// DIP-0018 bech32m encoding. Populated only when
    /// `accountType == 14` (PlatformPayment).
    @Relationship(deleteRule: .cascade, inverse: \PersistentPlatformAddress.account)
    public var platformAddresses: [PersistentPlatformAddress]

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
    public var involvedTransactions: [PersistentTransaction] = []

    public init(
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
        self.derivedPlatformNodeKeys = []
        self.createdAt = Date()
        self.lastUpdated = Date()
        self.coreAddresses = []
        self.platformAddresses = []
        self.involvedTransactions = []
    }
}
