package org.dashfoundation.dashsdk.persistence.dao

import androidx.room.Dao
import androidx.room.Delete
import androidx.room.Query
import androidx.room.Upsert
import kotlinx.coroutines.flow.Flow
import org.dashfoundation.dashsdk.persistence.entities.DataContractEntity

/**
 * Queries over [DataContractEntity], mirroring the
 * `PersistentDataContract` predicate helpers: `predicate(contractId:)`,
 * `predicate(ownerId:)`, `predicate(name:)`,
 * `contractsWithTokensPredicate(…)`, `predicate(keyword:)`,
 * `needsSyncPredicate(olderThan:)`, `predicate(network:)`.
 */
@Dao
interface DataContractDao {

    @Query("SELECT * FROM data_contracts")
    fun observeAll(): Flow<List<DataContractEntity>>

    /** Mirror of `predicate(contractId:)` (binary form of the base58 id). */
    @Query("SELECT * FROM data_contracts WHERE id = :id")
    fun observeById(id: ByteArray): Flow<DataContractEntity?>

    @Query("SELECT * FROM data_contracts WHERE id = :id")
    suspend fun getById(id: ByteArray): DataContractEntity?

    /** Mirror of `predicate(network:)`. */
    @Query("SELECT * FROM data_contracts WHERE networkRaw = :networkRaw")
    fun observeByNetwork(networkRaw: Int): Flow<List<DataContractEntity>>

    /** Mirror of `predicate(ownerId:)` (ContractIdentityLinker back-fill). */
    @Query("SELECT * FROM data_contracts WHERE ownerId = :ownerId")
    fun observeByOwnerId(ownerId: ByteArray): Flow<List<DataContractEntity>>

    @Query("SELECT * FROM data_contracts WHERE ownerId = :ownerId")
    suspend fun getByOwnerId(ownerId: ByteArray): List<DataContractEntity>

    /** Mirror of `predicate(name:)` (`localizedStandardContains` → LIKE). */
    @Query("SELECT * FROM data_contracts WHERE name LIKE '%' || :name || '%'")
    fun observeByNameContains(name: String): Flow<List<DataContractEntity>>

    /** Mirror of `contractsWithTokensPredicate`. */
    @Query("SELECT * FROM data_contracts WHERE hasTokens = 1")
    fun observeWithTokens(): Flow<List<DataContractEntity>>

    /** Mirror of `contractsWithTokensPredicate(network:)`. */
    @Query("SELECT * FROM data_contracts WHERE hasTokens = 1 AND networkRaw = :networkRaw")
    fun observeWithTokensByNetwork(networkRaw: Int): Flow<List<DataContractEntity>>

    /** Mirror of `predicate(keyword:)` (`keywordRelations.contains`). */
    @Query(
        "SELECT data_contracts.* FROM data_contracts INNER JOIN keywords " +
            "ON keywords.dataContractId = data_contracts.id WHERE keywords.keyword = :keyword"
    )
    fun observeByKeyword(keyword: String): Flow<List<DataContractEntity>>

    /** Mirror of `needsSyncPredicate(olderThan:)`; epoch millis. */
    @Query(
        "SELECT * FROM data_contracts WHERE lastSyncedAt IS NULL " +
            "OR lastSyncedAt < :olderThanMillis"
    )
    suspend fun getNeedingSync(olderThanMillis: Long): List<DataContractEntity>

    @Upsert
    suspend fun upsert(contract: DataContractEntity)

    @Delete
    suspend fun delete(contract: DataContractEntity)

    @Query("DELETE FROM data_contracts WHERE id = :id")
    suspend fun deleteById(id: ByteArray)

    @Query("DELETE FROM data_contracts")
    suspend fun deleteAll()

    /** StorageExplorer row count. */
    @Query("SELECT COUNT(*) FROM data_contracts")
    fun count(): Flow<Long>

    /** StorageExplorer network-scoped row count. */
    @Query("SELECT COUNT(*) FROM data_contracts WHERE networkRaw = :networkRaw")
    fun countByNetwork(networkRaw: Int): Flow<Long>
}
