package org.dashfoundation.dashsdk.persistence.dao

import androidx.room.Dao
import androidx.room.Query
import androidx.room.Upsert
import org.dashfoundation.dashsdk.persistence.entities.IdentityIndexStateEntity

@Dao
interface IdentityIndexStateDao {
    @Query("SELECT * FROM identity_index_state WHERE walletId = :walletId")
    suspend fun get(walletId: ByteArray): IdentityIndexStateEntity?

    @Upsert
    suspend fun upsert(state: IdentityIndexStateEntity)

    @Query("DELETE FROM identity_index_state WHERE walletId = :walletId")
    suspend fun deleteByWallet(walletId: ByteArray)

    @Query("DELETE FROM identity_index_state")
    suspend fun deleteAll()
}
