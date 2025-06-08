import Foundation
import SwiftData

/// SwiftData model for persisting token balance data
@Model
final class PersistentTokenBalance {
    // MARK: - Core Properties
    var tokenId: String
    var identityId: Data
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
    var network: String
    
    // MARK: - Relationships
    @Relationship(deleteRule: .nullify) var identity: PersistentIdentity?
    
    // MARK: - Initialization
    init(
        tokenId: String,
        identityId: Data,
        balance: Int64 = 0,
        frozen: Bool = false,
        tokenName: String? = nil,
        tokenSymbol: String? = nil,
        tokenDecimals: Int32? = nil,
        network: String = Network.defaultNetwork.rawValue
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
    var formattedBalance: String {
        guard let decimals = tokenDecimals else {
            return "\(balance)"
        }
        
        let divisor = pow(10.0, Double(decimals))
        let amount = Double(balance) / divisor
        return String(format: "%.\(decimals)f", amount)
    }
    
    var displayBalance: String {
        if let symbol = tokenSymbol {
            return "\(formattedBalance) \(symbol)"
        }
        return formattedBalance
    }
    
    // MARK: - Methods
    func updateBalance(_ newBalance: Int64) {
        self.balance = newBalance
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

// MARK: - Conversion Extensions

extension PersistentTokenBalance {
    /// Create a simple token balance representation
    func toTokenBalance() -> (tokenId: String, balance: UInt64, frozen: Bool) {
        return (tokenId: tokenId, balance: UInt64(max(0, balance)), frozen: frozen)
    }
}

// MARK: - Queries

extension PersistentTokenBalance {
    /// Predicate to find balance by token and identity
    static func predicate(tokenId: String, identityId: Data) -> Predicate<PersistentTokenBalance> {
        #Predicate<PersistentTokenBalance> { balance in
            balance.tokenId == tokenId && balance.identityId == identityId
        }
    }
    
    /// Predicate to find all balances for an identity
    static func predicate(identityId: Data) -> Predicate<PersistentTokenBalance> {
        #Predicate<PersistentTokenBalance> { balance in
            balance.identityId == identityId
        }
    }
    
    /// Predicate to find all balances for a token
    static func predicate(tokenId: String) -> Predicate<PersistentTokenBalance> {
        #Predicate<PersistentTokenBalance> { balance in
            balance.tokenId == tokenId
        }
    }
    
    /// Predicate to find non-zero balances
    static var nonZeroBalancesPredicate: Predicate<PersistentTokenBalance> {
        #Predicate<PersistentTokenBalance> { balance in
            balance.balance > 0
        }
    }
    
    /// Predicate to find frozen balances
    static var frozenBalancesPredicate: Predicate<PersistentTokenBalance> {
        #Predicate<PersistentTokenBalance> { balance in
            balance.frozen == true
        }
    }
    
    /// Predicate to find balances needing sync
    static func needsSyncPredicate(olderThan date: Date) -> Predicate<PersistentTokenBalance> {
        #Predicate<PersistentTokenBalance> { balance in
            balance.lastSyncedAt == nil || balance.lastSyncedAt! < date
        }
    }
}