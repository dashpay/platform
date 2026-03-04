import Foundation

public struct TokenModel: Identifiable, Sendable {
    public let id: String
    public let contractId: String
    public let name: String
    public let symbol: String
    public let decimals: Int
    public let totalSupply: UInt64
    public let balance: UInt64
    public let frozenBalance: UInt64
    public let availableClaims: [(name: String, amount: UInt64)]
    public let pricePerToken: Double // in DASH

    public init(id: String, contractId: String, name: String, symbol: String, decimals: Int, totalSupply: UInt64, balance: UInt64, frozenBalance: UInt64 = 0, availableClaims: [(name: String, amount: UInt64)] = [], pricePerToken: Double = 0.001) {
        self.id = id
        self.contractId = contractId
        self.name = name
        self.symbol = symbol
        self.decimals = decimals
        self.totalSupply = totalSupply
        self.balance = balance
        self.frozenBalance = frozenBalance
        self.availableClaims = availableClaims
        self.pricePerToken = pricePerToken
    }

    public var formattedBalance: String {
        let divisor = pow(10.0, Double(decimals))
        let tokenAmount = Double(balance) / divisor
        return String(format: "%.\(decimals)f %@", tokenAmount, symbol)
    }

    public var formattedFrozenBalance: String {
        let divisor = pow(10.0, Double(decimals))
        let tokenAmount = Double(frozenBalance) / divisor
        return String(format: "%.\(decimals)f %@", tokenAmount, symbol)
    }

    public var formattedTotalSupply: String {
        let divisor = pow(10.0, Double(decimals))
        let tokenAmount = Double(totalSupply) / divisor
        return String(format: "%.\(decimals)f %@", tokenAmount, symbol)
    }

    public var availableBalance: UInt64 {
        return balance > frozenBalance ? balance - frozenBalance : 0
    }

    public var formattedAvailableBalance: String {
        let divisor = pow(10.0, Double(decimals))
        let tokenAmount = Double(availableBalance) / divisor
        return String(format: "%.\(decimals)f %@", tokenAmount, symbol)
    }
}
