import Foundation
import DashSDKFFI

// MARK: - Mnemonic Language

/// Language for mnemonic generation
public enum MnemonicLanguage: UInt32, CaseIterable {
    case english = 0
    case chineseSimplified = 1
    case chineseTraditional = 2
    case czech = 3
    case french = 4
    case italian = 5
    case japanese = 6
    case korean = 7
    case portuguese = 8
    case spanish = 9
}

// MARK: - Result Types

/// Balance information for a wallet or account
public struct Balance: Equatable, Codable, Sendable {
    public let confirmed: UInt64
    public let unconfirmed: UInt64
    public let immature: UInt64
    public let locked: UInt64
    public let total: UInt64

    /// Public initializer for Balance
    public init(confirmed: UInt64 = 0, unconfirmed: UInt64 = 0, immature: UInt64 = 0, locked: UInt64 = 0) {
        self.confirmed = confirmed
        self.unconfirmed = unconfirmed
        self.immature = immature
        self.locked = locked
        self.total = confirmed + unconfirmed + immature
    }

    /// Spendable balance (only confirmed, excluding locked)
    public var spendable: UInt64 {
        confirmed > locked ? confirmed - locked : 0
    }

    // MARK: - Formatting Helpers

    /// Format confirmed balance as DASH string
    public var formattedConfirmed: String {
        formatDash(confirmed)
    }

    /// Format unconfirmed balance as DASH string
    public var formattedUnconfirmed: String {
        formatDash(unconfirmed)
    }

    /// Format total balance as DASH string
    public var formattedTotal: String {
        formatDash(total)
    }

    /// Format an amount in duffs as DASH string
    /// - Parameter amount: Amount in duffs (1 DASH = 100,000,000 duffs)
    /// - Returns: Formatted string like "1.23456789 DASH"
    private func formatDash(_ amount: UInt64) -> String {
        let dash = Double(amount) / 100_000_000.0
        return String(format: "%.8f DASH", dash)
    }
}

/// UTXO information
public struct UTXO: Identifiable, Equatable, Sendable {
    public let txid: Data
    public let vout: UInt32
    public let amount: UInt64
    public let address: String
    public let scriptPubKey: Data
    public let height: UInt32
    public let confirmations: UInt32

    /// Unique identifier combining transaction ID and output index
    public var id: String {
        "\(txid.map { String(format: "%02x", $0) }.joined()):\(vout)"
    }

    /// Whether this UTXO has at least 6 confirmations
    public var isConfirmed: Bool {
        confirmations >= 6
    }

    /// Whether this UTXO can be spent (requires 6 confirmations)
    public var isSpendable: Bool {
        isConfirmed
    }

    /// Public initializer for UTXO (for creating from app data)
    public init(
        txid: Data,
        vout: UInt32,
        amount: UInt64,
        address: String,
        scriptPubKey: Data,
        height: UInt32 = 0,
        confirmations: UInt32 = 0
    ) {
        self.txid = txid
        self.vout = vout
        self.amount = amount
        self.address = address
        self.scriptPubKey = scriptPubKey
        self.height = height
        self.confirmations = confirmations
    }
}
