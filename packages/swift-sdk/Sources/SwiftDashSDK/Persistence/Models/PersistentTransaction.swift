import Foundation
import SwiftData

/// SwiftData model for persisting a wallet transaction.
///
/// Stores the full transaction record including context (mempool,
/// confirmed, chain-locked), direction, amounts, and fee.
///
/// A transaction is intentionally **not** scoped to a single wallet
/// or account. The same on-chain tx can have outputs into account A
/// and account B inside one wallet (or — for cross-wallet flows —
/// into accounts under different wallets), and the transaction row
/// is shared across all of them. Per-wallet membership is recovered
/// by joining through the TXOs (`outputs` + `inputs`); see
/// `PersistentTxo.walletId` for the per-row denorm that makes those
/// joins index-friendly.
@Model
public final class PersistentTransaction {
    /// Index on `firstSeen` so per-wallet queries — which fetch
    /// `PersistentTxo` rows by `walletId` then sort their parent
    /// transactions by `firstSeen` — get a sorted scan instead of
    /// an in-memory O(N log N) pass. The unique `txid` index covers
    /// point-lookups; this one covers the timeline.
    #Index<PersistentTransaction>([\.firstSeen])

    /// Transaction ID (32-byte hash, raw little-endian wire bytes —
    /// the same orientation Rust hands us via the FFI `[u8; 32]`).
    /// Stored as raw `Data` so the unique index covers 32 bytes
    /// instead of a 64-char hex string, and the persistence
    /// handler avoids a hex round-trip on every write.
    @Attribute(.unique) public var txid: Data
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

    /// Transaction outputs created by this transaction.
    ///
    /// Cascade-deletes the matching `PersistentTxo` rows when the
    /// transaction is removed — outputs cannot meaningfully exist
    /// without their containing transaction (the outpoint, script,
    /// amount, and address are all derived from it).
    @Relationship(deleteRule: .cascade, inverse: \PersistentTxo.transaction)
    public var outputs: [PersistentTxo] = []

    /// Transaction outputs spent *by* this transaction.
    ///
    /// Inverse of `PersistentTxo.spendingTransaction`. Default
    /// `.nullify` delete rule (do not pass `.cascade`!) — those TXOs
    /// are owned by their *creating* transaction, not this one.
    /// Cascading from the spending side would let a recent tx wipe
    /// outputs of an older tx on delete: a data-loss bug. Removing
    /// this transaction merely detaches the spend-link and the TXOs
    /// flip back to "unspent" until something else claims them.
    @Relationship(inverse: \PersistentTxo.spendingTransaction)
    public var inputs: [PersistentTxo] = []

    /// Pending input outpoints — entries this transaction's input
    /// list references but for which no `PersistentTxo` has been
    /// upserted yet. Filled by `PlatformWalletPersistenceHandler.
    /// upsertTransaction` via the FFI's `input_outpoints` slice;
    /// each entry is consumed (deleted) by `upsertUtxo` when the
    /// matching previous-output finally arrives. See
    /// `PersistentPendingInput` for the full reconciliation flow.
    /// Cascade-delete: removing the spending tx drops every pending
    /// row that hasn't resolved yet.
    @Relationship(deleteRule: .cascade, inverse: \PersistentPendingInput.spendingTransaction)
    public var pendingInputs: [PersistentPendingInput] = []

    public init(
        txid: Data,
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

    /// Hex-encoded txid for UI / log sites. The on-disk row stores
    /// the raw 32 bytes in wire/internal order (matches what
    /// `dashcore::Txid::as_ref()` hands the FFI). The canonical
    /// Bitcoin/Dash display convention is the *reverse* of those
    /// bytes (the `Txid: Display` impl in dashcore-rust does the
    /// same flip), so block-explorer hex matches what users see
    /// here. Storage stays unflipped — predicate fetches compare
    /// wire-order `Data` directly without re-encoding.
    public var txidHex: String {
        txid.reversed().map { String(format: "%02x", $0) }.joined()
    }

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
