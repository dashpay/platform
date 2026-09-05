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
     * (AssetLockAddressTopUp) and a recoverable, non-terminal status.
     * Backs the "Pending Platform Top Ups" orphan surface (← the SwiftData
     * `@Query` behind `PendingPlatformFundFromAssetLocksList.swift`, whose
     * Swift filter is `fundingTypeRaw == 4 && isVisibleAsResumable`).
     *
     * The recoverable set is `[1, 3] ∪ {5}` — Broadcast, InstantSendLocked,
     * ChainLocked, and RecoveredFromChain. `0` (Built) is excluded because
     * the funding transaction has not been broadcast; `4` (Consumed) is the
     * terminal tombstone that Rust's `resume_asset_lock` rejects outright.
     *
     * `5` (RecoveredFromChain) is not a gap in the ordering — it is a
     * distinct status written by the restore scan and by the chainlock
     * promotion path for a lock whose Core finality is proven but whose
     * Platform-side consumption is unknown (`sync/reconstruction.rs`). A
     * range bounded at `3` dropped exactly those rows, so a chain-locked
     * address top-up rebuilt from history appeared on no surface at all.
     */
    @Query(
        "SELECT * FROM asset_locks WHERE walletId = :walletId " +
            "AND fundingTypeRaw = 4 " +
            "AND ((statusRaw >= 1 AND statusRaw <= 3) OR statusRaw = 5)"
    )
    fun observeResumableAddressTopUps(walletId: ByteArray): Flow<List<AssetLockEntity>>

    /**
     * Funding-type-scoped variant of [observeResumableAddressTopUps], using
     * the identical recoverable-status predicate.
     *
     * Exists because shielded address top-ups (`fundingTypeRaw == 5`,
     * `AssetLockShieldedAddressTopUp`) had no resumable query at all: the
     * query above is pinned to `4`, and the identity-recovery surface fed by
     * `TrackedAssetLock.eligibleFromNative` deliberately admits only funding
     * types `0..2`. A stalled or chain-locked shielded top-up was therefore
     * invisible on every host surface.
     *
     * Pass `4` (address) or `5` (shielded). Funding types `0..3` are
     * identity-family locks, whose recovery surface is the identity screens.
     */
    @Query(
        "SELECT * FROM asset_locks WHERE walletId = :walletId " +
            "AND fundingTypeRaw = :fundingTypeRaw " +
            "AND ((statusRaw >= 1 AND statusRaw <= 3) OR statusRaw = 5)"
    )
    fun observeResumableTopUpsByFundingType(
        walletId: ByteArray,
        fundingTypeRaw: Int,
    ): Flow<List<AssetLockEntity>>

    @Query("SELECT * FROM asset_locks WHERE outPointHex = :outPointHex")
    suspend fun getByOutPointHex(outPointHex: String): AssetLockEntity?

    /**
     * Strongest lifecycle status any asset lock funded by [txidHex] has
     * reached, or null when the transaction funds no tracked lock.
     *
     * [txidHex] is the explorer DISPLAY txid hex (64 chars, wire order
     * reversed) — the prefix of the `outPointHex` PK
     * (`<txidDisplayHex>:<vout>`). Deliberately keyed on the txid alone
     * and NOT on a whole outpoint: DIP-0027 lets one funding transaction
     * carry several credit outputs, and `sync/reconstruction.rs` persists
     * each of them under its own credit-output index, so the lock a given
     * funding transaction produced can live at any vout. Finality is a
     * property of the transaction, so `MAX` over the whole prefix is the
     * right reduction — any output of it reaching InstantSendLocked means
     * the transaction's inputs are gone.
     *
     * Same 64-hex input contract as [fundingTypeForTxid], enforced in SQL
     * and compared against the exact 65-char `<txid>:` prefix rather than
     * a LIKE pattern, so `%`/`_` in malformed input can never match
     * arbitrary rows.
     */
    @Query(
        "SELECT MAX(statusRaw) FROM asset_locks " +
            "WHERE length(:txidHex) = 64 " +
            "AND lower(:txidHex) NOT GLOB '*[^0-9a-f]*' " +
            "AND substr(outPointHex, 1, 65) = lower(:txidHex) || ':'"
    )
    suspend fun maxStatusForTxid(txidHex: String): Int?

    /**
     * Transaction-label resolver probe: the `fundingTypeRaw` of the asset
     * lock whose outpoint belongs to [txidHex]. [txidHex] is the explorer
     * DISPLAY txid hex (64 chars, wire order reversed; uppercase input is
     * canonicalized via `lower()`) — the prefix of the `outPointHex` PK
     * (`<txidDisplayHex>:<vout>`). The query enforces the 64-hex input
     * contract itself (length + hex-only GLOB, matching the
     * null-on-malformed behavior of
     * [TransactionDao.transactionKindForDisplayTxid]) and compares the
     * exact 65-char `<txid>:` prefix rather than a LIKE pattern, so
     * `%`/`_` in malformed input can never match arbitrary rows. Returns
     * null for malformed input or when no asset lock funds this tx (e.g.
     * plain send, or an AssetUnlock/unshield — see
     * [TransactionDao.transactionKindForTxid]).
     *
     * DIP-0027 permits several asset-lock outputs in one transaction;
     * those rows are created by the same funding flow and share a
     * `fundingTypeRaw`, so the unordered `LIMIT 1` pick is value-stable.
     */
    @Query(
        "SELECT fundingTypeRaw FROM asset_locks " +
            "WHERE length(:txidHex) = 64 " +
            "AND lower(:txidHex) NOT GLOB '*[^0-9a-f]*' " +
            "AND substr(outPointHex, 1, 65) = lower(:txidHex) || ':' " +
            "LIMIT 1"
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
