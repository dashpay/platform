import Foundation
import SwiftData

/// Side-table row tracking a transaction input whose previous-output
/// `PersistentTxo` hasn't landed in SwiftData yet.
///
/// The persistence pipeline can deliver a spending transaction
/// before its funding transaction in two distinct ways:
///
/// 1. **In-Swift out-of-order**: the FFI flushes the spending tx's
///    record to `upsertTransaction(...)` ahead of the funding tx's
///    output to `upsertUtxo(...)`. By the time we walk the spending
///    tx's inputs, no `PersistentTxo` exists yet for the previous
///    output, so a naive "fetch and mark spent" pass silently drops
///    the linkage.
/// 2. **Mid-sync restart**: a previous session persisted the
///    funding TXO but the live sync's `self.utxos` map didn't get
///    repopulated by `load_from_persistor`, so the Rust side never
///    classifies the input as "ours" and the FFI's
///    `utxos_spent` slice is empty for that outpoint. The funding
///    `PersistentTxo` exists on disk but never gets marked.
///
/// To handle both, every spending tx that arrives writes a
/// `PersistentPendingInput` row for each of its inputs whose
/// previous-output we don't yet have a `PersistentTxo` for. When a
/// later `upsertUtxo` creates the matching row, it consults this
/// table; finding a hit lets it set `isSpent = true`, link
/// `spendingTransaction = pending.spendingTransaction`, and delete
/// the pending row in one pass. The mechanism is order-symmetric:
/// the spend signal travels through the side table when arrival
/// order doesn't match dependency order.
///
/// The 36-byte `outpoint` (32-byte txid + 4-byte LE u32 vout —
/// composed via [`PersistentTxo.makeOutpoint(txid:vout:)`]) is the
/// join key between this table and `PersistentTxo`. We index it but
/// don't mark it `unique` — a re-org or double-spend scenario could
/// in principle produce two pending rows for the same outpoint,
/// and we'd rather both resolve naturally than reject the second
/// upsert outright.
@Model
public final class PersistentPendingInput {
    /// Two single-column indexes:
    ///   * `outpoint` — the per-outpoint reconciliation lookup that
    ///     runs on every `upsertUtxo`.
    ///   * `walletId` — per-wallet pending-input scans (cleanup when
    ///     a wallet is removed, the storage explorer's network
    ///     scope, "long-lived non-zero pending count" diagnostics).
    ///
    /// SwiftData allows only a single `#Index` macro per model;
    /// passing multiple key-path arrays declares multiple separate
    /// indexes from one macro call.
    #Index<PersistentPendingInput>([\.outpoint], [\.walletId])
    public var outpoint: Data

    /// Position of this input in the spending transaction's input
    /// list. Carried so a future UI surface can render the input
    /// index correctly without re-deriving from the raw tx bytes;
    /// the resolution flow itself only uses `outpoint`.
    public var inputIndex: UInt32

    /// 32-byte canonical txid of the spending transaction. Stored
    /// in addition to the relationship below so the entry remains
    /// usable if the parent `PersistentTransaction` isn't yet in the
    /// background context (re-upsert ordering, fault-in lag, …).
    public var spendingTxid: Data

    /// The transaction this input belongs to. Cascade-deleted from
    /// the parent side via `PersistentTransaction.pendingInputs` so
    /// removing a tx doesn't leave dangling pending rows.
    public var spendingTransaction: PersistentTransaction?

    /// Wallet id (`PersistentTxo.walletId` denorm) so cleanup /
    /// per-wallet diagnostics can scope without joining through the
    /// transaction relationship.
    public var walletId: Data

    /// Insertion timestamp — useful for spotting stale entries that
    /// never resolved (orphans whose previous output isn't ours).
    public var createdAt: Date

    /// Set when `applySweptTransaction` repurposes this row as a durable
    /// claim rather than an ordinary in-flight spend: the original
    /// spending transaction turned out to be a loser, this input wasn't in
    /// `released`, and the funding `PersistentTxo` still hasn't arrived to
    /// hold the claim itself. `spendingTxid` is overwritten to the winner
    /// (`superseded_by`) and `spendingTransaction` is detached so the row
    /// survives the loser's cascade-delete. `upsertUtxo` checks this flag
    /// on resolve: a tombstone forces `PersistentTxo.isSpent = true`
    /// unconditionally (a sweep's winner is already final, unlike an
    /// ordinary pending spend whose confirmation is still pending) and
    /// stamps `PersistentTxo.supersededByTxid` so the mark survives even
    /// when the winner's own row never materializes. Defaulted `false` so
    /// existing rows migrate as ordinary pending entries.
    public var isSweptTombstone: Bool = false

    /// Creation stamp of a swept tombstone: the wallet's `syncedHeight` at
    /// the round that flagged (or re-pointed) this row. A tombstone whose
    /// outpoint is a foreign input of a swept incoming payment never
    /// drains — no funding TXO ever arrives — so without a bound it is
    /// permanent junk an attacker grows one row per input by repeatedly
    /// double-spending payments at this wallet. The changeset-header
    /// collector (`collectFinalizedSweptTombstones`) deletes tombstones
    /// once `syncedHeight` clears this stamp by the sweep margin — the
    /// storage mirror of key-wallet's `prune_finalized_observed_spends`
    /// doctrine; a genuine claim drains into its TXO on funding arrival
    /// and leaves the collectible set with the row. `nil` means "flagged
    /// before this property existed, or with no synced height on record"
    /// — the collector back-fills it with the current height rather than
    /// guessing, so such rows wait a full margin from first sight.
    /// Optional, so existing stores lightweight-migrate with the rows
    /// reading unstamped.
    public var heldSinceHeight: UInt32?

    public init(
        outpoint: Data,
        inputIndex: UInt32,
        spendingTxid: Data,
        spendingTransaction: PersistentTransaction?,
        walletId: Data
    ) {
        self.outpoint = outpoint
        self.inputIndex = inputIndex
        self.spendingTxid = spendingTxid
        self.spendingTransaction = spendingTransaction
        self.walletId = walletId
        self.createdAt = Date()
    }
}
