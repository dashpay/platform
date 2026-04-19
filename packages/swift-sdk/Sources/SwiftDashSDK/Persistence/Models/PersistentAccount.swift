import Foundation
import SwiftData

/// SwiftData model for persisting a wallet account.
///
/// Each account represents an HD derivation path (BIP44, CoinJoin,
/// Identity, Platform Payment, etc.) with its own address pools,
/// transactions, and UTXOs. Cascade-deletes transactions and UTXOs
/// when the account is removed.
@Model
public final class PersistentAccount {
    /// Account type identifier — matches the `AccountTypeTagFFI`
    /// discriminant from the Rust side (0 = Standard, 1 = CoinJoin,
    /// … 14 = PlatformPayment). Stable across releases.
    public var accountType: UInt32
    /// Account index within the type (for indexed account types). For
    /// `PlatformPayment` this is the `account` field; for
    /// `DashpayReceivingFunds` / `DashpayExternalAccount` it's the
    /// account-level selector.
    public var accountIndex: UInt32
    /// Human-readable account type name.
    public var accountTypeName: String
    /// Whether this is a watch-only account.
    public var isWatchOnly: Bool
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
    /// `on_persist_account_fn`, consumed by `on_load_wallet_list_fn`
    /// to reconstruct a watch-only `Account` via `Account::from_xpub`.
    /// Empty `Data` means "not yet persisted" — account cannot be
    /// restored silently.
    public var accountExtendedPubKeyBytes: Data
    /// Record timestamps.
    public var createdAt: Date
    public var lastUpdated: Date

    /// Parent wallet.
    public var wallet: PersistentWallet?

    /// Transactions in this account.
    @Relationship(deleteRule: .cascade, inverse: \PersistentTransaction.account)
    public var transactions: [PersistentTransaction]

    /// Unspent transaction outputs in this account.
    @Relationship(deleteRule: .cascade, inverse: \PersistentUtxo.account)
    public var utxos: [PersistentUtxo]

    /// Addresses from this account's address pools (external +
    /// internal, or a single Absent pool for degenerate types). Holds
    /// Core-chain (base58check) addresses only — PlatformPayment
    /// accounts keep their addresses in `platformAddresses`.
    @Relationship(deleteRule: .cascade, inverse: \PersistentCoreAddress.account)
    public var coreAddresses: [PersistentCoreAddress]

    /// DIP-17 Platform Payment addresses for this account, keyed on
    /// DIP-0018 bech32m encoding. Populated only when
    /// `accountType == 14` (PlatformPayment).
    @Relationship(deleteRule: .cascade, inverse: \PersistentPlatformAddress.account)
    public var platformAddresses: [PersistentPlatformAddress]

    public init(
        accountType: UInt32,
        accountIndex: UInt32,
        accountTypeName: String,
        isWatchOnly: Bool = false
    ) {
        self.accountType = accountType
        self.accountIndex = accountIndex
        self.accountTypeName = accountTypeName
        self.isWatchOnly = isWatchOnly
        self.balanceConfirmed = 0
        self.balanceUnconfirmed = 0
        self.externalHighestUsed = -1
        self.internalHighestUsed = -1
        self.standardTag = 0
        self.registrationIndex = 0
        self.keyClass = 0
        self.userIdentityId = Data()
        self.friendIdentityId = Data()
        self.accountExtendedPubKeyBytes = Data()
        self.createdAt = Date()
        self.lastUpdated = Date()
        self.transactions = []
        self.utxos = []
        self.coreAddresses = []
        self.platformAddresses = []
    }
}
