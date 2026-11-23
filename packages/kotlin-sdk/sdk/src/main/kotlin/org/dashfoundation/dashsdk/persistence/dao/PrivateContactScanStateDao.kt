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

    /**
     * The identity's position records in any wallet, except those marked
     * unresolved (see [deleteUnmarkedPositionsByWallet]): what removing an
     * identity from the device deletes.
     */
    @Query(
        "DELETE FROM private_contact_requests_positions " +
            "WHERE identityId = :identityId AND privatePaymentsUnresolved = 0",
    )
    suspend fun deleteUnmarkedPositionsOfIdentity(identityId: ByteArray)

    /**
     * Rewind the identity's marked position records to the start of the
     * store. Removing the identity deletes the contact history a record's
     * position covers while the marker must stay, so a reload reads the store
     * again from its start instead of from past requests it no longer has.
     */
    @Query(
        "UPDATE private_contact_requests_positions SET position = 0 " +
            "WHERE identityId = :identityId AND privatePaymentsUnresolved = 1",
    )
    suspend fun rewindMarkedPositionsOfIdentity(identityId: ByteArray)

    /** The shielded addresses of private requests to the identity [identityId]. */
    @Query("DELETE FROM private_shielded_addresses WHERE ownerId = :identityId")
    suspend fun deleteAddressesOwnedBy(identityId: ByteArray)

    /** The ids of every wallet with a stored position record, a removed wallet's kept ones included. */
    @Query("SELECT DISTINCT walletId FROM private_contact_requests_positions")
    suspend fun walletIdsWithPositions(): List<ByteArray>

    @Query("DELETE FROM private_shielded_addresses WHERE walletId = :walletId")
    suspend fun deleteAddressesByWallet(walletId: ByteArray)
}
