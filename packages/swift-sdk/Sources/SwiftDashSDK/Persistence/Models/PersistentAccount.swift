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
    /// Account type identifier (matches Rust AccountType enum raw value).
    public var accountType: UInt32
    /// Account index within the type (for indexed account types).
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
        self.createdAt = Date()
        self.lastUpdated = Date()
        self.transactions = []
        self.utxos = []
    }
}
