package org.dashfoundation.dashsdk.persistence.dao

import androidx.room.Dao
import androidx.room.Delete
import androidx.room.Insert
import androidx.room.Query
import androidx.room.Upsert
import kotlinx.coroutines.flow.Flow
import org.dashfoundation.dashsdk.persistence.entities.DocumentEntity
import org.dashfoundation.dashsdk.persistence.entities.DocumentTypeEntity
import org.dashfoundation.dashsdk.persistence.entities.IndexEntity
import org.dashfoundation.dashsdk.persistence.entities.KeywordEntity
import org.dashfoundation.dashsdk.persistence.entities.PendingInputEntity
import org.dashfoundation.dashsdk.persistence.entities.PropertyEntity

/**
 * Queries over the document family — [DocumentEntity],
 * [DocumentTypeEntity], [IndexEntity], [KeywordEntity], [PropertyEntity]
 * — plus [PendingInputEntity] (grouped here per the module's DAO layout).
 *
 * Document reads mirror the `PersistentDocument` predicate helpers
 * (`predicate(documentId:)`, `predicate(contractId:network:)`,
 * `predicate(ownerId:)`, all excluding `isDeleted` rows); keyword reads
 * mirror `PersistentKeyword.predicate(keyword:/contractId:)`; pending
 * inputs mirror the reconciliation lookups in
 * `PlatformWalletPersistenceHandler.swift` (`$0.outpoint == outpoint`,
 * `$0.outpoint == outpoint && $0.spendingTxid == spendingTxid`,
 * `$0.walletId == walletId`).
 */
@Dao
interface DocumentDao {

    // MARK: Documents

    /** Mirror of `PersistentDocument.predicate(documentId:)`. */
    @Query("SELECT * FROM documents WHERE documentId = :documentId AND isDeleted = 0")
    fun observeByDocumentId(documentId: String): Flow<DocumentEntity?>

    @Query("SELECT * FROM documents WHERE documentId = :documentId AND isDeleted = 0")
    suspend fun getByDocumentId(documentId: String): DocumentEntity?

    /** Mirror of `predicate(contractId:network:)` (base58 contract id). */
    @Query(
        "SELECT * FROM documents WHERE contractId = :contractId " +
            "AND networkRaw = :networkRaw AND isDeleted = 0"
    )
    fun observeByContractAndNetwork(contractId: String, networkRaw: Int): Flow<List<DocumentEntity>>

    /** Mirror of `predicate(ownerId:)` (base58 owner id). */
    @Query("SELECT * FROM documents WHERE ownerId = :ownerId AND isDeleted = 0")
    fun observeByOwnerId(ownerId: String): Flow<List<DocumentEntity>>

    @Upsert
    suspend fun upsertDocument(document: DocumentEntity)

    /** Mirror of `PersistentDocument.markAsDeleted()`. */
    @Query(
        "UPDATE documents SET isDeleted = 1, updatedAt = :nowMillis " +
            "WHERE documentId = :documentId"
    )
    suspend fun markDocumentDeleted(documentId: String, nowMillis: Long): Int

    @Delete
    suspend fun deleteDocument(document: DocumentEntity)

    @Query("DELETE FROM documents")
    suspend fun deleteAllDocuments()

    /** StorageExplorer row count. */
    @Query("SELECT COUNT(*) FROM documents")
    fun countDocuments(): Flow<Long>

    /** StorageExplorer network-scoped row count. */
    @Query("SELECT COUNT(*) FROM documents WHERE networkRaw = :networkRaw")
    fun countDocumentsByNetwork(networkRaw: Int): Flow<Long>

    // MARK: Document types

    @Query("SELECT * FROM document_types WHERE id = :id")
    suspend fun getDocumentTypeById(id: ByteArray): DocumentTypeEntity?

    /** Contract's document types (TransitionDetailView drill-down). */
    @Query("SELECT * FROM document_types WHERE contractId = :contractId")
    fun observeDocumentTypesByContract(contractId: ByteArray): Flow<List<DocumentTypeEntity>>

    @Query("SELECT * FROM document_types WHERE contractId = :contractId AND name = :name")
    suspend fun getDocumentTypeByName(contractId: ByteArray, name: String): DocumentTypeEntity?

    @Upsert
    suspend fun upsertDocumentType(documentType: DocumentTypeEntity)

    @Delete
    suspend fun deleteDocumentType(documentType: DocumentTypeEntity)

    @Query("DELETE FROM document_types")
    suspend fun deleteAllDocumentTypes()

    /** StorageExplorer row count. */
    @Query("SELECT COUNT(*) FROM document_types")
    fun countDocumentTypes(): Flow<Long>

    // MARK: Indices

    @Query("SELECT * FROM indices WHERE documentTypeId = :documentTypeId")
    fun observeIndicesByDocumentType(documentTypeId: ByteArray): Flow<List<IndexEntity>>

    @Upsert
    suspend fun upsertIndex(index: IndexEntity)

    @Delete
    suspend fun deleteIndex(index: IndexEntity)

    @Query("DELETE FROM indices")
    suspend fun deleteAllIndices()

    /** StorageExplorer row count. */
    @Query("SELECT COUNT(*) FROM indices")
    fun countIndices(): Flow<Long>

    // MARK: Keywords

    /** Mirror of `PersistentKeyword.predicate(contractId:)` (base58). */
    @Query("SELECT * FROM keywords WHERE contractId = :contractId")
    fun observeKeywordsByContract(contractId: String): Flow<List<KeywordEntity>>

    /** Mirror of `PersistentKeyword.predicate(keyword:)` (contains). */
    @Query("SELECT * FROM keywords WHERE keyword LIKE '%' || :keyword || '%'")
    fun observeKeywordsContaining(keyword: String): Flow<List<KeywordEntity>>

    @Upsert
    suspend fun upsertKeyword(keyword: KeywordEntity)

    @Delete
    suspend fun deleteKeyword(keyword: KeywordEntity)

    @Query("DELETE FROM keywords")
    suspend fun deleteAllKeywords()

    /** StorageExplorer row count. */
    @Query("SELECT COUNT(*) FROM keywords")
    fun countKeywords(): Flow<Long>

    // MARK: Properties

    @Query("SELECT * FROM properties WHERE documentTypeId = :documentTypeId")
    fun observePropertiesByDocumentType(documentTypeId: ByteArray): Flow<List<PropertyEntity>>

    @Upsert
    suspend fun upsertProperty(property: PropertyEntity)

    @Delete
    suspend fun deleteProperty(property: PropertyEntity)

    @Query("DELETE FROM properties")
    suspend fun deleteAllProperties()

    /** StorageExplorer row count. */
    @Query("SELECT COUNT(*) FROM properties")
    fun countProperties(): Flow<Long>

    // MARK: Pending inputs

    /** Per-outpoint reconciliation lookup (runs on every TXO upsert). */
    @Query("SELECT * FROM pending_inputs WHERE outpoint = :outpoint")
    suspend fun getPendingInputsByOutpoint(outpoint: ByteArray): List<PendingInputEntity>

    /**
     * Duplicate guard used before inserting a pending row. Keyed by
     * `(outpoint, spendingTxid, walletId)`: a sweep's hold and release
     * verdicts are per wallet, so a second wallet recording the same
     * transaction must get its own row — otherwise the first wallet's
     * collector or release could erase the only hold the second was
     * entitled to keep.
     */
    @Query(
        "SELECT * FROM pending_inputs WHERE outpoint = :outpoint " +
            "AND spendingTxid = :spendingTxid AND walletId = :walletId LIMIT 1"
    )
    suspend fun getPendingInput(
        outpoint: ByteArray,
        spendingTxid: ByteArray,
        walletId: ByteArray,
    ): PendingInputEntity?

    /** Per-wallet pending-input scan (cleanup / diagnostics). */
    @Query("SELECT * FROM pending_inputs WHERE walletId = :walletId")
    fun observePendingInputsByWallet(walletId: ByteArray): Flow<List<PendingInputEntity>>

    /**
     * Every wallet's rows claimed by one of [spendingTxids] — the ordinary
     * rows a loser staged (`spendingTxid == spendingTransactionTxid`) and
     * the tombstones an earlier sweep re-pointed at it (`spendingTxid`
     * alone, the FK already detached). One bulk read per sweep batch, all
     * wallets, because the hold is global: the first callback that sees a
     * sweep tombstones every wallet's claim on the loser's inputs. Chunked
     * by the caller (`SWEEP_BIND_CHUNK`) so the arity stays under the
     * 999-variable ceiling API 29's framework SQLite still carries.
     */
    @Query("SELECT * FROM pending_inputs WHERE spendingTxid IN (:spendingTxids)")
    suspend fun getPendingInputsBySpendingTxids(spendingTxids: List<ByteArray>): List<PendingInputEntity>

    /**
     * Turn the rows with these ids into swept tombstones held by [winner]:
     * detach the FK (the loser's row is about to be deleted and must not
     * cascade the claim away), re-point the scalar at the winner, flag the
     * row, and stamp the winner's mined height when this sweep has one —
     * an IS-locked, unmined winner (`hasWinnerMinedHeight = false`) keeps
     * whatever stamp the row already carries, because upstream's
     * observed-spend entry is never retracted by an unconfirmed conflict
     * and collection at the old height stays sound. Rowid-keyed and
     * chunked by the caller.
     */
    @Query(
        "UPDATE pending_inputs SET spendingTransactionTxid = NULL, spendingTxid = :winner, " +
            "isSweptTombstone = 1, " +
            "winnerMinedHeight = CASE WHEN :hasWinnerMinedHeight THEN :winnerMinedHeight " +
            "ELSE winnerMinedHeight END " +
            "WHERE id IN (:ids)",
    )
    suspend fun tombstonePendingInputs(
        ids: List<Long>,
        winner: ByteArray,
        hasWinnerMinedHeight: Boolean,
        winnerMinedHeight: Int,
    )

    /** Rowid-keyed bulk delete; chunked by the caller. */
    @Query("DELETE FROM pending_inputs WHERE id IN (:ids)")
    suspend fun deletePendingInputsByIds(ids: List<Long>)

    /**
     * Delete every wallet's tombstones on [outpoints] — outputs of a
     * transaction swept in this round, dead coins nobody may hold a claim
     * on (holding one would wedge the parent's chainlocked reinstatement).
     * Chunked by the caller.
     */
    @Query(
        "DELETE FROM pending_inputs WHERE isSweptTombstone = 1 AND outpoint IN (:outpoints)",
    )
    suspend fun deleteSweptTombstonesByOutpoints(outpoints: List<ByteArray>)

    /**
     * Delete [walletId]'s tombstones on [outpoints] — this wallet's
     * release of those coins. A released placeholder is deleted outright,
     * never left as a freed tombstone: no row is the correct end state, and
     * the funding output's own later upsert creates the real row freshly
     * unspent. Ordinary rows on the same outpoints are NOT touched — they
     * are some surviving spender's spend-before-funding claim, not the
     * swept loser's. Chunked by the caller.
     */
    @Query(
        "DELETE FROM pending_inputs WHERE walletId = :walletId AND isSweptTombstone = 1 " +
            "AND outpoint IN (:outpoints)",
    )
    suspend fun deleteWalletSweptTombstonesByOutpoints(walletId: ByteArray, outpoints: List<ByteArray>)

    /** Bulk insert of freshly minted tombstones; Room binds one row at a time. */
    @Insert
    suspend fun insertPendingInputs(rows: List<PendingInputEntity>)

    /**
     * Bounded tombstone lifetime: delete this wallet's swept tombstones
     * whose winner's mined height the chainlock finality boundary has
     * reached (`:boundary` = `min(chainlockHeight, syncedHeight)`, read
     * by the caller from the wallet row at the end of the round) —
     * key-wallet's `prune_finalized_observed_spends` condition verbatim,
     * no observation-age margin: the stamp IS the winner's height, so at
     * the boundary the funding transaction (mined at or below it) has been
     * filter-scanned with no false negatives. A tombstone still
     * collectible here never drained — its funding TXO never arrived — so
     * the junk case (a foreign input of a swept incoming payment) is
     * exactly what this removes; a genuine claim's row was already
     * deleted by the drain that moved the hold onto the TXO. Selects
     * tombstones only, served by the
     * `(walletId, isSweptTombstone, winnerMinedHeight)` index; ordinary
     * pending rows are never materialised here. Unstamped rows are never
     * collected — and they are a CURRENT, deliberate shape, not legacy
     * data: a mempool-context sweep (IS-locked, unmined winner) writes its
     * tombstone with a null stamp, because such a winner has no mining
     * deadline and no boundary can prove the held funding
     * delivered-or-never. An unstamped hold resolves only through proof —
     * the funding TXO drains it, a later block-context sweep re-stamps it
     * into this collector's reach, or a release deletes it — and holding
     * an unresolved one forever is the contract, not a safe fallback.
     */
    @Query(
        "DELETE FROM pending_inputs " +
            "WHERE walletId = :walletId AND isSweptTombstone = 1 " +
            "AND winnerMinedHeight IS NOT NULL AND winnerMinedHeight <= :boundary",
    )
    suspend fun collectFinalizedSweptTombstones(walletId: ByteArray, boundary: Int)

    @Upsert
    suspend fun upsertPendingInput(pendingInput: PendingInputEntity)

    @Delete
    suspend fun deletePendingInput(pendingInput: PendingInputEntity)

    /** Wallet teardown mirror of `deleteWalletData`'s pending-input pass. */
    @Query("DELETE FROM pending_inputs WHERE walletId = :walletId")
    suspend fun deletePendingInputsByWallet(walletId: ByteArray)

    @Query("DELETE FROM pending_inputs")
    suspend fun deleteAllPendingInputs()

    /** StorageExplorer row count ("long-lived non-zero pending count"). */
    @Query("SELECT COUNT(*) FROM pending_inputs")
    fun countPendingInputs(): Flow<Long>
}
