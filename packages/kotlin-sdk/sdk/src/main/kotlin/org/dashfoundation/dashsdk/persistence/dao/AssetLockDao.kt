package org.dashfoundation.dashsdk.persistence.dao

import androidx.room.Dao
import androidx.room.Delete
import androidx.room.Query
import androidx.room.Upsert
import kotlinx.coroutines.flow.Flow
import org.dashfoundation.dashsdk.persistence.entities.AssetLockEntity

/**
 * Queries over [AssetLockEntity], mirroring the Swift call sites:
 * `PersistentAssetLock.predicate(walletId:)`,
 * `predicate(walletId:identityIndex:)` (RegistrationProgressView), the
 * funding-type-scoped progress views (`fundingTypeRaw == 4 / 5`), the
 * consumed-lock picker (`fundingTypeRaw == 4 && statusRaw == 4`), the
 * resumable-locks load path (`statusRaw < 2`), and the handler's upsert /
 * delete by `outPointHex`.
 */
@Dao
interface AssetLockDao {

    @Query("SELECT * FROM asset_locks WHERE walletId = :walletId")
    fun observeByWallet(walletId: ByteArray): Flow<List<AssetLockEntity>>

    /** RegistrationProgressView's per-slot query. */
    @Query(
        "SELECT * FROM asset_locks WHERE walletId = :walletId " +
            "AND identityIndexRaw = :identityIndexRaw"
    )
    fun observeByWalletAndIdentityIndex(
        walletId: ByteArray,
        identityIndexRaw: Int,
    ): Flow<List<AssetLockEntity>>

    /** Address/shielded top-up progress views (fundingTypeRaw 4 / 5). */
    @Query(
        "SELECT * FROM asset_locks WHERE walletId = :walletId " +
            "AND fundingTypeRaw = :fundingTypeRaw"
    )
    fun observeByWalletAndFundingType(
        walletId: ByteArray,
        fundingTypeRaw: Int,
    ): Flow<List<AssetLockEntity>>

    /** Consumed address-funding locks picker (`fundingTypeRaw == 4 && statusRaw == 4`). */
    @Query(
        "SELECT * FROM asset_locks WHERE walletId = :walletId " +
            "AND fundingTypeRaw = :fundingTypeRaw AND statusRaw = :statusRaw"
    )
    suspend fun getByWalletFundingTypeAndStatus(
        walletId: ByteArray,
        fundingTypeRaw: Int,
        statusRaw: Int,
    ): List<AssetLockEntity>

    /** Load-time rehydration: not-yet-InstantSendLocked rows (`statusRaw < 2`). */
    @Query("SELECT * FROM asset_locks WHERE walletId = :walletId AND statusRaw < 2")
    suspend fun getUnresolvedByWallet(walletId: ByteArray): List<AssetLockEntity>

    /**
     * Resumable Platform-address top-up locks — `fundingTypeRaw == 4`
     * (AssetLockAddressTopUp) and `statusRaw ∈ [1, 3]` (Broadcast through
     * ChainLocked, excluding Built and Consumed). Backs the "Pending
     * Platform Top Ups" orphan surface (← the SwiftData `@Query` behind
     * `PendingPlatformFundFromAssetLocksList.swift`, whose Swift filter is
     * `fundingTypeRaw == 4 && isVisibleAsResumable`).
     */
    @Query(
        "SELECT * FROM asset_locks WHERE walletId = :walletId " +
            "AND fundingTypeRaw = 4 AND statusRaw >= 1 AND statusRaw <= 3"
    )
    fun observeResumableAddressTopUps(walletId: ByteArray): Flow<List<AssetLockEntity>>

    @Query("SELECT * FROM asset_locks WHERE outPointHex = :outPointHex")
    suspend fun getByOutPointHex(outPointHex: String): AssetLockEntity?

    /**
     * Transaction-label resolver probe: the `fundingTypeRaw` of the asset
     * lock whose outpoint belongs to [txidHex]. [txidHex] is the explorer
     * DISPLAY txid hex (64 lowercase chars, wire order reversed) — the
     * prefix of the `outPointHex` PK (`<txidDisplayHex>:<vout>`), matched
     * via `LIKE '<txidHex>:%'`. A txid is a pure-hex string (no `%`/`_`),
     * so the LIKE pattern carries no wildcards of its own. Returns null
     * when no asset lock funds this tx (e.g. plain send, or an
     * AssetUnlock/unshield — see [TransactionDao.transactionKindForTxid]).
     */
    @Query(
        "SELECT fundingTypeRaw FROM asset_locks " +
            "WHERE outPointHex LIKE :txidHex || ':%' LIMIT 1"
    )
    suspend fun fundingTypeForTxid(txidHex: String): Int?

    @Upsert
    suspend fun upsert(assetLock: AssetLockEntity)

    @Delete
    suspend fun delete(assetLock: AssetLockEntity)

    /** Consumed-lock removal path (`$0.outPointHex == hex`). */
    @Query("DELETE FROM asset_locks WHERE outPointHex = :outPointHex")
    suspend fun deleteByOutPointHex(outPointHex: String)

    /** Wallet teardown mirror of `deleteWalletData`'s asset-lock pass. */
    @Query("DELETE FROM asset_locks WHERE walletId = :walletId")
    suspend fun deleteByWallet(walletId: ByteArray)

    @Query("DELETE FROM asset_locks")
    suspend fun deleteAll()

    /** StorageExplorer row count. */
    @Query("SELECT COUNT(*) FROM asset_locks")
    fun count(): Flow<Long>

    @Query("SELECT COUNT(*) FROM asset_locks WHERE walletId = :walletId")
    fun countByWallet(walletId: ByteArray): Flow<Long>
}
