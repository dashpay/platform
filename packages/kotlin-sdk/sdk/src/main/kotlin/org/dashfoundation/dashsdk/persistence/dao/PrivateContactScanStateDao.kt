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

    /**
     * The wallet's position records, except those marked unresolved: a
     * marked record stands for a private payment that may still execute, so
     * it outlives its wallet until a record stored cleared for the same
     * wallet and identity replaces it.
     */
    @Query(
        "DELETE FROM private_contact_requests_positions " +
            "WHERE walletId = :walletId AND privatePaymentsUnresolved = 0",
    )
    suspend fun deleteUnmarkedPositionsByWallet(walletId: ByteArray)

    /** The ids of every wallet with a stored position record, a removed wallet's kept ones included. */
    @Query("SELECT DISTINCT walletId FROM private_contact_requests_positions")
    suspend fun walletIdsWithPositions(): List<ByteArray>

    @Query("DELETE FROM private_shielded_addresses WHERE walletId = :walletId")
    suspend fun deleteAddressesByWallet(walletId: ByteArray)
}
