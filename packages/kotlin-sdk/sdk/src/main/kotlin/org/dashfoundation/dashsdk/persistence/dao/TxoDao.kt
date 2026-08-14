package org.dashfoundation.dashsdk.persistence.dao

import androidx.room.Dao
import androidx.room.Delete
import androidx.room.Query
import androidx.room.Upsert
import kotlinx.coroutines.flow.Flow
import org.dashfoundation.dashsdk.persistence.entities.TxoEntity

/**
 * Queries over [TxoEntity], mirroring the Swift call sites:
 * per-wallet scans (`$0.walletId == walletId` — TransactionListView,
 * WalletMemoryExplorer, deleteWalletData), unspent filters
 * (`$0.isSpent == false`), and outpoint point-lookups from the
 * persistence handler's upsert / spend-reconciliation paths.
 */
@Dao
interface TxoDao {

    @Query("SELECT * FROM txos WHERE walletId = :walletId")
    fun observeByWallet(walletId: ByteArray): Flow<List<TxoEntity>>

    /** WalletMemoryExplorer: `txo.walletId == walletId && txo.isSpent == false`. */
    @Query("SELECT * FROM txos WHERE walletId = :walletId AND isSpent = 0")
    fun observeUnspentByWallet(walletId: ByteArray): Flow<List<TxoEntity>>

    /** Handler's global unspent scan (`$0.isSpent == false`). */
    @Query("SELECT * FROM txos WHERE isSpent = 0")
    fun observeUnspent(): Flow<List<TxoEntity>>

    @Query("SELECT * FROM txos WHERE outpoint = :outpoint")
    fun observeByOutpoint(outpoint: ByteArray): Flow<TxoEntity?>

    @Query("SELECT * FROM txos WHERE outpoint = :outpoint")
    suspend fun getByOutpoint(outpoint: ByteArray): TxoEntity?

    /**
     * TXOs linked to a spending transaction but not yet marked spent —
     * the spend-flip reconcile set for [spendingTxid]'s confirmation
     * (the tx-upsert pass that flips `isSpent` once the spend is
     * in-block; see `onWalletChangesetTransaction`).
     */
    @Query("SELECT * FROM txos WHERE spendingTxid = :spendingTxid AND isSpent = 0")
    suspend fun getUnspentBySpendingTxid(spendingTxid: ByteArray): List<TxoEntity>

    /**
     * Hold every coin [spendingTxid] claimed out of the restore set, without
     * naming a spender for them.
     *
     * Used when [spendingTxid] was swept: it can never confirm, so its claim
     * is not a spend, but most of the coins it named really were taken — by
     * the transaction that beat it. A swept transaction is always
     * unconfirmed, so its inputs sit at `isSpent = 0`, and deleting it would
     * otherwise return all of them, the consumed one included.
     *
     * Run this *before* deleting the transaction, while the link that
     * identifies those rows is still there — the foreign key nulls
     * `spendingTxid` on delete, and afterwards nothing finds them. Then
     * clear the genuinely free ones with [releaseByOutpoint].
     */
    @Query("UPDATE txos SET isSpent = 1, spendingTxid = NULL, spendingInputIndex = NULL WHERE spendingTxid = :spendingTxid")
    suspend fun holdSpentWithoutSpender(spendingTxid: ByteArray)

    /**
     * Mark one outpoint unspent again — a coin a sweep released, meaning no
     * surviving transaction spent it *at the time the sweep was computed*.
     *
     * Keyed by outpoint rather than by spender because that is how upstream
     * reports it: the transaction that took the other inputs may never be
     * recorded here at all, so the released set is the only authority on
     * which coins came free.
     *
     * `spendingTxid IS NULL` is what keeps that from overreaching. A round
     * can carry both a release and a later transaction that legitimately
     * spends the freed coin — merging folds several events together, and
     * every record is written before sweeps are processed — so by the time
     * this runs the coin may already be claimed again. Only rows
     * [holdSpentWithoutSpender] just detached qualify; anything a live
     * transaction still claims keeps that claim.
     */
    @Query(
        "UPDATE txos SET isSpent = 0, spendingInputIndex = NULL " +
            "WHERE outpoint = :outpoint AND spendingTxid IS NULL",
    )
    suspend fun releaseByOutpoint(outpoint: ByteArray)

    @Upsert
    suspend fun upsert(txo: TxoEntity)

    @Delete
    suspend fun delete(txo: TxoEntity)

    /** Wallet teardown mirror of `deleteWalletData`'s TXO pass. */
    @Query("DELETE FROM txos WHERE walletId = :walletId")
    suspend fun deleteByWallet(walletId: ByteArray)

    @Query("DELETE FROM txos")
    suspend fun deleteAll()

    /** StorageExplorer row count. */
    @Query("SELECT COUNT(*) FROM txos")
    fun count(): Flow<Long>

    @Query("SELECT COUNT(*) FROM txos WHERE walletId = :walletId")
    fun countByWallet(walletId: ByteArray): Flow<Long>
}
