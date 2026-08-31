package org.dashfoundation.dashsdk.persistence.dao

import androidx.room.Dao
import androidx.room.Delete
import androidx.room.Query
import androidx.room.Upsert
import kotlinx.coroutines.flow.Flow
import org.dashfoundation.dashsdk.persistence.entities.TxoEntity
import java.util.Date

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
     * Flip `isSpent` on every still-unspent TXO consumed by
     * [spendingTxid] — the heal a finalized asset lock drives when SPV
     * block matching missed its spender and the ordinary in-block flip
     * never ran (see `onPersistAssetLockUpsert`).
     *
     * Column-scoped and conditioned on `isSpent = 0`: it cannot regress
     * an already-spent row, and unlike a read-then-[upsert] round trip it
     * never writes back a stale copy of the columns it does not own.
     * Promote-only and idempotent — a second run matches no rows. Returns
     * the number of rows healed.
     */
    @Query(
        "UPDATE txos SET isSpent = 1, lastUpdated = :now " +
            "WHERE spendingTxid = :spendingTxid AND isSpent = 0",
    )
    suspend fun markSpentBySpendingTxid(spendingTxid: ByteArray, now: Date): Int

    /** Single-row [markSpentBySpendingTxid], keyed by the TXO's own outpoint. */
    @Query(
        "UPDATE txos SET isSpent = 1, lastUpdated = :now " +
            "WHERE outpoint = :outpoint AND isSpent = 0",
    )
    suspend fun markSpentByOutpoint(outpoint: ByteArray, now: Date): Int

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
