package org.dashfoundation.dashsdk.persistence.dao

import androidx.room.Dao
import androidx.room.Delete
import androidx.room.Query
import androidx.room.Upsert
import kotlinx.coroutines.flow.Flow
import org.dashfoundation.dashsdk.persistence.entities.TransactionEntity

/**
 * Queries over [TransactionEntity]. Point lookups by txid mirror the
 * persister's `#Predicate { $0.txid == txidData }`; the timeline join
 * (per-wallet TXOs → parent transactions sorted by `firstSeen`, see
 * `TransactionListView.swift`) is served by [observeByTxids] +
 * `TxoDao.observeByWallet`.
 */
@Dao
interface TransactionDao {

    @Query("SELECT * FROM transactions ORDER BY firstSeen DESC")
    fun observeAll(): Flow<List<TransactionEntity>>

    @Query("SELECT * FROM transactions WHERE txid = :txid")
    fun observeByTxid(txid: ByteArray): Flow<TransactionEntity?>

    @Query("SELECT * FROM transactions WHERE txid = :txid")
    suspend fun getByTxid(txid: ByteArray): TransactionEntity?

    /** Timeline join helper — parents of a wallet's TXO set. */
    @Query("SELECT * FROM transactions WHERE txid IN (:txids) ORDER BY firstSeen DESC")
    fun observeByTxids(txids: List<ByteArray>): Flow<List<TransactionEntity>>

    @Upsert
    suspend fun upsert(transaction: TransactionEntity)

    @Delete
    suspend fun delete(transaction: TransactionEntity)

    @Query("DELETE FROM transactions WHERE txid = :txid")
    suspend fun deleteByTxid(txid: ByteArray)

    /**
     * Orphan sweep run after a wallet wipe (Swift `deleteWalletData`'s
     * post-delete pass): drop transactions no longer referenced by any
     * TXO (as creating `txid` or spending `spendingTxid`) or pending
     * input. Transactions are not wallet-scoped — the same on-chain tx can
     * land in several wallets — so this deletes only rows nothing points at
     * anymore, never a tx a surviving wallet still references.
     */
    @Query(
        "DELETE FROM transactions WHERE txid NOT IN " +
            "(SELECT txid FROM txos WHERE txid IS NOT NULL) " +
            "AND txid NOT IN (SELECT spendingTxid FROM txos WHERE spendingTxid IS NOT NULL) " +
            "AND txid NOT IN " +
            "(SELECT spendingTransactionTxid FROM pending_inputs " +
            "WHERE spendingTransactionTxid IS NOT NULL)"
    )
    suspend fun deleteOrphanTransactions()

    @Query("DELETE FROM transactions")
    suspend fun deleteAll()

    /** StorageExplorer row count. */
    @Query("SELECT COUNT(*) FROM transactions")
    fun count(): Flow<Long>
}
