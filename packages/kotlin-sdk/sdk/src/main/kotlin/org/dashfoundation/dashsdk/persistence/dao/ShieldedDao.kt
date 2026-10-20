package org.dashfoundation.dashsdk.persistence.dao

import androidx.room.Dao
import androidx.room.Delete
import androidx.room.Query
import androidx.room.Upsert
import kotlinx.coroutines.flow.Flow
import org.dashfoundation.dashsdk.persistence.entities.ShieldedActivityEntity
import org.dashfoundation.dashsdk.persistence.entities.ShieldedNoteEntity
import org.dashfoundation.dashsdk.persistence.entities.ShieldedOutgoingNoteEntity
import org.dashfoundation.dashsdk.persistence.entities.ShieldedSyncStateEntity
import org.dashfoundation.dashsdk.persistence.entities.ShieldedViewingKeyEntity

/**
 * Queries over the shielded (Orchard) family — [ShieldedNoteEntity],
 * [ShieldedOutgoingNoteEntity], [ShieldedActivityEntity],
 * [ShieldedSyncStateEntity], [ShieldedViewingKeyEntity].
 *
 * Reads mirror the Swift call sites: upsert by nullifier / `(walletId,
 * accountIndex, cmx)` / `(walletId, accountIndex, entryId)` /
 * `(walletId, accountIndex)` in `PlatformWalletPersistenceHandler.swift`,
 * the unspent-balance scan in CoreContentView
 * (`$0.walletId == walletId && !$0.isSpent`), the activity timeline in
 * ShieldedActivityView (per-wallet, block-height sort), and the per-wallet
 * teardown deletes.
 */
@Dao
interface ShieldedDao {

    // MARK: Notes (received, spendable)

    @Query("SELECT * FROM shielded_notes WHERE nullifier = :nullifier")
    suspend fun getNoteByNullifier(nullifier: ByteArray): ShieldedNoteEntity?

    @Query("SELECT * FROM shielded_notes WHERE walletId = :walletId")
    fun observeNotesByWallet(walletId: ByteArray): Flow<List<ShieldedNoteEntity>>

    /**
     * Load-path snapshot of every note (mirror of the Swift loader's
     * whole-table fetch before its network filter). Shielded rows carry
     * no wallet FK, so restore reads them directly.
     */
    @Query("SELECT * FROM shielded_notes")
    suspend fun getAllNotes(): List<ShieldedNoteEntity>

    @Query("SELECT * FROM shielded_notes WHERE walletId = :walletId AND accountIndex = :accountIndex")
    fun observeNotesByWalletAccount(
        walletId: ByteArray,
        accountIndex: Int,
    ): Flow<List<ShieldedNoteEntity>>

    /** CoreContentView balance scan: `$0.walletId == walletId && !$0.isSpent`. */
    @Query("SELECT * FROM shielded_notes WHERE walletId = :walletId AND isSpent = 0")
    fun observeUnspentNotesByWallet(walletId: ByteArray): Flow<List<ShieldedNoteEntity>>

    @Upsert
    suspend fun upsertNote(note: ShieldedNoteEntity)

    @Delete
    suspend fun deleteNote(note: ShieldedNoteEntity)

    /** Wallet teardown mirror of `deleteWalletData`'s shielded-note pass. */
    @Query("DELETE FROM shielded_notes WHERE walletId = :walletId")
    suspend fun deleteNotesByWallet(walletId: ByteArray)

    @Query("DELETE FROM shielded_notes")
    suspend fun deleteAllNotes()

    /** StorageExplorer row count. */
    @Query("SELECT COUNT(*) FROM shielded_notes")
    fun countNotes(): Flow<Long>

    // MARK: Outgoing (sent) notes

    /** Persister upsert key (the Swift `#Unique` triple). */
    @Query(
        "SELECT * FROM shielded_outgoing_notes WHERE walletId = :walletId " +
            "AND accountIndex = :accountIndex AND cmx = :cmx"
    )
    suspend fun getOutgoingNote(
        walletId: ByteArray,
        accountIndex: Int,
        cmx: ByteArray,
    ): ShieldedOutgoingNoteEntity?

    @Query("SELECT * FROM shielded_outgoing_notes WHERE walletId = :walletId")
    fun observeOutgoingNotesByWallet(walletId: ByteArray): Flow<List<ShieldedOutgoingNoteEntity>>

    /** Load-path whole-table snapshot (see [getAllNotes]). */
    @Query("SELECT * FROM shielded_outgoing_notes")
    suspend fun getAllOutgoingNotes(): List<ShieldedOutgoingNoteEntity>

    @Query(
        "SELECT * FROM shielded_outgoing_notes WHERE walletId = :walletId " +
            "AND accountIndex = :accountIndex"
    )
    fun observeOutgoingNotesByWalletAccount(
        walletId: ByteArray,
        accountIndex: Int,
    ): Flow<List<ShieldedOutgoingNoteEntity>>

    @Upsert
    suspend fun upsertOutgoingNote(note: ShieldedOutgoingNoteEntity)

    @Delete
    suspend fun deleteOutgoingNote(note: ShieldedOutgoingNoteEntity)

    /** Wallet teardown mirror. */
    @Query("DELETE FROM shielded_outgoing_notes WHERE walletId = :walletId")
    suspend fun deleteOutgoingNotesByWallet(walletId: ByteArray)

    @Query("DELETE FROM shielded_outgoing_notes")
    suspend fun deleteAllOutgoingNotes()

    /** StorageExplorer row count. */
    @Query("SELECT COUNT(*) FROM shielded_outgoing_notes")
    fun countOutgoingNotes(): Flow<Long>

    // MARK: Activity

    /** Persister upsert key (the Swift `#Unique` triple). */
    @Query(
        "SELECT * FROM shielded_activities WHERE walletId = :walletId " +
            "AND accountIndex = :accountIndex AND entryId = :entryId"
    )
    suspend fun getActivity(
        walletId: ByteArray,
        accountIndex: Int,
        entryId: ByteArray,
    ): ShieldedActivityEntity?

    /**
     * ShieldedActivityView timeline: per-wallet, confirmed by height desc
     * with the `createdAtMs` tiebreak (pendings are re-floated in code, as
     * in Swift).
     */
    @Query(
        "SELECT * FROM shielded_activities WHERE walletId = :walletId " +
            "ORDER BY blockHeight DESC, createdAtMs DESC"
    )
    fun observeActivityByWallet(walletId: ByteArray): Flow<List<ShieldedActivityEntity>>

    /** Load-path whole-table snapshot (see [getAllNotes]). */
    @Query("SELECT * FROM shielded_activities")
    suspend fun getAllActivity(): List<ShieldedActivityEntity>

    @Query(
        "SELECT * FROM shielded_activities WHERE walletId = :walletId " +
            "AND accountIndex = :accountIndex ORDER BY blockHeight DESC, createdAtMs DESC"
    )
    fun observeActivityByWalletAccount(
        walletId: ByteArray,
        accountIndex: Int,
    ): Flow<List<ShieldedActivityEntity>>

    @Upsert
    suspend fun upsertActivity(activity: ShieldedActivityEntity)

    @Delete
    suspend fun deleteActivity(activity: ShieldedActivityEntity)

    /** Wallet teardown mirror. */
    @Query("DELETE FROM shielded_activities WHERE walletId = :walletId")
    suspend fun deleteActivityByWallet(walletId: ByteArray)

    @Query("DELETE FROM shielded_activities")
    suspend fun deleteAllActivity()

    /** StorageExplorer row count. */
    @Query("SELECT COUNT(*) FROM shielded_activities")
    fun countActivity(): Flow<Long>

    // MARK: Sync state

    /** Persister upsert key (`row.walletId == walletId && row.accountIndex == accountIndex`). */
    @Query(
        "SELECT * FROM shielded_sync_states WHERE walletId = :walletId " +
            "AND accountIndex = :accountIndex"
    )
    suspend fun getSyncState(walletId: ByteArray, accountIndex: Int): ShieldedSyncStateEntity?

    @Query("SELECT * FROM shielded_sync_states WHERE walletId = :walletId")
    fun observeSyncStatesByWallet(walletId: ByteArray): Flow<List<ShieldedSyncStateEntity>>

    /** Load-path whole-table snapshot (see [getAllNotes]). */
    @Query("SELECT * FROM shielded_sync_states")
    suspend fun getAllSyncStates(): List<ShieldedSyncStateEntity>

    @Upsert
    suspend fun upsertSyncState(state: ShieldedSyncStateEntity)

    /** Wallet teardown mirror. */
    @Query("DELETE FROM shielded_sync_states WHERE walletId = :walletId")
    suspend fun deleteSyncStatesByWallet(walletId: ByteArray)

    @Query("DELETE FROM shielded_sync_states")
    suspend fun deleteAllSyncStates()

    /** StorageExplorer row count. */
    @Query("SELECT COUNT(*) FROM shielded_sync_states")
    fun countSyncStates(): Flow<Long>

    // MARK: Orchard full viewing keys

    @Query(
        "SELECT * FROM shielded_viewing_keys WHERE walletId = :walletId " +
            "AND accountIndex = :accountIndex"
    )
    suspend fun getViewingKey(
        walletId: ByteArray,
        accountIndex: Int,
    ): ShieldedViewingKeyEntity?

    @Query("SELECT * FROM shielded_viewing_keys WHERE walletId = :walletId ORDER BY accountIndex")
    fun observeViewingKeysByWallet(walletId: ByteArray): Flow<List<ShieldedViewingKeyEntity>>

    /** Network-scoped restore helper; filters in SQL before fixed-size entity validation. */
    @Query("SELECT * FROM shielded_viewing_keys WHERE walletId = :walletId ORDER BY accountIndex")
    suspend fun getViewingKeysByWallet(walletId: ByteArray): List<ShieldedViewingKeyEntity>

    /** Restore snapshot. Entity construction validates every row is exactly 32/96 bytes. */
    @Query("SELECT * FROM shielded_viewing_keys ORDER BY walletId, accountIndex")
    suspend fun getAllViewingKeys(): List<ShieldedViewingKeyEntity>

    @Upsert
    suspend fun upsertViewingKey(viewingKey: ShieldedViewingKeyEntity)

    @Query("DELETE FROM shielded_viewing_keys WHERE walletId = :walletId")
    suspend fun deleteViewingKeysByWallet(walletId: ByteArray)

    @Query("DELETE FROM shielded_viewing_keys")
    suspend fun deleteAllViewingKeys()

    @Query("SELECT COUNT(*) FROM shielded_viewing_keys")
    fun countViewingKeys(): Flow<Long>
}
