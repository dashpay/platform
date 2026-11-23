package org.dashfoundation.dashsdk.persistence.dao

import androidx.room.Dao
import androidx.room.Query
import androidx.room.Upsert
import org.dashfoundation.dashsdk.persistence.entities.IdentitySlotLeaseEntity

/** The invitation-claim slot leases, per wallet. */
@Dao
interface IdentitySlotLeaseDao {
    @Upsert
    suspend fun upsert(row: IdentitySlotLeaseEntity)

    @Query(
        "DELETE FROM identity_slot_leases " +
            "WHERE walletId = :walletId AND invitationOutPoint = :invitationOutPoint",
    )
    suspend fun delete(walletId: ByteArray, invitationOutPoint: ByteArray)

    @Query("SELECT * FROM identity_slot_leases WHERE walletId = :walletId")
    suspend fun byWallet(walletId: ByteArray): List<IdentitySlotLeaseEntity>

    @Query("DELETE FROM identity_slot_leases WHERE walletId = :walletId")
    suspend fun deleteByWallet(walletId: ByteArray)
}
