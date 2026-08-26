package org.dashfoundation.dashsdk.persistence.dao

import androidx.room.Dao
import androidx.room.Delete
import androidx.room.Query
import androidx.room.Upsert
import kotlinx.coroutines.flow.Flow
import org.dashfoundation.dashsdk.persistence.entities.TransactionEntity
import org.dashfoundation.dashsdk.persistence.entities.TransactionAccountInvolvementEntity

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

    /**
     * Transaction-label resolver probe for the withdraw/unshield case,
     * which has NO `asset_locks` row and is identified solely by
     * `transactionTypeKind == 7` (AssetUnlock).
     *
     * [txidWire] is the raw little-endian WIRE txid — the exact
     * [TransactionEntity.txid] PK form (a BLOB), i.e. the explorer display
     * hex reversed then hex-decoded. When the caller holds the display hex
     * (as the resolver does), prefer [transactionKindForDisplayTxid], which
     * does the reversal. Returns null when no such tx is stored;
     * `transactionTypeKind == 0xFF` (255) means "not yet populated".
     */
    @Query("SELECT transactionTypeKind FROM transactions WHERE txid = :txidWire LIMIT 1")
    suspend fun transactionKindForTxid(txidWire: ByteArray): Int?

    /**
     * Convenience over [transactionKindForTxid] keyed by the explorer
     * DISPLAY txid hex (64 lowercase chars, wire order reversed) the
     * resolver already holds — the same display form used by
     * [AssetLockDao.fundingTypeForTxid]. Reverses to wire order before the
     * BLOB PK match. Returns null for malformed hex (not 64 hex chars) or
     * when no such tx is stored. Not a Room query — a plain default method
     * delegating to [transactionKindForTxid].
     */
    suspend fun transactionKindForDisplayTxid(txidDisplayHex: String): Int? {
        val wire = displayHexToWireTxid(txidDisplayHex) ?: return null
        return transactionKindForTxid(wire)
    }

    /** Timeline join helper — parents of a wallet's TXO set. */
    @Query("SELECT * FROM transactions WHERE txid IN (:txids) ORDER BY firstSeen DESC")
    fun observeByTxids(txids: List<ByteArray>): Flow<List<TransactionEntity>>

    @Upsert
    suspend fun upsert(transaction: TransactionEntity)

    /**
     * TXO-reconcile repair: credit a missed own-output back into the
     * transaction's stored net amount. A record born blind to one of its
     * own outputs (the CoinJoin-funded-send change-drop) persists
     * `netAmount` short by exactly that output's value, so the repair is
     * a plain add. Returns the number of rows updated (0 = no such tx).
     */
    @Query("UPDATE transactions SET netAmount = netAmount + :delta WHERE txid = :txid")
    suspend fun addToNetAmount(txid: ByteArray, delta: Long): Int

    @Upsert
    suspend fun upsertInvolvement(involvement: TransactionAccountInvolvementEntity)

    /**
     * Provider kinds 2…5 scoped through explicit account membership. The
     * ordering preserves Core's same-block transaction order when present.
     *
     * `isGloballySwept = 0` excludes a provider transaction that itself lost
     * a double-spend on one of its own inputs — an edge case (most losers
     * are ordinary spends), but a swept row is never restorable regardless
     * of kind. See [TransactionEntity.isGloballySwept].
     */
    @Query(
        "SELECT DISTINCT transactions.* FROM transactions " +
            "INNER JOIN transaction_account_involvements involvement " +
            "ON involvement.transactionTxid = transactions.txid " +
            "INNER JOIN accounts ON accounts.id = involvement.accountId " +
            "WHERE accounts.walletId = :walletId " +
            "AND accounts.accountType BETWEEN 8 AND 11 " +
            "AND transactions.transactionTypeKind BETWEEN 2 AND 5 " +
            "AND transactions.isGloballySwept = 0 " +
            "ORDER BY transactions.blockHeight ASC, " +
            "transactions.hasBlockPosition DESC, transactions.blockPosition ASC, " +
            "transactions.firstSeen ASC"
    )
    suspend fun getProviderSpecialTransactionsByWallet(walletId: ByteArray): List<TransactionEntity>

    @Query("SELECT COUNT(*) FROM transaction_account_involvements WHERE transactionTxid = :txid")
    suspend fun countInvolvements(txid: ByteArray): Int

    @Delete
    suspend fun delete(transaction: TransactionEntity)

    /**
     * Durable global exclusion for a swept loser — set in EVERY wallet's
     * `onWalletChangesetTransactionsSwept` callback that observes the sweep,
     * not only the one whose [deleteByTxid] happens to remove the shared
     * row. Idempotent: re-flagging an already-flagged row is a no-op. See
     * [TransactionEntity.isGloballySwept].
     */
    @Query("UPDATE transactions SET isGloballySwept = 1 WHERE txid = :txid")
    suspend fun markGloballySwept(txid: ByteArray)

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
            "WHERE spendingTransactionTxid IS NOT NULL) " +
            "AND txid NOT IN (SELECT transactionTxid FROM transaction_account_involvements)"
    )
    suspend fun deleteOrphanTransactions()

    @Query("DELETE FROM transactions")
    suspend fun deleteAll()

    /** StorageExplorer row count. */
    @Query("SELECT COUNT(*) FROM transactions")
    fun count(): Flow<Long>

    companion object {
        /**
         * Explorer DISPLAY txid hex (wire order reversed, 64 lowercase
         * chars) → raw little-endian WIRE txid, the [TransactionEntity.txid]
         * PK form. Mirrors the txid half of [decodeOutPointHex]. Returns
         * null for any non-64-char or non-hex input.
         */
        private fun displayHexToWireTxid(displayHex: String): ByteArray? {
            if (displayHex.length != 64) return null
            val display = ByteArray(32)
            for (i in 0 until 32) {
                val hi = Character.digit(displayHex[i * 2], 16)
                val lo = Character.digit(displayHex[i * 2 + 1], 16)
                if (hi < 0 || lo < 0) return null
                display[i] = ((hi shl 4) or lo).toByte()
            }
            val wire = ByteArray(32)
            for (i in 0 until 32) wire[i] = display[31 - i]
            return wire
        }
    }
}
