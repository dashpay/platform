package org.dashfoundation.dashsdk.persistence.dao

import androidx.room.Dao
import androidx.room.Insert
import androidx.room.OnConflictStrategy
import androidx.room.Query
import kotlinx.coroutines.flow.Flow
import org.dashfoundation.dashsdk.persistence.entities.WalletDrainOperationEntity

/**
 * Queries over [WalletDrainOperationEntity], mirroring
 * `WalletDrainOperationStore` in
 * `packages/swift-sdk/SwiftExampleApp/SwiftExampleApp/Services/WalletDrainOperationStore.swift`:
 * `records()`, `insert(_:)`, `remove(operationId:)` and `remove(walletId:)`.
 */
@Dao
interface WalletDrainOperationDao {

    /** Every stored operation, on every network, oldest first. */
    @Query("SELECT * FROM wallet_drain_operations ORDER BY createdAtMillis, operationId")
    suspend fun getAll(): List<WalletDrainOperationEntity>

    /** [getAll] as a stream, for the saved-transfers list. */
    @Query("SELECT * FROM wallet_drain_operations ORDER BY createdAtMillis, operationId")
    fun observeAll(): Flow<List<WalletDrainOperationEntity>>

    /**
     * Store [operation] before its drain is called. ABORT, not REPLACE: an
     * id is minted fresh for each transfer, so a conflict is a caller bug
     * and must not silently rewrite the recipient a resume pays.
     */
    @Insert(onConflict = OnConflictStrategy.ABORT)
    suspend fun insert(operation: WalletDrainOperationEntity)

    /** Returns the number of rows removed (0 when it was already gone). */
    @Query("DELETE FROM wallet_drain_operations WHERE operationId = :operationId")
    suspend fun deleteByOperationId(operationId: ByteArray): Int

    /** Wallet teardown: a re-imported wallet must not inherit these. */
    @Query("DELETE FROM wallet_drain_operations WHERE walletId = :walletId")
    suspend fun deleteByWallet(walletId: ByteArray)

    @Query("DELETE FROM wallet_drain_operations")
    suspend fun deleteAll()

    /** StorageExplorer row count. */
    @Query("SELECT COUNT(*) FROM wallet_drain_operations")
    fun count(): Flow<Long>
}
