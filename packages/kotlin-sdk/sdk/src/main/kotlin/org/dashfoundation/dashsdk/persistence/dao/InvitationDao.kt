package org.dashfoundation.dashsdk.persistence.dao

import androidx.room.Dao
import androidx.room.Query
import androidx.room.Upsert
import kotlinx.coroutines.flow.Flow
import org.dashfoundation.dashsdk.persistence.entities.InvitationEntity

/**
 * Queries over [InvitationEntity], mirroring the Swift call sites: the
 * `InvitationsView` `@Query` (all rows, newest first, filtered to loaded
 * wallets in the UI), the reclaim sheet's per-row status/marker saves, and
 * the persistence handler's upsert / delete by `outPointHex`.
 */
@Dao
interface InvitationDao {

    /** "Sent invitations" list source (newest first; UI filters to loaded wallets). */
    @Query("SELECT * FROM invitations ORDER BY createdAtSecs DESC")
    fun observeAll(): Flow<List<InvitationEntity>>

    @Query(
        "SELECT * FROM invitations WHERE walletId = :walletId " +
            "ORDER BY createdAtSecs DESC"
    )
    fun observeByWallet(walletId: ByteArray): Flow<List<InvitationEntity>>

    @Query("SELECT * FROM invitations WHERE outPointHex = :outPointHex")
    suspend fun getByOutPointHex(outPointHex: String): InvitationEntity?

    @Upsert
    suspend fun upsert(invitation: InvitationEntity)

    /**
     * Reclaim-flow terminal save: status + marker in one statement so the
     * row can never hold a terminal status with a stale in-flight marker.
     */
    @Query(
        "UPDATE invitations SET statusRaw = :statusRaw, " +
            "reclaimInFlight = :reclaimInFlight, updatedAt = :updatedAtMillis " +
            "WHERE outPointHex = :outPointHex"
    )
    suspend fun setStatusAndMarker(
        outPointHex: String,
        statusRaw: Int,
        reclaimInFlight: Boolean,
        updatedAtMillis: Long,
    )

    /** Pre-consume marker write (the crash-forensics flag on its own). */
    @Query(
        "UPDATE invitations SET reclaimInFlight = :reclaimInFlight, " +
            "updatedAt = :updatedAtMillis WHERE outPointHex = :outPointHex"
    )
    suspend fun setReclaimInFlight(
        outPointHex: String,
        reclaimInFlight: Boolean,
        updatedAtMillis: Long,
    )

    /** Removal path of the persistence callback. */
    @Query("DELETE FROM invitations WHERE outPointHex = :outPointHex")
    suspend fun deleteByOutPointHex(outPointHex: String)

    /** Wallet teardown mirror of `deleteWalletData`'s per-wallet pass. */
    @Query("DELETE FROM invitations WHERE walletId = :walletId")
    suspend fun deleteByWallet(walletId: ByteArray)

    @Query("DELETE FROM invitations")
    suspend fun deleteAll()

    @Query("SELECT COUNT(*) FROM invitations")
    fun count(): Flow<Long>
}
