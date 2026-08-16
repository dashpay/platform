import Foundation
import SwiftData

/// SwiftData model for persisting a transaction output (spent or
/// unspent).
///
/// Each row represents a single TXO produced by some `PersistentTransaction`
/// (`transaction`). When the TXO is later spent, the spending tx is
/// linked via `spendingTransaction` and `isSpent` flips to `true` —
/// the row is kept (rather than deleted) so the wallet's history
/// stays whole.
@Model
public final class PersistentTxo {
    /// Index `walletId` so per-wallet TXO scans — the canonical
    /// "show every TXO (and, by union of `transaction` +
    /// `spendingTransaction`, every transaction) that touches wallet
    /// W" path — hit an index instead of scanning the entire TXO
    /// table. The denorm is what makes the predicate translatable
    /// to SQL in the first place; this just makes the resulting
    /// query fast at scale.
    #Index<PersistentTxo>([\.walletId])

    /// Outpoint: 36 raw bytes (32-byte txid in wire orientation +
    /// 4-byte vout little-endian) — the standard Bitcoin outpoint
    /// serialization. Unique identifier stored explicitly so
    /// SwiftData predicate fetches can hit a single column without
    /// traversing the `transaction` relationship. Always equals
    /// `PersistentTxo.makeOutpoint(txid: transaction.txid, vout: vout)`.
    @Attribute(.unique) public var outpoint: Data
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
    /// Whether this TXO has been spent.
    ///
    /// Denormalized: should track `spendingTransaction != nil`. Kept
    /// as an explicit column because per-row spent/unspent filters
    /// are a hot query path, and chasing the optional relationship
    /// in a predicate drops SwiftData onto the same nested-optional
    /// codepath that crashes elsewhere. The persistence handler is
    /// responsible for keeping the two in sync; do not enforce
    /// invariants here.
    public var isSpent: Bool
    /// Record timestamps.
    public var createdAt: Date
    public var lastUpdated: Date

    /// 32-byte wallet ID this TXO belongs to. Denormalized from
    /// `account?.wallet.walletId` so per-wallet `@Query` predicates
    /// can filter with a single equality check instead of chaining
    /// through the optional `account` relationship — SwiftData's
    /// predicate compiler can't translate that chain into SQLite and
    /// crashes with `Unsupported function expression TERNARY(...).walletId`.
    /// This is the single column callers filter on for "show every
    /// TXO (and, by union of `transaction` + `spendingTransaction`,
    /// every transaction) that touches wallet W". Empty `Data()` for
    /// rows migrated from older schema; the next sync pass will
    /// populate it.
    public var walletId: Data = Data()

    /// Containing transaction (the one that *created* this output).
    /// Cascade-deleted from the parent side (see
    /// `PersistentTransaction.outputs`). Optional only because the
    /// underlying SwiftData inverse must allow nil during the brief
    /// window between row insert and relationship attachment; in
    /// steady state every TXO has a non-nil `transaction`.
    public var transaction: PersistentTransaction?

    /// The transaction that *spent* this output, or nil if the TXO
    /// is unspent. Inverse of `PersistentTransaction.inputs`. Uses
    /// the default `.nullify` delete rule from that side — deleting
    /// the spending tx must not cascade-delete this row.
    public var spendingTransaction: PersistentTransaction?

    /// 32-byte txid of the transaction a sweep's winner is known to have
    /// beaten this coin to, set only by `upsertUtxo` resolving a
    /// `PersistentPendingInput` tombstone (`isSweptTombstone`) — i.e. this
    /// TXO's funding output arrived after its loser was already swept and
    /// deleted, so there was never a `spendingTransaction` row to link.
    /// `nil` in every other case, including the ordinary "sweep held this
    /// coin with no spender on record" state that `applySweptTransaction`
    /// writes directly onto an already-materialized row (`spendingTransaction
    /// = nil`, no tombstone involved). That distinction is what
    /// `upsertUtxo`'s recovery clear keys on: a coin the wallet re-delivers
    /// as unspent lifts `isSpent` only when both `spendingTransaction` and
    /// this are nil, so a tombstoned coin isn't waved back into the restore
    /// set just because nobody ever linked the winner's own row.
    public var supersededByTxid: Data?

    /// Position of this output within `spendingTransaction.input`
    /// (i.e. the canonical "vin index"). Captured at the moment the
    /// spend is reconciled — sourced from
    /// `TransactionRecordFFI.input_outpoints` index, which itself
    /// comes from `tx.input.iter()` on the Rust side, so the value
    /// matches the serialized transaction's input ordering exactly.
    /// `nil` when the TXO is unspent (no spending tx, no vin index)
    /// or when migrated from an older row that predates the column.
    /// Surfaced by `TransactionStorageDetailView` so input rows
    /// render in serialized vin order with their real positions
    /// rather than being re-sorted by outpoint hex (which loses
    /// the relationship between row and serialized index).
    public var spendingInputIndex: UInt32? = nil

    /// Parent account. No longer paired with an inverse on the
    /// account side — the canonical account path is
    /// `coreAddress?.account`. This field is the fallback when the
    /// address row isn't yet linked (out-of-order flush, address
    /// pool rebuild, etc.).
    public var account: PersistentAccount?

    /// Owning `PersistentCoreAddress` row, if it exists in the
    /// account's address pool. Linked alongside `address` (the
    /// Base58Check string) — the string is the authoritative
    /// identifier and survives even when the address pool is rebuilt
    /// or the TXO was paid to an address never in our pool (e.g. an
    /// outgoing recipient). The relationship is the convenient
    /// pointer for navigating to derivation metadata, balance, and
    /// pool tag without a separate fetch. Inverse of
    /// `PersistentCoreAddress.txos`; `.cascade` on that side so
    /// account / wallet teardown drops TXOs cleanly.
    public var coreAddress: PersistentCoreAddress?

    public init(
        transaction: PersistentTransaction,
        vout: UInt32,
        amount: UInt64,
        address: String,
        scriptPubKey: Data = Data(),
        height: UInt32 = 0
    ) {
        self.outpoint = Self.makeOutpoint(txid: transaction.txid, vout: vout)
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
        self.transaction = transaction
    }

    /// Build the 36-byte outpoint key (32-byte txid raw bytes +
    /// 4-byte vout little-endian). Exposed so the persistence
    /// handler can compose predicates / lookups directly from the
    /// FFI's `[u8; 32]` + `u32` without going through string
    /// formatting.
    public static func makeOutpoint(txid: Data, vout: UInt32) -> Data {
        var data = Data(capacity: 36)
        data.append(txid)
        var v = vout.littleEndian
        withUnsafeBytes(of: &v) { data.append(contentsOf: $0) }
        return data
    }

    /// Convenience accessor for the containing transaction's txid
    /// as raw 32-byte `Data`. Prefers the `transaction` relationship;
    /// falls back to the first 32 bytes of `outpoint` when the
    /// inverse is briefly nil during insert (so storage-explorer
    /// rows still render a stable identifier rather than collapsing
    /// to empty).
    public var txid: Data {
        if let transaction {
            return transaction.txid
        }
        return outpoint.count >= 32 ? Data(outpoint.prefix(32)) : Data()
    }

    /// Hex-encoded txid for UI / log sites. Reverses bytes to match
    /// the canonical block-explorer display (same flip as
    /// `dashcore::Txid: Display`). Mirrors
    /// `PersistentTransaction.txidHex` directly so the two stay in
    /// sync; can't simply forward to it because we want the same
    /// hex even when `transaction` is briefly unattached.
    public var txidHex: String {
        let rawTxid = txid
        guard rawTxid.count == 32 else { return "" }
        return rawTxid.reversed().map { String(format: "%02x", $0) }.joined()
    }

    /// Human-readable outpoint (`<txid hex>:<vout>`) for UI / log
    /// sites. Reconstructs from `txidHex` so the byte-flip stays
    /// consistent across all display surfaces.
    public var outpointHex: String {
        let hex = txidHex
        return hex.isEmpty ? "" : "\(hex):\(vout)"
    }

    public var formattedAmount: String {
        let dash = Double(amount) / 100_000_000.0
        return String(format: "%.8f DASH", dash)
    }
}
