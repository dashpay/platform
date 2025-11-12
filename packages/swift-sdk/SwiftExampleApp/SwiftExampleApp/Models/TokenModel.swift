import Foundation

struct TokenModel: Identifiable {
    let id: String
    let contractId: String
    let name: String
    let symbol: String
    let decimals: Int
    let totalSupply: UInt64
    let balance: UInt64
    let frozenBalance: UInt64
    let availableClaims: [(name: String, amount: UInt64)]
    let pricePerToken: Double // in DASH
    
    init(id: String, contractId: String, name: String, symbol: String, decimals: Int, totalSupply: UInt64, balance: UInt64, frozenBalance: UInt64 = 0, availableClaims: [(name: String, amount: UInt64)] = [], pricePerToken: Double = 0.001) {
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
    
    var formattedBalance: String {
        let divisor = pow(10.0, Double(decimals))
        let tokenAmount = Double(balance) / divisor
        return String(format: "%.\(decimals)f %@", tokenAmount, symbol)
    }
    
    var formattedFrozenBalance: String {
        let divisor = pow(10.0, Double(decimals))
        let tokenAmount = Double(frozenBalance) / divisor
        return String(format: "%.\(decimals)f %@", tokenAmount, symbol)
    }
    
    var formattedTotalSupply: String {
        let divisor = pow(10.0, Double(decimals))
        let tokenAmount = Double(totalSupply) / divisor
        return String(format: "%.\(decimals)f %@", tokenAmount, symbol)
    }
    
    var availableBalance: UInt64 {
        return balance > frozenBalance ? balance - frozenBalance : 0
    }
    
    var formattedAvailableBalance: String {
        let divisor = pow(10.0, Double(decimals))
        let tokenAmount = Double(availableBalance) / divisor
        return String(format: "%.\(decimals)f %@", tokenAmount, symbol)
    }
}