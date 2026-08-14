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
     * Release the spend claim [spendingTxid] still holds — used when that
     * transaction was swept and the coins it named are genuinely free.
     *
     * The `spendingTxid` foreign key already nulls itself when the spending
     * row is deleted, but `isSpent` is a plain column and would survive,
     * leaving a coin marked spent by a transaction that no longer exists.
     * Run this *before* deleting the transaction, while the link that
     * identifies those rows is still there.
     *
     * Only rows still pointing at [spendingTxid] are touched, which is what
     * makes this safe for a sweep: the winner has already re-pointed the
     * inputs it took at itself, so what remains is the loser's own.
     */
    @Query("UPDATE txos SET isSpent = 0, spendingTxid = NULL, spendingInputIndex = NULL WHERE spendingTxid = :spendingTxid")
    suspend fun releaseSpendClaim(spendingTxid: ByteArray)

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
