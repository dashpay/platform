package org.dashfoundation.dashsdk.persistence.dao

import androidx.room.Dao
import androidx.room.Delete
import androidx.room.Query
import androidx.room.Upsert
import kotlinx.coroutines.flow.Flow
import org.dashfoundation.dashsdk.persistence.entities.DpnsNameEntity

/**
 * Queries over [DpnsNameEntity], mirroring
 * `PersistentDPNSName.predicate(identityId:)` and the persister's
 * upsert by `(networkRaw, normalizedParentDomainName, normalizedLabel)`.
 */
@Dao
interface DpnsNameDao {

    /** Mirror of `predicate(identityId:)` — labels owned by one identity. */
    @Query("SELECT * FROM dpns_names WHERE identityId = :identityId AND isOwned = 1")
    fun observeByIdentity(identityId: ByteArray): Flow<List<DpnsNameEntity>>

    /** Marketplace rows include retained sold/transferred history. */
    @Query("SELECT * FROM dpns_names WHERE identityId = :identityId ORDER BY normalizedLabel")
    fun observeMarketplaceByIdentity(identityId: ByteArray): Flow<List<DpnsNameEntity>>

    @Query("SELECT * FROM dpns_names WHERE identityId = :identityId")
    suspend fun getAllByIdentity(identityId: ByteArray): List<DpnsNameEntity>

    @Query("SELECT * FROM dpns_names WHERE documentId = :documentId LIMIT 1")
    suspend fun getByDocumentId(documentId: ByteArray): DpnsNameEntity?

    /**
     * Clear fields owned by marketplace reconciliation without deleting the
     * identity snapshot's label-cache row.
     */
    @Query(
        "UPDATE dpns_names SET documentId = NULL, priceCredits = NULL, " +
            "saleStatusRaw = 0, counterpartyIdentityId = NULL, " +
            "documentCreatedAtMs = 0, documentUpdatedAtMs = 0, " +
            "documentTransferredAtMs = 0, marketplaceUpdatedAt = 0, " +
            "lastUpdated = :lastUpdated WHERE documentId = :documentId"
    )
    suspend fun clearMarketplaceByDocumentId(documentId: ByteArray, lastUpdated: java.util.Date)

    /** Persister upsert key (the Swift `#Unique` triple). */
    @Query(
        "SELECT * FROM dpns_names WHERE networkRaw = :networkRaw " +
            "AND normalizedParentDomainName = :normalizedParentDomainName " +
            "AND normalizedLabel = :normalizedLabel"
    )
    suspend fun getByUniqueKey(
        networkRaw: Int,
        normalizedParentDomainName: String,
        normalizedLabel: String,
    ): DpnsNameEntity?

    @Query("SELECT * FROM dpns_names WHERE networkRaw = :networkRaw")
    fun observeByNetwork(networkRaw: Int): Flow<List<DpnsNameEntity>>

    @Upsert
    suspend fun upsert(name: DpnsNameEntity)

    @Delete
    suspend fun delete(name: DpnsNameEntity)

    @Query("DELETE FROM dpns_names")
    suspend fun deleteAll()

    /** StorageExplorer row count. */
    @Query("SELECT COUNT(*) FROM dpns_names")
    fun count(): Flow<Long>

    /** StorageExplorer network-scoped row count. */
    @Query("SELECT COUNT(*) FROM dpns_names WHERE networkRaw = :networkRaw")
    fun countByNetwork(networkRaw: Int): Flow<Long>
}
