package org.dashfoundation.dashsdk.persistence.dao

import androidx.room.Dao
import androidx.room.Delete
import androidx.room.Insert
import androidx.room.OnConflictStrategy
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

    /**
     * Insert [identity] unless a row with its id exists. Returns the new row id, or -1 when
     * a row was already there and nothing was written.
     */
    @Insert(onConflict = OnConflictStrategy.IGNORE)
    suspend fun insertIfAbsent(identity: IdentityEntity): Long

    /**
     * Load Identity's write to a row already on the device: only the fields a load owns, on
     * the current row, and only while the row is on [networkRaw]. The fetched [balance]
     * replaces the stored one only on a row no wallet owns: a wallet's own identity has its
     * balance kept current by wallet sync, which may have written a newer one than the fetch
     * saw. The alias and the type change only when one is given (a null [identityType] keeps
     * the stored type, so a type the user did not pick never overwrites a newer one), and
     * so does the wire-order [masternodeProTxHash] of a masternode or evonode loaded for
     * voting (null keeps the recorded one), in the same statement, so a load never leaves a
     * row without it. Every other column (wallet link, key identifiers, DPNS names) is left as
     * the row has it. Returns the number of rows changed (0 when the row is gone or on another
     * network).
     */
    @Query(
        "UPDATE identities SET " +
            "balance = CASE WHEN walletId IS NULL THEN :balance ELSE balance END, " +
            "alias = COALESCE(:alias, alias), " +
            "isLocal = :isLocal, identityType = COALESCE(:identityType, identityType), " +
            "masternodeProTxHash = COALESCE(:masternodeProTxHash, masternodeProTxHash), " +
            "masternodeRoleRaw = COALESCE(:masternodeRoleRaw, masternodeRoleRaw), " +
            "lastUpdated = :nowMillis " +
            "WHERE identityId = :identityId AND networkRaw = :networkRaw"
    )
    suspend fun updateLoadedFields(
        identityId: ByteArray,
        networkRaw: Int,
        balance: Long,
        alias: String?,
        isLocal: Boolean,
        identityType: String?,
        nowMillis: Long,
        masternodeProTxHash: ByteArray? = null,
        masternodeRoleRaw: String? = null,
    ): Int

    /**
     * Point the row at the voting key [newIdentifier] only while it is on [networkRaw] and
     * still names [expectedIdentifier] (null for none), so a key association changed by
     * another write since it was read, or a row recreated on another network, is never
     * overwritten. Returns the number of rows changed: 0 when the row is gone, on another
     * network, or names another key.
     *
     * The low-level row write of `MasternodeVotingKeyStore.assignToIdentity`, which also
     * writes the key and runs both under the voting key lock. Use that instead: changing the
     * association here alone can name a key that is not stored, or race a clear.
     */
    @Query(
        "UPDATE identities SET votingPrivateKeyIdentifier = :newIdentifier, " +
            "lastUpdated = :nowMillis " +
            "WHERE identityId = :identityId AND networkRaw = :networkRaw " +
            "AND votingPrivateKeyIdentifier IS :expectedIdentifier"
    )
    suspend fun replaceVotingPrivateKeyIdentifier(
        identityId: ByteArray,
        networkRaw: Int,
        expectedIdentifier: String?,
        newIdentifier: String?,
        nowMillis: Long,
    ): Int

    /**
     * Record a check of the voting key [identifier] names: only while the row is on
     * [networkRaw] and still names [identifier], so an answer about a key replaced meanwhile
     * is never written on its successor. Returns the number of rows changed. The low-level
     * write of `MasternodeVotingKeyRecords.record`, which writes what Rust says to record;
     * use that instead.
     */
    @Query(
        "UPDATE identities SET votingKeyCheckRaw = :raw, votingKeyCheckMessage = :message, " +
            "votingKeyCheckAddress = :address " +
            "WHERE identityId = :identityId AND networkRaw = :networkRaw " +
            "AND votingPrivateKeyIdentifier = :identifier"
    )
    suspend fun recordVotingKeyCheck(
        identityId: ByteArray,
        networkRaw: Int,
        identifier: String,
        raw: String,
        message: String?,
        address: ByteArray,
    ): Int

    /**
     * The rows on [networkRaw] that record [proTxHash] as the masternode they vote for: one per
     * voting key a rotation left on this device (Swift's `masternodeProTxHash == target` fetch).
     */
    @Query("SELECT * FROM identities WHERE networkRaw = :networkRaw AND masternodeProTxHash = :proTxHash")
    suspend fun getByMasternodeProTxHash(networkRaw: Int, proTxHash: ByteArray): List<IdentityEntity>

    /**
     * Record the pro_tx_hash and role Rust's load rule resolved for a row stored before they
     * were recorded, only while the row is on [networkRaw] and still records no pro_tx_hash.
     * Returns the number of rows changed.
     */
    @Query(
        "UPDATE identities SET masternodeProTxHash = :proTxHash, masternodeRoleRaw = :roleRaw " +
            "WHERE identityId = :identityId AND networkRaw = :networkRaw AND masternodeProTxHash IS NULL"
    )
    suspend fun recordResolvedMasternode(identityId: ByteArray, networkRaw: Int, proTxHash: ByteArray, roleRaw: String): Int

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

    /** Whether any identity row names the voting key [identifier]. */
    @Query("SELECT EXISTS(SELECT 1 FROM identities WHERE votingPrivateKeyIdentifier = :identifier)")
    suspend fun existsByVotingPrivateKeyIdentifier(identifier: String): Boolean

    /** StorageExplorer network-scoped row count. */
    @Query("SELECT COUNT(*) FROM identities WHERE networkRaw = :networkRaw")
    fun countByNetwork(networkRaw: Int): Flow<Long>
}
