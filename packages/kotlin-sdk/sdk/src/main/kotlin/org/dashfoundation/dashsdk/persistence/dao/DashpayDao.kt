package org.dashfoundation.dashsdk.persistence.dao

import androidx.room.Dao
import androidx.room.Query
import androidx.room.Upsert
import kotlinx.coroutines.flow.Flow
import org.dashfoundation.dashsdk.persistence.entities.DashpayContactRequestEntity
import org.dashfoundation.dashsdk.persistence.entities.DashpayIgnoredSenderEntity
import org.dashfoundation.dashsdk.persistence.entities.DashpayProfileEntity

/**
 * Queries over [DashpayProfileEntity], [DashpayContactRequestEntity] and
 * [DashpayIgnoredSenderEntity], mirroring
 * `PersistentDashpayProfile.predicate(identityId:)`,
 * `PersistentDashpayContactRequest.predicate(ownerIdentityId:)` /
 * `predicate(ownerIdentityId:isOutgoing:)`,
 * `PersistentDashpayIgnoredSender.predicate(...)`, and the persister's
 * upsert / tombstone / ignore-delta paths keyed on
 * `(networkRaw, ownerIdentityId, contactIdentityId, isOutgoing)` and
 * `(networkRaw, ownerIdentityId, ignoredSenderId)`.
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

    /** Restore-path read: every contact row owned by [ownerIdentityId]. */
    @Query("SELECT * FROM dashpay_contact_requests WHERE ownerIdentityId = :ownerIdentityId")
    suspend fun getContactRequestsByOwner(
        ownerIdentityId: ByteArray,
    ): List<DashpayContactRequestEntity>

    @Query("DELETE FROM dashpay_contact_requests")
    suspend fun deleteAllContactRequests()

    // MARK: Ignored senders (per-sender mute, local-only)

    /** Ignore-delta upsert (`isIgnored == true`). */
    @Upsert
    suspend fun upsertIgnoredSender(row: DashpayIgnoredSenderEntity)

    /** Ignore-delta delete (`isIgnored == false`, an un-ignore). */
    @Query(
        "DELETE FROM dashpay_ignored_senders WHERE ownerIdentityId = :ownerIdentityId " +
            "AND ignoredSenderId = :ignoredSenderId"
    )
    suspend fun deleteIgnoredSender(ownerIdentityId: ByteArray, ignoredSenderId: ByteArray)

    /** Restore-path read: every sender ignored by [ownerIdentityId]. */
    @Query("SELECT * FROM dashpay_ignored_senders WHERE ownerIdentityId = :ownerIdentityId")
    suspend fun getIgnoredSendersByOwner(
        ownerIdentityId: ByteArray,
    ): List<DashpayIgnoredSenderEntity>

    /** Ignored screen (mirror of Swift `IgnoredContactsView`'s `@Query`). */
    @Query("SELECT * FROM dashpay_ignored_senders WHERE ownerIdentityId = :ownerIdentityId")
    fun observeIgnoredSenders(
        ownerIdentityId: ByteArray,
    ): Flow<List<DashpayIgnoredSenderEntity>>

    /** StorageExplorer row count. */
    @Query("SELECT COUNT(*) FROM dashpay_contact_requests")
    fun countContactRequests(): Flow<Long>

    /** StorageExplorer network-scoped row count. */
    @Query("SELECT COUNT(*) FROM dashpay_contact_requests WHERE networkRaw = :networkRaw")
    fun countContactRequestsByNetwork(networkRaw: Int): Flow<Long>
}
