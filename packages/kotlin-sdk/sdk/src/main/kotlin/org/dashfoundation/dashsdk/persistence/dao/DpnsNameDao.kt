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
    @Query("SELECT * FROM dpns_names WHERE identityId = :identityId")
    fun observeByIdentity(identityId: ByteArray): Flow<List<DpnsNameEntity>>

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
