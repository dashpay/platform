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

    @Query("DELETE FROM transactions")
    suspend fun deleteAll()

    /** StorageExplorer row count. */
    @Query("SELECT COUNT(*) FROM transactions")
    fun count(): Flow<Long>
}
