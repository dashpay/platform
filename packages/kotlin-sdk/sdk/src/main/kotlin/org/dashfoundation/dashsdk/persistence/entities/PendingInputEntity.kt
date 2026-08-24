package org.dashfoundation.dashsdk.persistence.entities

import androidx.room.ColumnInfo
import androidx.room.Entity
import androidx.room.ForeignKey
import androidx.room.Index
import androidx.room.PrimaryKey
import java.util.Date

/**
 * Port of `PersistentPendingInput.swift` — side-table row tracking a
 * transaction input whose previous-output TXO hasn't landed yet.
 *
 * Deliberately NO unique constraint on [outpoint] (per the Swift doc: a
 * re-org / double-spend could produce two pending rows for one outpoint
 * and both should resolve naturally) — hence a surrogate rowid PK,
 * mirroring SwiftData's hidden `persistentModelID`.
 *
 * Swift `#Index([\.outpoint], [\.walletId])` → the two indices below.
 *
 * [spendingTransactionTxid] materializes the optional
 * `spendingTransaction` relationship (CASCADE per
 * `PersistentTransaction.pendingInputs` `.cascade`); the non-null
 * [spendingTxid] scalar mirrors Swift's separate denorm that survives when
 * the parent row isn't present yet.
 */
@Entity(
    tableName = "pending_inputs",
    indices = [
        Index(value = ["outpoint"]),
        Index(value = ["walletId"]),
        Index(value = ["spendingTransactionTxid"]),
    ],
    foreignKeys = [
        ForeignKey(
            entity = TransactionEntity::class,
            parentColumns = ["txid"],
            childColumns = ["spendingTransactionTxid"],
            onDelete = ForeignKey.CASCADE,
        ),
    ],
)
data class PendingInputEntity(
    /** Surrogate rowid — Swift has no unique attribute on this model. */
    @PrimaryKey(autoGenerate = true) val id: Long = 0,
    /** 36-byte outpoint (txid + vout LE) — join key against `txos.outpoint`. */
    val outpoint: ByteArray,
    /** Position of this input in the spending tx. Swift `UInt32` → [Int]. */
    val inputIndex: Int,
    /** 32-byte txid of the spending transaction (denorm, always set). */
    val spendingTxid: ByteArray,
    /** FK materialization of the Swift `spendingTransaction` relationship. */
    val spendingTransactionTxid: ByteArray? = null,
    /** Wallet id denorm for cleanup / per-wallet diagnostics. */
    val walletId: ByteArray,
    val createdAt: Date = Date(),
    /**
     * Port of Swift `PersistentPendingInput.isSweptTombstone`. Set by
     * `onWalletChangesetTransactionsSwept` when this row's spend turns out
     * to belong to a swept loser and the input wasn't in `released`:
     * [spendingTransactionTxid] is cleared (detaching the FK so the row
     * survives the loser's cascade-delete) and [spendingTxid] is
     * overwritten with the winner's txid. `onWalletChangesetUtxoAdded`
     * checks this flag when it later drains the row — a tombstone forces
     * `TxoEntity.isSpent = true` unconditionally (a sweep's winner is
     * already final, unlike an ordinary pending spend whose confirmation is
     * still pending) and stamps `TxoEntity.supersededByTxid` so the mark
     * survives even when the winner's own `transactions` row never
     * materializes. Defaulted `false` so pre-migration rows read as
     * ordinary pending entries.
     *
     * Declares its default so the exported schema agrees with what
     * `MIGRATION_10_11` writes: SQLite requires one on a NOT NULL
     * `ADD COLUMN`, and Room compares defaults when validating a migrated
     * database against the entity — a mismatch fails the upgrade outright.
     */
    @ColumnInfo(defaultValue = "0")
    val isSweptTombstone: Boolean = false,
    /**
     * The mined block height of the WINNER that swept this tombstone's
     * loser — the winner's own height, carried on the sweep event itself,
     * not any observation watermark. This stamp is the row's whole
     * lifetime rule: the collector deletes the tombstone once the
     * chainlock finality boundary `min(chainlockHeight, syncedHeight)`
     * reaches it — key-wallet's `prune_finalized_observed_spends`
     * condition verbatim, no observation-age margin — because at that
     * boundary the funding transaction (necessarily mined at or below
     * the winner's height) has been filter-scanned with no false
     * negatives, so an undrained row is provably not the wallet's coin.
     * A genuine claim drains into its TXO on funding arrival and leaves
     * the collectible set with the row.
     *
     * NULL is never collected. A mempool/IS-context sweep (unmined
     * winner) writes its tombstone unstamped on purpose: under DIP-10
     * the IS lock alone settles the input, but the winner has no mining
     * deadline, so no boundary can ever prove its funding output
     * delivered-or-never — the hold lasts until the funding TXO drains
     * it, a later block-context sweep stamps it, or a release deletes
     * it. An IS-locked re-point likewise keeps the existing stamp.
     * Nullable, so the ADD COLUMN migration needs no default and
     * pre-migration rows read as unstamped.
     */
    val winnerMinedHeight: Int? = null,
)
