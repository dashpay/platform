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

    /**
     * One outpoint-ordered page of a wallet's TXOs, for a pass that must
     * not hold the whole table at once (the store reconcile's reverse
     * half). Pass an empty [after] to start — an empty BLOB sorts before
     * every real 36-byte outpoint — then the previous page's last
     * `outpoint` to continue.
     *
     * `outpoint` is the primary key, so the order is an index walk and the
     * cursor is exact: no row can be visited twice or skipped because
     * another one was inserted or deleted mid-sweep.
     */
    @Query(
        "SELECT * FROM txos WHERE walletId = :walletId AND outpoint > :after " +
            "ORDER BY outpoint LIMIT :limit",
    )
    suspend fun pageByWallet(walletId: ByteArray, after: ByteArray, limit: Int): List<TxoEntity>

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
    /**
     * Rows keyed by outpoint — the sweep pass's bulk read of a loser's
     * decoded inputs and of a batch's released outpoints. Callers chunk
     * the list (`SWEEP_BIND_CHUNK`) so the statement arity stays under
     * the 999-variable ceiling API 29's framework SQLite still carries.
     */
    @Query("SELECT * FROM txos WHERE outpoint IN (:outpoints)")
    suspend fun getByOutpoints(outpoints: List<ByteArray>): List<TxoEntity>

    /**
     * Every row still linked to one of [spendingTxids] — the sweep pass's
     * link-keyed fallback for a loser whose stored bytes cannot name its
     * inputs (record lost, or a stub row written by `utxos_added` before
     * the record arrived). Chunked by the caller.
     */
    @Query("SELECT * FROM txos WHERE spendingTxid IN (:spendingTxids)")
    suspend fun getBySpendingTxids(spendingTxids: List<ByteArray>): List<TxoEntity>

    /**
     * Hold the coins at [outpoints] out of the restore set, attributed to
     * [supersededBy] — the same stamp the SQLite store writes as
     * `spent_in_txid`, and the same one the pending-input drain writes when
     * the claim had no TXO row yet. The hold is keyed by OUTPOINT, computed
     * from the swept loser's own decoded inputs, never by this row's link:
     * a link can move between the record and the sweep (a winner recorded
     * in the same round takes it first), and a hold keyed by link would
     * miss exactly the coin the winner consumed. The link is left alone
     * here — [detachSpenders] drops only links that point at a swept loser;
     * a link to the winner or to any other surviving record is kept, and
     * the stamp holds the coin regardless.
     *
     * Global, not wallet-scoped: `supersededBy` is a txid fact, and the
     * first callback that sees the sweep holds every wallet's rows for the
     * loser's inputs; only the RELEASE is per wallet ([releaseByOutpoints]).
     * A stamped hold only ever comes free through a release or through the
     * wallet re-delivering the unlinked coin unspent. Chunked by the caller.
     */
    @Query(
        "UPDATE txos SET isSpent = 1, supersededByTxid = :supersededBy " +
            "WHERE outpoint IN (:outpoints)",
    )
    suspend fun holdByOutpoints(outpoints: List<ByteArray>, supersededBy: ByteArray)

    /**
     * Drop every link that points at one of [spendingTxids] — the swept
     * losers of one batch. The foreign key would null these on the losers'
     * delete anyway; doing it explicitly, before the delete, keeps the
     * order the sweep pass documents (hold by outpoint, detach the dead
     * link, delete the row) independent of FK enforcement. Chunked by the
     * caller.
     */
    @Query(
        "UPDATE txos SET spendingTxid = NULL, spendingInputIndex = NULL " +
            "WHERE spendingTxid IN (:spendingTxids)",
    )
    suspend fun detachSpenders(spendingTxids: List<ByteArray>)

    /**
     * Mark [walletId]'s own coins at [outpoints] unspent again — coins a
     * sweep released, meaning no surviving transaction spent them *at the
     * time the sweep was computed*. Keyed by outpoint because that is how
     * upstream reports it: the transaction that took the other inputs may
     * never be recorded here at all, so the released set is the only
     * authority on which coins came free.
     *
     * Per wallet, unlike [holdByOutpoints]: a released set is only ever
     * true of the wallet that computed it, so it never touches another
     * wallet's row. The caller has already excluded every vetoed outpoint
     * (a row linked to, or stamped with, a stored network-final spender
     * that this round did not sweep) and every outpoint whose funding
     * transaction is itself swept this round (deleted instead). The link
     * is not touched: a link to a swept loser was detached by
     * [detachSpenders], and a link to a surviving mempool spender is kept
     * as attribution at `isSpent = 0`, exactly what such a link means on
     * the record channel.
     *
     * `supersededByTxid` clears in the same statement, the way the SQLite
     * store's release UPDATE clears `spent_in_txid`: a released coin
     * keeping its dead winner's marker would read as a durable claim to
     * every later hold on this outpoint. Chunked by the caller.
     */
    @Query(
        "UPDATE txos SET isSpent = 0, supersededByTxid = NULL " +
            "WHERE outpoint IN (:outpoints) AND walletId = :walletId",
    )
    suspend fun releaseByOutpoints(outpoints: List<ByteArray>, walletId: ByteArray)

    /**
     * Delete every TXO created by one of [txids] — the swept losers' own
     * outputs, dead coins for every wallet. The FK from `txos.txid` to
     * `transactions.txid` (CASCADE) does this on the losers' delete too;
     * the explicit form runs first so the sweep pass never depends on FK
     * enforcement for the one removal that is a funds fact. Chunked by the
     * caller.
     */
    @Query("DELETE FROM txos WHERE txid IN (:txids)")
    suspend fun deleteByTxids(txids: List<ByteArray>)

    /**
     * Delete the rows at [outpoints] outright — outputs of a transaction
     * swept in this round that some loser claimed or some release named.
     * A coin created by a dead transaction cannot be unspent, only gone:
     * a chainlocked reinstatement of the parent re-delivers it through the
     * ordinary `utxos_added` upsert with nothing left standing in its way.
     * Chunked by the caller.
     */
    @Query("DELETE FROM txos WHERE outpoint IN (:outpoints)")
    suspend fun deleteByOutpoints(outpoints: List<ByteArray>)

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
