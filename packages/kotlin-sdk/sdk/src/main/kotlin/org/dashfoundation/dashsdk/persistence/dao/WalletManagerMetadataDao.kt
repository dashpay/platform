package org.dashfoundation.dashsdk.persistence.dao

import androidx.room.Dao
import androidx.room.Query
import androidx.room.Upsert
import kotlinx.coroutines.flow.Flow
import org.dashfoundation.dashsdk.persistence.entities.WalletManagerMetadataEntity

/**
 * Queries over [WalletManagerMetadataEntity] — the per-network singleton
 * row mirroring `PersistentWalletManagerMetadata.swift`.
 */
@Dao
interface WalletManagerMetadataDao {

    @Query("SELECT * FROM wallet_manager_metadata WHERE networkRaw = :networkRaw")
    fun observeByNetwork(networkRaw: Int): Flow<WalletManagerMetadataEntity?>

    @Query("SELECT * FROM wallet_manager_metadata WHERE networkRaw = :networkRaw")
    suspend fun getByNetwork(networkRaw: Int): WalletManagerMetadataEntity?

    @Upsert
    suspend fun upsert(metadata: WalletManagerMetadataEntity)

    @Query("DELETE FROM wallet_manager_metadata WHERE networkRaw = :networkRaw")
    suspend fun deleteByNetwork(networkRaw: Int)

    @Query("DELETE FROM wallet_manager_metadata")
    suspend fun deleteAll()

    @Query("SELECT COUNT(*) FROM wallet_manager_metadata")
    fun count(): Flow<Long>
}
