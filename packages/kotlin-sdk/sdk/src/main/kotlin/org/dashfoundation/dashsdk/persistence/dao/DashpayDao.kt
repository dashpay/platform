package org.dashfoundation.dashsdk.persistence.dao

import androidx.room.Dao
import androidx.room.Query
import androidx.room.Upsert
import kotlinx.coroutines.flow.Flow
import org.dashfoundation.dashsdk.persistence.entities.DashpayContactProfileEntity
import org.dashfoundation.dashsdk.persistence.entities.DashpayContactRequestEntity
import org.dashfoundation.dashsdk.persistence.entities.DashpayIgnoredSenderEntity
import org.dashfoundation.dashsdk.persistence.entities.DashpayPaymentEntity
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

    // MARK: Cached contact profiles

    /** Mirror of `PersistentDashpayContactProfile.predicate(ownerIdentityId:)`. */
    @Query("SELECT * FROM dashpay_contact_profiles WHERE ownerIdentityId = :ownerIdentityId")
    fun observeContactProfiles(
        ownerIdentityId: ByteArray,
    ): Flow<List<DashpayContactProfileEntity>>

    /** Mirror of `predicate(ownerIdentityId:contactIdentityId:)`. */
    @Query(
        "SELECT * FROM dashpay_contact_profiles WHERE ownerIdentityId = :ownerIdentityId " +
            "AND contactIdentityId = :contactIdentityId"
    )
    fun observeContactProfile(
        ownerIdentityId: ByteArray,
        contactIdentityId: ByteArray,
    ): Flow<DashpayContactProfileEntity?>

    /** Persister upsert (`is_present == true` changeset entry). */
    @Upsert
    suspend fun upsertContactProfile(profile: DashpayContactProfileEntity)

    /**
     * Persister tombstone (`is_present == false` changeset entry): the
     * contact removed their on-chain profile, so the cached row must go —
     * an upsert-only pipeline would leave a stale name/avatar forever.
     */
    @Query(
        "DELETE FROM dashpay_contact_profiles WHERE networkRaw = :networkRaw " +
            "AND ownerIdentityId = :ownerIdentityId " +
            "AND contactIdentityId = :contactIdentityId"
    )
    suspend fun deleteContactProfile(
        networkRaw: Int,
        ownerIdentityId: ByteArray,
        contactIdentityId: ByteArray,
    )

    /** Restore-path read: every cached contact profile owned by [ownerIdentityId]. */
    @Query("SELECT * FROM dashpay_contact_profiles WHERE ownerIdentityId = :ownerIdentityId")
    suspend fun getContactProfilesByOwner(
        ownerIdentityId: ByteArray,
    ): List<DashpayContactProfileEntity>

    @Query("DELETE FROM dashpay_contact_profiles")
    suspend fun deleteAllContactProfiles()

    /** StorageExplorer row count. */
    @Query("SELECT COUNT(*) FROM dashpay_contact_profiles")
    fun countContactProfiles(): Flow<Long>

    /** StorageExplorer network-scoped row count. */
    @Query("SELECT COUNT(*) FROM dashpay_contact_profiles WHERE networkRaw = :networkRaw")
    fun countContactProfilesByNetwork(networkRaw: Int): Flow<Long>

    // MARK: Payments (pull-persisted; see DashpayPaymentEntity KDoc)

    /** Mirror of `PersistentDashpayPayment.predicate(ownerIdentityId:)`. */
    @Query("SELECT * FROM dashpay_payments WHERE ownerIdentityId = :ownerIdentityId")
    fun observePayments(ownerIdentityId: ByteArray): Flow<List<DashpayPaymentEntity>>

    /**
     * Mirror of `predicate(ownerIdentityId:counterpartyIdentityId:)` —
     * the payment list on the contact-detail screen shows only the
     * history with that one contact.
     */
    @Query(
        "SELECT * FROM dashpay_payments WHERE ownerIdentityId = :ownerIdentityId " +
            "AND counterpartyIdentityId = :counterpartyIdentityId"
    )
    fun observePayments(
        ownerIdentityId: ByteArray,
        counterpartyIdentityId: ByteArray,
    ): Flow<List<DashpayPaymentEntity>>

    @Upsert
    suspend fun upsertPayments(payments: List<DashpayPaymentEntity>)

    /** Restore-path read: every payment row owned by [ownerIdentityId]. */
    @Query("SELECT * FROM dashpay_payments WHERE ownerIdentityId = :ownerIdentityId")
    suspend fun getPaymentsByOwner(ownerIdentityId: ByteArray): List<DashpayPaymentEntity>

    @Query("DELETE FROM dashpay_payments")
    suspend fun deleteAllPayments()

    /** StorageExplorer row count. */
    @Query("SELECT COUNT(*) FROM dashpay_payments")
    fun countPayments(): Flow<Long>

    /** StorageExplorer network-scoped row count. */
    @Query("SELECT COUNT(*) FROM dashpay_payments WHERE networkRaw = :networkRaw")
    fun countPaymentsByNetwork(networkRaw: Int): Flow<Long>
}
