package org.dashfoundation.dashsdk.persistence.dao

import androidx.room.Dao
import androidx.room.OnConflictStrategy
import androidx.room.Insert
import androidx.room.Query
import kotlinx.coroutines.flow.Flow
import org.dashfoundation.dashsdk.persistence.entities.InvitationEntity

/**
 * Queries over [InvitationEntity], mirroring the invitation persistence
 * bridge call sites: the handler's upsert (`onPersistInvitationUpsert`) and
 * delete-by-outpoint (`onPersistInvitationRemoval`), plus wallet teardown.
 *
 * Upsert is REPLACE-on-conflict keyed by the 36-byte `outPoint` PK so a
 * status transition (Created → Claimed → Reclaimed) overwrites the row in
 * place.
 */
@Dao
interface InvitationDao {

    @Insert(onConflict = OnConflictStrategy.REPLACE)
    suspend fun upsert(invitation: InvitationEntity)

    @Query("SELECT * FROM invitations WHERE outPoint = :outPoint")
    suspend fun getByOutPoint(outPoint: ByteArray): InvitationEntity?

    @Query("SELECT * FROM invitations WHERE walletId = :walletId")
    fun observeByWallet(walletId: ByteArray): Flow<List<InvitationEntity>>

    /** Removal path (`onPersistInvitationRemoval`). */
    @Query("DELETE FROM invitations WHERE outPoint = :outPoint")
    suspend fun deleteByOutPoint(outPoint: ByteArray)

    /** Wallet teardown mirror of `deleteWalletData`. */
    @Query("DELETE FROM invitations WHERE walletId = :walletId")
    suspend fun deleteByWallet(walletId: ByteArray)

    @Query("DELETE FROM invitations")
    suspend fun deleteAll()

    @Query("SELECT COUNT(*) FROM invitations")
    fun count(): Flow<Long>

    @Query("SELECT COUNT(*) FROM invitations WHERE walletId = :walletId")
    fun countByWallet(walletId: ByteArray): Flow<Long>
}
