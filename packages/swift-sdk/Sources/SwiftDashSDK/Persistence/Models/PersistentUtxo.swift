import Foundation
import SwiftData

/// SwiftData model for persisting an unspent transaction output.
///
/// Represents a single UTXO that can be spent by the wallet.
/// Linked to its parent account for cascade deletion.
@Model
public final class PersistentUtxo {
    /// Outpoint: txid hex + ":" + vout index (unique identifier).
    @Attribute(.unique) public var outpoint: String
    /// Transaction ID (32-byte hash as hex).
    public var txid: String
    /// Output index within the transaction.
    public var vout: UInt32
    /// Value in duffs.
    public var amount: UInt64
    /// Owning address (Base58Check).
    public var address: String
    /// Script pubkey bytes.
    public var scriptPubKey: Data
    /// Block height where created.
    public var height: UInt32
    /// Whether this is a coinbase output.
    public var isCoinbase: Bool
    /// Whether confirmed in a block.
    public var isConfirmed: Bool
    /// Whether locked by InstantSend.
    public var isInstantLocked: Bool
    /// Whether reserved/locked for a specific purpose.
    public var isLocked: Bool
    /// Whether this UTXO has been spent.
    public var isSpent: Bool
    /// Record timestamps.
    public var createdAt: Date
    public var lastUpdated: Date

    /// Parent account.
    public var account: PersistentAccount?

    public init(
        txid: String,
        vout: UInt32,
        amount: UInt64,
        address: String,
        scriptPubKey: Data = Data(),
        height: UInt32 = 0
    ) {
        self.outpoint = "\(txid):\(vout)"
        self.txid = txid
        self.vout = vout
        self.amount = amount
        self.address = address
        self.scriptPubKey = scriptPubKey
        self.height = height
        self.isCoinbase = false
        self.isConfirmed = false
        self.isInstantLocked = false
        self.isLocked = false
        self.isSpent = false
        self.createdAt = Date()
        self.lastUpdated = Date()
    }

    public var formattedAmount: String {
        let dash = Double(amount) / 100_000_000.0
        return String(format: "%.8f DASH", dash)
    }
}
