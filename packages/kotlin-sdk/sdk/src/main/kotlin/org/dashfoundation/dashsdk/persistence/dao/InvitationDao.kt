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

    @Query("SELECT * FROM invitations WHERE outPointHex = :outPointHex")
    suspend fun getByOutPointHex(outPointHex: String): InvitationEntity?

    @Upsert
    suspend fun upsert(invitation: InvitationEntity)

    /**
     * Reclaim-flow terminal save: status + marker in one statement so the
     * row can never hold a terminal status with a stale in-flight marker.
     *
     * @return rows updated — `0` means the row vanished (e.g. a concurrent
     *   wallet deletion); callers that gate irreversible work on this write
     *   must treat `0` as failure, not success.
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
    ): Int

    /**
     * Pre-consume marker write (the crash-forensics flag on its own).
     * @return rows updated — see [setStatusAndMarker]'s `0` contract.
     */
    @Query(
        "UPDATE invitations SET reclaimInFlight = :reclaimInFlight, " +
            "updatedAt = :updatedAtMillis WHERE outPointHex = :outPointHex"
    )
    suspend fun setReclaimInFlight(
        outPointHex: String,
        reclaimInFlight: Boolean,
        updatedAtMillis: Long,
    ): Int

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
