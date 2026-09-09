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

    /**
     * Persister upsert key — the composite row identity `(walletId,
     * address)`. An address-only lookup could return another wallet's row
     * (same seed imported twice derives identical addresses) and the
     * pool-emit upsert would then reassign that row's `walletId`/
     * `accountId`, corrupting the other wallet's balances.
     */
    @Query(
        "SELECT * FROM platform_addresses WHERE walletId = :walletId AND address = :address",
    )
    suspend fun getByWalletAndAddress(
        walletId: ByteArray,
        address: String,
    ): PlatformAddressEntity?

    /**
     * Every row carrying [addressHash], across wallets, in deterministic
     * order. The signer callback arrives with the 20-byte hash only (no
     * wallet context), and with per-wallet uniqueness several wallets may
     * hold the hash — the caller scans for a row it can actually sign
     * with (non-empty derivation path + stored mnemonic; any such row
     * derives the same private key, since the hash pins the key).
     */
    @Query(
        "SELECT * FROM platform_addresses WHERE addressHash = :addressHash ORDER BY walletId",
    )
    suspend fun getAllByAddressHash(addressHash: ByteArray): List<PlatformAddressEntity>

    /**
     * Wallet-scoped BLAST balance lookup — the balance-callback upsert key
     * after the multi-wallet-collision fix (`$0.walletId == walletId &&
     * $0.addressHash == addressHash`). A hash-only predicate can match
     * another wallet's row in a multi-wallet store (same seed imported on
     * coin-type-sharing networks, watch-only duplicates), so the balance
     * persister must narrow by walletId too. Mirror of the Swift
     * `PlatformWalletPersistenceHandler.persistAddressBalances` fix.
     */
    @Query(
        "SELECT * FROM platform_addresses WHERE walletId = :walletId AND addressHash = :addressHash",
    )
    suspend fun getByWalletAndAddressHash(
        walletId: ByteArray,
        addressHash: ByteArray,
    ): PlatformAddressEntity?

    @Upsert
    suspend fun upsert(address: PlatformAddressEntity)

    @Delete
    suspend fun delete(address: PlatformAddressEntity)

    @Query("DELETE FROM platform_addresses WHERE walletId = :walletId")
    suspend fun deleteByWallet(walletId: ByteArray)

    @Query("DELETE FROM platform_addresses")
    suspend fun deleteAll()

    /**
     * Zero the synced (BLAST) balance fields of every address owned by a
     * wallet in [walletIds] **in place**, preserving the durable derivation
     * metadata (`address`, `addressHash`, `publicKey`, `accountIndex`,
     * `addressIndex`, `derivationPath`). Backs the Sync-tab "Clear" action:
     * deleting the rows would empty the UI until an app restart (no in-session
     * sync re-emits the address pool), so we reset the sync-derived state
     * only. Mirror of the Swift `clearLocalState` in-place zero.
     *
     * @param nowMillis epoch-millis stamp for `lastUpdated` (Room stores
     *   `Date` as epoch millis).
     */
    @Query(
        "UPDATE platform_addresses SET balance = 0, nonce = 0, isUsed = 0, " +
            "firstSeenHeight = 0, lastSeenHeight = 0, lastUpdated = :nowMillis " +
            "WHERE walletId IN (:walletIds)",
    )
    suspend fun zeroBalancesForWallets(walletIds: List<ByteArray>, nowMillis: Long)

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

    /**
     * Delete the sync-state watermark row(s) for one network — the Sync-tab
     * "Clear" action deletes the watermark so the next pass is a full rescan
     * (mirror of the Swift `clearLocalState` sync-state delete, scoped by
     * `networkRaw`). Other networks' watermarks are left untouched.
     */
    @Query("DELETE FROM platform_addresses_sync_states WHERE networkRaw = :networkRaw")
    suspend fun deleteSyncStatesByNetwork(networkRaw: Int)

    @Query("DELETE FROM platform_addresses_sync_states")
    suspend fun deleteAllSyncStates()

    /** StorageExplorer row count. */
    @Query("SELECT COUNT(*) FROM platform_addresses_sync_states")
    fun countSyncStates(): Flow<Long>

    /** StorageExplorer network-scoped row count. */
    @Query("SELECT COUNT(*) FROM platform_addresses_sync_states WHERE networkRaw = :networkRaw")
    fun countSyncStatesByNetwork(networkRaw: Int): Flow<Long>
}
