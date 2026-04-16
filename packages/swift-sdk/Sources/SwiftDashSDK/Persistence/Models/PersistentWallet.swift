import Foundation
import SwiftData

/// SwiftData model for persisting core wallet metadata.
///
/// Represents a single HD wallet with its sync state and balance.
/// Owns accounts via cascade delete — removing a wallet removes all
/// its accounts, transactions, and UTXOs.
@Model
public final class PersistentWallet {
    /// 32-byte wallet ID (SHA256 of root public key).
    @Attribute(.unique) public var walletId: Data
    /// Network name (mainnet, testnet, devnet).
    public var network: String
    /// Optional wallet name.
    public var name: String?
    /// Birth height — block height when the wallet was created.
    public var birthHeight: UInt32
    /// Last synced core block height.
    public var syncedHeight: UInt32
    /// Timestamp of last sync (Unix seconds).
    public var lastSynced: UInt64
    /// Confirmed balance in duffs.
    public var balanceConfirmed: UInt64
    /// Unconfirmed balance in duffs.
    public var balanceUnconfirmed: UInt64
    /// Immature balance in duffs.
    public var balanceImmature: UInt64
    /// Locked balance in duffs.
    public var balanceLocked: UInt64
    /// Record timestamps.
    public var createdAt: Date
    public var lastUpdated: Date

    /// Accounts belonging to this wallet.
    @Relationship(deleteRule: .cascade, inverse: \PersistentAccount.wallet)
    public var accounts: [PersistentAccount]

    public init(
        walletId: Data,
        network: String,
        birthHeight: UInt32 = 0,
        syncedHeight: UInt32 = 0
    ) {
        self.walletId = walletId
        self.network = network
        self.birthHeight = birthHeight
        self.syncedHeight = syncedHeight
        self.lastSynced = 0
        self.balanceConfirmed = 0
        self.balanceUnconfirmed = 0
        self.balanceImmature = 0
        self.balanceLocked = 0
        self.createdAt = Date()
        self.lastUpdated = Date()
        self.accounts = []
    }
}

// MARK: - Queries

extension PersistentWallet {
    public static func predicate(walletId: Data) -> Predicate<PersistentWallet> {
        #Predicate<PersistentWallet> { $0.walletId == walletId }
    }
}
