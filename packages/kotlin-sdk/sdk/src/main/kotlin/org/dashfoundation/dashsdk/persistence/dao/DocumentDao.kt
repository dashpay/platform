package org.dashfoundation.dashsdk.persistence.dao

import androidx.room.Dao
import androidx.room.Delete
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

    /** Duplicate guard used before inserting a pending row. */
    @Query(
        "SELECT * FROM pending_inputs WHERE outpoint = :outpoint " +
            "AND spendingTxid = :spendingTxid"
    )
    suspend fun getPendingInput(outpoint: ByteArray, spendingTxid: ByteArray): PendingInputEntity?

    /** Per-wallet pending-input scan (cleanup / diagnostics). */
    @Query("SELECT * FROM pending_inputs WHERE walletId = :walletId")
    fun observePendingInputsByWallet(walletId: ByteArray): Flow<List<PendingInputEntity>>

    /**
     * Repoint every pending input of [walletId]'s own still recorded
     * against loser [txid] at [supersededBy] instead, except the outpoints
     * named in [releasedOutpoints] — those came free and are left for
     * `onWalletChangesetTransactionsSwept`'s own cascade-delete of [txid]
     * to remove. [spendingTransactionTxid] is cleared first so the FK no
     * longer targets the row about to be deleted (a live `transactions`
     * row cascades its `pending_inputs` children), and `isSweptTombstone`
     * marks the row so `onWalletChangesetUtxoAdded` knows this is a durable
     * claim rather than an ordinary in-flight spend once the funding TXO
     * finally lands.
     *
     * [txid] can be shared across wallets — the same loser can spend coins
     * from more than one of them — and upstream hands each wallet its own
     * [releasedOutpoints], computed only from that wallet's point of view.
     * The `walletId` filter is what keeps this call from repointing or
     * tombstoning a row a different wallet owns using a release decision
     * that was never made about it.
     */
    @Query(
        "UPDATE pending_inputs SET spendingTransactionTxid = NULL, " +
            "spendingTxid = :supersededBy, isSweptTombstone = 1 " +
            "WHERE spendingTransactionTxid = :txid AND walletId = :walletId " +
            "AND outpoint NOT IN (:releasedOutpoints)",
    )
    suspend fun tombstoneUnreleasedPendingInputs(
        txid: ByteArray,
        supersededBy: ByteArray,
        releasedOutpoints: List<ByteArray>,
        walletId: ByteArray,
    )

    /**
     * Chained-sweep continuation of [tombstoneUnreleasedPendingInputs]: a
     * pending row that an earlier sweep already tombstoned to [txid]
     * detached itself from the `spendingTransactionTxid` relationship at
     * that point, so a sweep of [txid] itself cannot find it there — only
     * the scalar `spendingTxid` this row was repointed to still names it.
     * Delete the ones this round frees. Nothing else owns them once
     * detached — unlike a live pending row, there is no cascade-delete of
     * [txid]'s `transactions` row left to do that job for them.
     *
     * A tombstone names one specific wallet's coin — the `walletId` it was
     * written with — so [walletId] here has to be the same wallet whose
     * [releasedOutpoints] produced it; otherwise this would apply one
     * wallet's release decision to a claim it was never entitled to make.
     */
    @Query(
        "DELETE FROM pending_inputs WHERE spendingTxid = :txid AND isSweptTombstone = 1 " +
            "AND walletId = :walletId AND outpoint IN (:releasedOutpoints)",
    )
    suspend fun deleteReleasedSweptTombstones(
        txid: ByteArray,
        releasedOutpoints: List<ByteArray>,
        walletId: ByteArray,
    )

    /**
     * The held half of [deleteReleasedSweptTombstones]: repoint every
     * surviving tombstone of [txid] owned by [walletId] at the new
     * [supersededBy] instead, so a third sweep down the chain can still
     * find it by scalar `spendingTxid`. [isSweptTombstone] is already set
     * from the first tombstoning and stays set.
     */
    @Query(
        "UPDATE pending_inputs SET spendingTxid = :supersededBy " +
            "WHERE spendingTxid = :txid AND isSweptTombstone = 1 AND walletId = :walletId " +
            "AND outpoint NOT IN (:releasedOutpoints)",
    )
    suspend fun retargetSweptTombstones(
        txid: ByteArray,
        supersededBy: ByteArray,
        releasedOutpoints: List<ByteArray>,
        walletId: ByteArray,
    )

    /**
     * Whether some wallet other than [walletId] still has a live pending
     * input pointing at [txid] as its spending transaction.
     *
     * Mirrors [TxoDao.hasOtherWalletSpender] for the pending-input side of
     * the same shared-row problem: [txid]'s `transactions` row is a
     * statement about the transaction as a whole, so only the callback that
     * finds no other wallet's claim left on it — TXO or pending input — is
     * allowed to delete it.
     */
    @Query(
        "SELECT EXISTS(SELECT 1 FROM pending_inputs " +
            "WHERE spendingTransactionTxid = :txid AND walletId != :walletId)",
    )
    suspend fun hasOtherWalletPendingInput(txid: ByteArray, walletId: ByteArray): Boolean

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
