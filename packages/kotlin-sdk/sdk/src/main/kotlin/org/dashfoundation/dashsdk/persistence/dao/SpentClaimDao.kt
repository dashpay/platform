package org.dashfoundation.dashsdk.persistence.dao

import androidx.room.Dao
import androidx.room.Query
import androidx.room.Upsert
import org.dashfoundation.dashsdk.persistence.entities.SpentClaimEntity

@Dao
interface SpentClaimDao {
    @Upsert
    suspend fun upsert(claim: SpentClaimEntity)

    @Query("DELETE FROM core_spent_claims WHERE walletId = :walletId AND txid = :txid AND vout = :vout")
    suspend fun release(walletId: ByteArray, txid: ByteArray, vout: Int)

    /** One grouped scan for all wallet restore entries, including unknown claimants. */
    @Query("SELECT * FROM core_spent_claims ORDER BY walletId, txid, vout")
    suspend fun getAll(): List<SpentClaimEntity>
}
