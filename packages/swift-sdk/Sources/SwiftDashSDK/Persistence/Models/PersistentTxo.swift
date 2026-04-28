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
    /// `PersistentCoreAddress.txos`; `.nullify` on that side so
    /// pool rebuilds don't cascade-delete TXOs.
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
    /// as raw 32-byte `Data`. Returns empty `Data()` if the
    /// relationship isn't attached (which should only happen
    /// briefly during construction).
    public var txid: Data {
        transaction?.txid ?? Data()
    }

    /// Hex-encoded txid for UI / log sites.
    public var txidHex: String {
        txid.map { String(format: "%02x", $0) }.joined()
    }

    /// Human-readable outpoint (`<txid hex>:<vout>`) for UI / log
    /// sites. Reconstructs from the parent transaction's txid plus
    /// `self.vout` rather than re-decoding the stored 36-byte blob,
    /// which avoids one allocation and matches the legacy display
    /// format.
    public var outpointHex: String {
        "\(txidHex):\(vout)"
    }

    public var formattedAmount: String {
        let dash = Double(amount) / 100_000_000.0
        return String(format: "%.8f DASH", dash)
    }
}
