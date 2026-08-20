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
     * Creation stamp of a swept tombstone: the wallet's `syncedHeight` at
     * the round that flagged (or re-pointed) this row. A tombstone whose
     * outpoint is a foreign input of a swept incoming payment never
     * drains — no funding TXO ever arrives — so without a bound it is
     * permanent junk an attacker grows one row per input by repeatedly
     * double-spending payments at this wallet. The header callback's
     * collector deletes tombstones once `syncedHeight` clears this stamp
     * by the sweep margin (the storage mirror of key-wallet's
     * `prune_finalized_observed_spends` doctrine); a genuine claim drains
     * into its TXO on funding arrival and leaves the collectible set with
     * the row. NULL means "flagged before this column existed, or with no
     * synced height on record" — the collector back-fills it with the
     * current height rather than guessing, so such rows wait a full
     * margin from first sight. Nullable, so the ADD COLUMN migration
     * needs no default and pre-migration rows read as unstamped.
     */
    val heldSinceHeight: Int? = null,
)
