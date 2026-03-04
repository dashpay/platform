import Foundation
import SwiftData

/// SwiftData model for persisting token balance data
@Model
public final class PersistentTokenBalance {
    // MARK: - Core Properties
    public var tokenId: String
    public var identityId: Data
    public var balance: Int64
    public var frozen: Bool

    // MARK: - Timestamps
    public var createdAt: Date
    public var lastUpdated: Date
    public var lastSyncedAt: Date?

    // MARK: - Token Info (Cached)
    public var tokenName: String?
    public var tokenSymbol: String?
    public var tokenDecimals: Int32?

    // MARK: - Network
    public var network: String

    // MARK: - Relationships
    @Relationship(deleteRule: .nullify) public var identity: PersistentIdentity?
    @Relationship(inverse: \PersistentToken.balances) public var token: PersistentToken?

    // MARK: - Initialization
    public init(
        tokenId: String,
        identityId: Data,
        balance: Int64 = 0,
        frozen: Bool = false,
        tokenName: String? = nil,
        tokenSymbol: String? = nil,
        tokenDecimals: Int32? = nil,
        network: String = "testnet"
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
        self.network = network
    }

    // MARK: - Computed Properties
    public var formattedBalance: String {
        guard let decimals = tokenDecimals else {
            return "\(balance)"
        }

        let divisor = pow(10.0, Double(decimals))
        let amount = Double(balance) / divisor
        return String(format: "%.\(decimals)f", amount)
    }

    public var displayBalance: String {
        if let symbol = tokenSymbol {
            return "\(formattedBalance) \(symbol)"
        }
        return formattedBalance
    }

    // MARK: - Methods
    public func updateBalance(_ newBalance: Int64) {
        self.balance = newBalance
        self.lastUpdated = Date()
    }

    public func freeze() {
        self.frozen = true
        self.lastUpdated = Date()
    }

    public func unfreeze() {
        self.frozen = false
        self.lastUpdated = Date()
    }

    public func markAsSynced() {
        self.lastSyncedAt = Date()
    }

    public func updateTokenInfo(name: String?, symbol: String?, decimals: Int32?) {
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

// MARK: - Conversion Extensions

extension PersistentTokenBalance {
    /// Create a simple token balance representation
    public func toTokenBalance() -> (tokenId: String, balance: UInt64, frozen: Bool) {
        return (tokenId: tokenId, balance: UInt64(max(0, balance)), frozen: frozen)
    }
}

// MARK: - Queries

extension PersistentTokenBalance {
    public static func predicate(tokenId: String, identityId: Data) -> Predicate<PersistentTokenBalance> {
        #Predicate<PersistentTokenBalance> { balance in
            balance.tokenId == tokenId && balance.identityId == identityId
        }
    }

    public static func predicate(identityId: Data) -> Predicate<PersistentTokenBalance> {
        #Predicate<PersistentTokenBalance> { balance in
            balance.identityId == identityId
        }
    }

    public static func predicate(tokenId: String) -> Predicate<PersistentTokenBalance> {
        #Predicate<PersistentTokenBalance> { balance in
            balance.tokenId == tokenId
        }
    }

    public static var nonZeroBalancesPredicate: Predicate<PersistentTokenBalance> {
        #Predicate<PersistentTokenBalance> { balance in
            balance.balance > 0
        }
    }

    public static var frozenBalancesPredicate: Predicate<PersistentTokenBalance> {
        #Predicate<PersistentTokenBalance> { balance in
            balance.frozen == true
        }
    }

    public static func needsSyncPredicate(olderThan date: Date) -> Predicate<PersistentTokenBalance> {
        #Predicate<PersistentTokenBalance> { balance in
            balance.lastSyncedAt == nil || balance.lastSyncedAt! < date
        }
    }
}
