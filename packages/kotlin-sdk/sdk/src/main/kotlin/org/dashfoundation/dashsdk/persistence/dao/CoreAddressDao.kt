package org.dashfoundation.dashsdk.persistence.dao

import androidx.room.Dao
import androidx.room.Delete
import androidx.room.Query
import androidx.room.Upsert
import kotlinx.coroutines.flow.Flow
import org.dashfoundation.dashsdk.persistence.entities.CoreAddressEntity

/**
 * Queries over [CoreAddressEntity], mirroring the persister's upsert-by-
 * Base58Check-string (`$0.address == address`) and the per-account pool
 * views.
 */
@Dao
interface CoreAddressDao {

    @Query("SELECT * FROM core_addresses WHERE address = :address")
    fun observeByAddress(address: String): Flow<CoreAddressEntity?>

    @Query("SELECT * FROM core_addresses WHERE address = :address")
    suspend fun getByAddress(address: String): CoreAddressEntity?

    @Query("SELECT * FROM core_addresses WHERE accountId = :accountId ORDER BY poolTypeTag, addressIndex")
    fun observeByAccount(accountId: Long): Flow<List<CoreAddressEntity>>

    @Upsert
    suspend fun upsert(address: CoreAddressEntity)

    @Delete
    suspend fun delete(address: CoreAddressEntity)

    @Query("DELETE FROM core_addresses WHERE accountId = :accountId")
    suspend fun deleteByAccount(accountId: Long)

    @Query("DELETE FROM core_addresses")
    suspend fun deleteAll()

    /** StorageExplorer row count. */
    @Query("SELECT COUNT(*) FROM core_addresses")
    fun count(): Flow<Long>
}
