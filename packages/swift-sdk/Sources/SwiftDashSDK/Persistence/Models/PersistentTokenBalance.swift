import Foundation
import SwiftData

/// SwiftData model for persisting token balance data
@Model
public final class PersistentTokenBalance {
    /// Index `networkRaw` for per-network balance scans. Token-balance
    /// rows are aggregated per-identity per-token; UI surfaces always
    /// scope to the active network.
    #Index<PersistentTokenBalance>([\.networkRaw])

    // MARK: - Core Properties
    public var tokenId: String
    public var identityId: Data
    /// Schema-stable signed carrier for the protocol's unsigned balance.
    /// SwiftData/SQLite keep the original `balance` Int64 column unchanged;
    /// interpret its bits through `unsignedBalance` at every API boundary.
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
    /// Stored as the `Network.rawValue` `UInt32` so SwiftData
    /// `#Predicate` expressions can evaluate it directly. See
    /// `PersistentIdentity.networkRaw` for the full rationale.
    public var networkRaw: UInt32

    /// Type-safe accessor over `networkRaw`. Setter writes through.
    public var network: Network {
        get { Network(rawValue: networkRaw) ?? .testnet }
        set { networkRaw = newValue.rawValue }
    }

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
    public var unsignedBalance: UInt64 {
        get { UInt64(bitPattern: balance) }
        set { balance = Int64(bitPattern: newValue) }
    }

    public var formattedBalance: String {
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

    public var displayBalance: String {
        if let symbol = tokenSymbol ?? token?.name {
            return "\(formattedBalance) \(symbol)"
        }
        return formattedBalance
    }

    // MARK: - Methods
    /// Original signed-carrier API retained for source compatibility.
    public func updateBalance(_ newBalance: Int64) {
        self.balance = newBalance
        self.lastUpdated = Date()
    }

    /// Full-domain unsigned update API.
    public func updateUnsignedBalance(_ newBalance: UInt64) {
        self.unsignedBalance = newBalance
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
        return (tokenId: tokenId, balance: unsignedBalance, frozen: frozen)
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
            balance.balance != 0
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
