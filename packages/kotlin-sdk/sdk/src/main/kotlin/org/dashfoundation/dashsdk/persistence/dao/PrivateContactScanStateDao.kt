package org.dashfoundation.dashsdk.persistence.dao

import androidx.room.Dao
import androidx.room.Query
import androidx.room.Upsert
import org.dashfoundation.dashsdk.persistence.entities.PrivateContactRequestsPositionEntity
import org.dashfoundation.dashsdk.persistence.entities.PrivateShieldedAddressEntity

/** The private contact request sync's scan state, per wallet. */
@Dao
interface PrivateContactScanStateDao {
    @Upsert
    suspend fun upsertPosition(row: PrivateContactRequestsPositionEntity)

    @Upsert
    suspend fun upsertAddress(row: PrivateShieldedAddressEntity)

    @Query(
        "DELETE FROM private_shielded_addresses " +
            "WHERE walletId = :walletId AND ownerId = :ownerId AND contactId = :contactId",
    )
    suspend fun deleteAddress(walletId: ByteArray, ownerId: ByteArray, contactId: ByteArray)

    @Query(
        "DELETE FROM private_contact_requests_positions " +
            "WHERE walletId = :walletId AND identityId = :identityId",
    )
    suspend fun deletePositionsOfIdentity(walletId: ByteArray, identityId: ByteArray)

    @Query("DELETE FROM private_shielded_addresses WHERE walletId = :walletId AND ownerId = :ownerId")
    suspend fun deleteAddressesOfIdentity(walletId: ByteArray, ownerId: ByteArray)

    @Query("SELECT * FROM private_contact_requests_positions WHERE walletId = :walletId")
    suspend fun positionsByWallet(walletId: ByteArray): List<PrivateContactRequestsPositionEntity>

    @Query("SELECT * FROM private_shielded_addresses WHERE walletId = :walletId")
    suspend fun addressesByWallet(walletId: ByteArray): List<PrivateShieldedAddressEntity>

    @Query("DELETE FROM private_contact_requests_positions WHERE walletId = :walletId")
    suspend fun deletePositionsByWallet(walletId: ByteArray)

    @Query("DELETE FROM private_shielded_addresses WHERE walletId = :walletId")
    suspend fun deleteAddressesByWallet(walletId: ByteArray)
}
