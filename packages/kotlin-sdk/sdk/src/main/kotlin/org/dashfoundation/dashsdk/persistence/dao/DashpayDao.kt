package org.dashfoundation.dashsdk.persistence.dao

import androidx.room.Dao
import androidx.room.Query
import androidx.room.Upsert
import kotlinx.coroutines.flow.Flow
import org.dashfoundation.dashsdk.persistence.entities.DashpayContactRequestEntity
import org.dashfoundation.dashsdk.persistence.entities.DashpayProfileEntity

/**
 * Queries over [DashpayProfileEntity] and [DashpayContactRequestEntity],
 * mirroring `PersistentDashpayProfile.predicate(identityId:)`,
 * `PersistentDashpayContactRequest.predicate(ownerIdentityId:)` /
 * `predicate(ownerIdentityId:isOutgoing:)`, and the persister's
 * upsert / tombstone paths keyed on
 * `(networkRaw, ownerIdentityId, contactIdentityId, isOutgoing)`.
 */
@Dao
interface DashpayDao {

    // MARK: Profiles

    /** Mirror of `PersistentDashpayProfile.predicate(identityId:)`. */
    @Query("SELECT * FROM dashpay_profiles WHERE identityId = :identityId")
    fun observeProfileByIdentity(identityId: ByteArray): Flow<DashpayProfileEntity?>

    @Query("SELECT * FROM dashpay_profiles WHERE networkRaw = :networkRaw AND identityId = :identityId")
    suspend fun getProfile(networkRaw: Int, identityId: ByteArray): DashpayProfileEntity?

    @Upsert
    suspend fun upsertProfile(profile: DashpayProfileEntity)

    @Query("DELETE FROM dashpay_profiles WHERE identityId = :identityId")
    suspend fun deleteProfilesByIdentity(identityId: ByteArray)

    @Query("DELETE FROM dashpay_profiles")
    suspend fun deleteAllProfiles()

    /** StorageExplorer row count. */
    @Query("SELECT COUNT(*) FROM dashpay_profiles")
    fun countProfiles(): Flow<Long>

    /** StorageExplorer network-scoped row count. */
    @Query("SELECT COUNT(*) FROM dashpay_profiles WHERE networkRaw = :networkRaw")
    fun countProfilesByNetwork(networkRaw: Int): Flow<Long>

    // MARK: Contact requests

    /** Mirror of `predicate(ownerIdentityId:)`. */
    @Query("SELECT * FROM dashpay_contact_requests WHERE ownerIdentityId = :ownerIdentityId")
    fun observeContactRequests(ownerIdentityId: ByteArray): Flow<List<DashpayContactRequestEntity>>

    /** Mirror of `predicate(ownerIdentityId:isOutgoing:)`. */
    @Query(
        "SELECT * FROM dashpay_contact_requests WHERE ownerIdentityId = :ownerIdentityId " +
            "AND isOutgoing = :isOutgoing"
    )
    fun observeContactRequests(
        ownerIdentityId: ByteArray,
        isOutgoing: Boolean,
    ): Flow<List<DashpayContactRequestEntity>>

    /** Persister upsert key (the Swift `#Unique` quad). */
    @Query(
        "SELECT * FROM dashpay_contact_requests WHERE networkRaw = :networkRaw " +
            "AND ownerIdentityId = :ownerIdentityId " +
            "AND contactIdentityId = :contactIdentityId AND isOutgoing = :isOutgoing"
    )
    suspend fun getContactRequest(
        networkRaw: Int,
        ownerIdentityId: ByteArray,
        contactIdentityId: ByteArray,
        isOutgoing: Boolean,
    ): DashpayContactRequestEntity?

    @Upsert
    suspend fun upsertContactRequest(request: DashpayContactRequestEntity)

    /** Mirror of the persister's `deleteContactRow` tombstone path. */
    @Query(
        "DELETE FROM dashpay_contact_requests WHERE ownerIdentityId = :ownerIdentityId " +
            "AND contactIdentityId = :contactIdentityId AND isOutgoing = :isOutgoing"
    )
    suspend fun deleteContactRequest(
        ownerIdentityId: ByteArray,
        contactIdentityId: ByteArray,
        isOutgoing: Boolean,
    )

    @Query("DELETE FROM dashpay_contact_requests")
    suspend fun deleteAllContactRequests()

    /** StorageExplorer row count. */
    @Query("SELECT COUNT(*) FROM dashpay_contact_requests")
    fun countContactRequests(): Flow<Long>

    /** StorageExplorer network-scoped row count. */
    @Query("SELECT COUNT(*) FROM dashpay_contact_requests WHERE networkRaw = :networkRaw")
    fun countContactRequestsByNetwork(networkRaw: Int): Flow<Long>
}
