package org.dashfoundation.dashsdk.persistence.dao

import androidx.room.Dao
import androidx.room.Delete
import androidx.room.Query
import androidx.room.Upsert
import kotlinx.coroutines.flow.Flow
import org.dashfoundation.dashsdk.persistence.entities.IdentityEntity

/**
 * Queries over [IdentityEntity], mirroring the `PersistentIdentity`
 * predicate helpers (`predicate(identityId:)`, `predicate(network:)`,
 * `walletOwnedIdentitiesPredicate(…)`, `predicate(type:)`,
 * `needsSyncPredicate(olderThan:)`) plus the example app's per-wallet /
 * per-slot queries and the mutation helpers
 * (`updateBalance` / `updateDpnsName` / `updateMainDpnsName` / `remove`).
 */
@Dao
interface IdentityDao {

    @Query("SELECT * FROM identities")
    fun observeAll(): Flow<List<IdentityEntity>>

    @Query("SELECT * FROM identities WHERE identityId = :identityId")
    fun observeByIdentityId(identityId: ByteArray): Flow<IdentityEntity?>

    @Query("SELECT * FROM identities WHERE identityId = :identityId")
    suspend fun getByIdentityId(identityId: ByteArray): IdentityEntity?

    /** Mirror of `predicate(network:)`. */
    @Query("SELECT * FROM identities WHERE networkRaw = :networkRaw")
    fun observeByNetwork(networkRaw: Int): Flow<List<IdentityEntity>>

    /** Mirror of `walletOwnedIdentitiesPredicate` (`identity.wallet != nil`). */
    @Query("SELECT * FROM identities WHERE walletId IS NOT NULL")
    fun observeWalletOwned(): Flow<List<IdentityEntity>>

    /** Mirror of `walletOwnedIdentitiesPredicate(network:)`. */
    @Query("SELECT * FROM identities WHERE walletId IS NOT NULL AND networkRaw = :networkRaw")
    fun observeWalletOwnedByNetwork(networkRaw: Int): Flow<List<IdentityEntity>>

    /** WalletDetailView: `$0.wallet?.walletId == walletId`. */
    @Query("SELECT * FROM identities WHERE walletId = :walletId ORDER BY identityIndex")
    fun observeByWallet(walletId: ByteArray): Flow<List<IdentityEntity>>

    /** PendingRegistrationsList: wallet + DIP-9 slot. */
    @Query(
        "SELECT * FROM identities WHERE walletId = :walletId " +
            "AND identityIndex = :identityIndex"
    )
    fun observeByWalletAndIdentityIndex(
        walletId: ByteArray,
        identityIndex: Int,
    ): Flow<List<IdentityEntity>>

    /** StorageRecordDetailViews: slot-only lookup. */
    @Query("SELECT * FROM identities WHERE identityIndex = :identityIndex")
    fun observeByIdentityIndex(identityIndex: Int): Flow<List<IdentityEntity>>

    /** Mirror of `predicate(type:)` (rawValue string). */
    @Query("SELECT * FROM identities WHERE identityType = :identityType")
    fun observeByType(identityType: String): Flow<List<IdentityEntity>>

    /** Mirror of `needsSyncPredicate(olderThan:)`; epoch millis. */
    @Query("SELECT * FROM identities WHERE lastSyncedAt IS NULL OR lastSyncedAt < :olderThanMillis")
    suspend fun getNeedingSync(olderThanMillis: Long): List<IdentityEntity>

    @Upsert
    suspend fun upsert(identity: IdentityEntity)

    /** Mirror of `PersistentIdentity.updateBalance(in:identityId:balance:)`. */
    @Query(
        "UPDATE identities SET balance = :balance, lastUpdated = :nowMillis " +
            "WHERE identityId = :identityId"
    )
    suspend fun updateBalance(identityId: ByteArray, balance: Long, nowMillis: Long): Int

    /** Mirror of `PersistentIdentity.updateDpnsName`. */
    @Query(
        "UPDATE identities SET dpnsName = :dpnsName, lastUpdated = :nowMillis " +
            "WHERE identityId = :identityId"
    )
    suspend fun updateDpnsName(identityId: ByteArray, dpnsName: String?, nowMillis: Long): Int

    /** Mirror of `PersistentIdentity.updateMainDpnsName`. */
    @Query(
        "UPDATE identities SET mainDpnsName = :mainDpnsName, lastUpdated = :nowMillis " +
            "WHERE identityId = :identityId"
    )
    suspend fun updateMainDpnsName(identityId: ByteArray, mainDpnsName: String?, nowMillis: Long): Int

    /**
     * One-shot upgrade heal — promote `isLocal` on wallet-linked rows
     * still carrying `false`. The persister used to write a constant
     * `false`, so a wallet's own identities (which are always local) were
     * mis-marked on stores from that era.
     *
     * Promote-only and idempotent: a `true` on an unlinked row (a manual
     * add) is never touched, and a second run matches no rows. Returns the
     * number of rows healed. Mirror of Swift `healIdentityIsLocalFlags`
     * (PlatformWalletPersistenceHandler.swift:4688).
     */
    @Query("UPDATE identities SET isLocal = 1 WHERE walletId IS NOT NULL AND isLocal = 0")
    suspend fun healIsLocalFlags(): Int

    @Delete
    suspend fun delete(identity: IdentityEntity)

    /** Mirror of `PersistentIdentity.remove(in:identityId:)`. */
    @Query("DELETE FROM identities WHERE identityId = :identityId")
    suspend fun deleteByIdentityId(identityId: ByteArray)

    @Query("DELETE FROM identities")
    suspend fun deleteAll()

    /** StorageExplorer row count. */
    @Query("SELECT COUNT(*) FROM identities")
    fun count(): Flow<Long>

    /** StorageExplorer network-scoped row count. */
    @Query("SELECT COUNT(*) FROM identities WHERE networkRaw = :networkRaw")
    fun countByNetwork(networkRaw: Int): Flow<Long>
}
