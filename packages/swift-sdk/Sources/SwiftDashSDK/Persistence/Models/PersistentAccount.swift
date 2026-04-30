import Foundation
import SwiftData

/// SwiftData model for persisting a wallet account.
///
/// Each account represents an HD derivation path (BIP44, CoinJoin,
/// Identity, Platform Payment, etc.) with its own address pools.
///
/// Note: an account does **not** own a list of transactions or TXOs
/// directly anymore. A single transaction can produce outputs into
/// several accounts (or even several wallets), and TXOs hang off the
/// per-address `PersistentCoreAddress.txos` collection so the
/// account ↔ TXO link flows naturally through the address pool.
/// Per-account TXOs are derived as
/// `coreAddresses.flatMap(\.txos)`; per-account transactions are
/// the union of those TXOs' `transaction` (creating tx) and
/// `spendingTransaction` (spending tx). Account scope flows
/// through addresses; nothing is denormalized on this side.
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
    /// Bincode-encoded `ExtendedPubKey` for this account. Populated by
    /// `on_persist_account_registrations_fn`, consumed by
    /// `on_load_wallet_list_fn` to reconstruct a watch-only `Account`
    /// via `Account::from_xpub`. `nil` means "not yet persisted" —
    /// account cannot be restored silently. Unique because two
    /// accounts can't legitimately share an xpub (would imply a key
    /// reuse / derivation collision); SQL UNIQUE allows multiple
    /// `nil` values, so freshly-inserted unhydrated rows don't
    /// conflict.
    @Attribute(.unique) public var accountExtendedPubKeyBytes: Data?
    /// Record timestamps.
    public var createdAt: Date
    public var lastUpdated: Date

    /// Parent wallet. Every account currently belongs to a wallet. If
    /// standalone non-wallet accounts are introduced later, this
    /// becomes optional again.
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
        self.createdAt = Date()
        self.lastUpdated = Date()
        self.coreAddresses = []
        self.platformAddresses = []
    }
}
