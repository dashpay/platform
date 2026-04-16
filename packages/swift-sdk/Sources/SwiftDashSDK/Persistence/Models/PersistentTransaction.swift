import Foundation
import SwiftData

/// SwiftData model for persisting a wallet transaction.
///
/// Stores the full transaction record including context (mempool,
/// confirmed, chain-locked), direction, amounts, and fee.
@Model
public final class PersistentTransaction {
    /// Transaction ID (32-byte hash, stored as hex for indexing).
    @Attribute(.unique) public var txid: String
    /// Raw transaction bytes.
    public var transactionData: Data?
    /// Context: 0=mempool, 1=instantSend, 2=inBlock, 3=inChainLockedBlock.
    public var context: UInt32
    /// Block height (0 for mempool).
    public var blockHeight: UInt32
    /// Block hash (nil for mempool).
    public var blockHash: Data?
    /// Block timestamp.
    public var blockTimestamp: UInt32
    /// Direction: 0=incoming, 1=outgoing, 2=internal, 3=coinJoin.
    public var direction: UInt32
    /// Transaction type name (Standard, CoinJoin, etc.).
    public var transactionType: String
    /// Net amount in duffs (signed: positive=received, negative=sent).
    public var netAmount: Int64
    /// Fee in duffs (nil if unknown).
    public var fee: UInt64?
    /// User-assigned label.
    public var label: String
    /// Timestamp when first observed (Unix seconds).
    public var firstSeen: UInt64
    /// Record timestamps.
    public var createdAt: Date
    public var lastUpdated: Date

    /// Parent account.
    public var account: PersistentAccount?

    public init(
        txid: String,
        context: UInt32 = 0,
        blockHeight: UInt32 = 0,
        direction: UInt32 = 0,
        transactionType: String = "Standard",
        netAmount: Int64 = 0,
        firstSeen: UInt64 = 0
    ) {
        self.txid = txid
        self.context = context
        self.blockHeight = blockHeight
        self.blockTimestamp = 0
        self.direction = direction
        self.transactionType = transactionType
        self.netAmount = netAmount
        self.firstSeen = firstSeen
        self.label = ""
        self.createdAt = Date()
        self.lastUpdated = Date()
    }

    // MARK: - Display Helpers

    public var contextName: String {
        switch context {
        case 0: return "Mempool"
        case 1: return "InstantSend"
        case 2: return "In Block"
        case 3: return "Chain Locked"
        default: return "Unknown"
        }
    }

    public var directionName: String {
        switch direction {
        case 0: return "Incoming"
        case 1: return "Outgoing"
        case 2: return "Internal"
        case 3: return "CoinJoin"
        default: return "Unknown"
        }
    }

    public var formattedAmount: String {
        let dash = Double(abs(netAmount)) / 100_000_000.0
        let sign = netAmount >= 0 ? "+" : "-"
        return String(format: "%@%.8f DASH", sign, dash)
    }
}
