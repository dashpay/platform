package org.dashfoundation.dashsdk.persistence.dao

import androidx.room.Dao
import androidx.room.Delete
import androidx.room.Query
import androidx.room.Upsert
import kotlinx.coroutines.flow.Flow
import org.dashfoundation.dashsdk.persistence.entities.PlatformAddressEntity
import org.dashfoundation.dashsdk.persistence.entities.PlatformAddressesSyncStateEntity

/**
 * Queries over [PlatformAddressEntity] and
 * [PlatformAddressesSyncStateEntity], mirroring
 * `PersistentPlatformAddress.predicate(walletId:)`,
 * `nonZeroBalancesPredicate`, the BLAST upsert-by-`addressHash` lookups
 * (KeychainSigner / SendViewModel / persistence handler), and the
 * sync-state row keyed by the network-scoped pseudo `walletId` plus the
 * `networkRaw` queries in WalletDetailView / SendTransactionView.
 */
@Dao
interface PlatformAddressDao {

    // MARK: Addresses

    /** Mirror of `predicate(walletId:)`. */
    @Query("SELECT * FROM platform_addresses WHERE walletId = :walletId")
    fun observeByWallet(walletId: ByteArray): Flow<List<PlatformAddressEntity>>

    /** Mirror of `nonZeroBalancesPredicate`. */
    @Query("SELECT * FROM platform_addresses WHERE balance > 0")
    fun observeNonZeroBalances(): Flow<List<PlatformAddressEntity>>

    /** Persister upsert key (`$0.address == address`). */
    @Query("SELECT * FROM platform_addresses WHERE address = :address")
    suspend fun getByAddress(address: String): PlatformAddressEntity?

    /** BLAST balance callback / signer lookup (`$0.addressHash == hash`). */
    @Query("SELECT * FROM platform_addresses WHERE addressHash = :addressHash")
    suspend fun getByAddressHash(addressHash: ByteArray): PlatformAddressEntity?

    @Upsert
    suspend fun upsert(address: PlatformAddressEntity)

    @Delete
    suspend fun delete(address: PlatformAddressEntity)

    @Query("DELETE FROM platform_addresses WHERE walletId = :walletId")
    suspend fun deleteByWallet(walletId: ByteArray)

    @Query("DELETE FROM platform_addresses")
    suspend fun deleteAll()

    /** StorageExplorer row count. */
    @Query("SELECT COUNT(*) FROM platform_addresses")
    fun count(): Flow<Long>

    @Query("SELECT COUNT(*) FROM platform_addresses WHERE walletId = :walletId")
    fun countByWallet(walletId: ByteArray): Flow<Long>

    // MARK: Sync state

    /** Watermark row by its network-scoped pseudo wallet id. */
    @Query("SELECT * FROM platform_addresses_sync_states WHERE walletId = :walletId")
    suspend fun getSyncState(walletId: ByteArray): PlatformAddressesSyncStateEntity?

    /** WalletDetailView / SendTransactionView: `$0.networkRaw == raw`. */
    @Query("SELECT * FROM platform_addresses_sync_states WHERE networkRaw = :networkRaw")
    fun observeSyncStatesByNetwork(networkRaw: Int): Flow<List<PlatformAddressesSyncStateEntity>>

    @Upsert
    suspend fun upsertSyncState(state: PlatformAddressesSyncStateEntity)

    @Query("DELETE FROM platform_addresses_sync_states WHERE walletId = :walletId")
    suspend fun deleteSyncState(walletId: ByteArray)

    @Query("DELETE FROM platform_addresses_sync_states")
    suspend fun deleteAllSyncStates()

    /** StorageExplorer row count. */
    @Query("SELECT COUNT(*) FROM platform_addresses_sync_states")
    fun countSyncStates(): Flow<Long>

    /** StorageExplorer network-scoped row count. */
    @Query("SELECT COUNT(*) FROM platform_addresses_sync_states WHERE networkRaw = :networkRaw")
    fun countSyncStatesByNetwork(networkRaw: Int): Flow<Long>
}
