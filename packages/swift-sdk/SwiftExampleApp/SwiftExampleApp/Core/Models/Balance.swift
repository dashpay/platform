import Foundation

public struct Balance: Equatable, Codable {
    public let confirmed: UInt64
    public let unconfirmed: UInt64
    public let immature: UInt64
    
    public var total: UInt64 {
        confirmed + unconfirmed
    }
    
    public var spendable: UInt64 {
        confirmed
    }
    
    public init(confirmed: UInt64 = 0, unconfirmed: UInt64 = 0, immature: UInt64 = 0) {
        self.confirmed = confirmed
        self.unconfirmed = unconfirmed
        self.immature = immature
    }
    
    // Formatting helpers
    public var formattedConfirmed: String {
        formatDash(confirmed)
    }
    
    public var formattedUnconfirmed: String {
        formatDash(unconfirmed)
    }
    
    public var formattedTotal: String {
        formatDash(total)
    }
    
    private func formatDash(_ amount: UInt64) -> String {
        let dash = Double(amount) / 100_000_000.0
        return String(format: "%.8f DASH", dash)
    }
}

// Detailed balance with additional info
public struct DetailedBalance: Equatable {
    public let balance: Balance
    public let addressCount: Int
    public let utxoCount: Int
    public let lastUpdated: Date
    
    public init(
        balance: Balance,
        addressCount: Int = 0,
        utxoCount: Int = 0,
        lastUpdated: Date = Date()
    ) {
        self.balance = balance
        self.addressCount = addressCount
        self.utxoCount = utxoCount
        self.lastUpdated = lastUpdated
    }
}