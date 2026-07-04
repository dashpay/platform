package org.dashfoundation.dashsdk.persistence.dao

import androidx.room.Dao
import androidx.room.Delete
import androidx.room.Query
import androidx.room.Upsert
import kotlinx.coroutines.flow.Flow
import org.dashfoundation.dashsdk.persistence.entities.WalletEntity

/**
 * Queries over [WalletEntity], mirroring the `PersistentWallet`
 * predicates (`predicate(walletId:)`, `predicate(walletGroupId:)`) plus
 * the per-network scans backed by the Swift `#Index([\.networkRaw])`.
 */
@Dao
interface WalletDao {

    /** All wallets (storage explorer / wallet list). */
    @Query("SELECT * FROM wallets")
    fun observeAll(): Flow<List<WalletEntity>>

    /** Mirror of `PersistentWallet.predicate(walletId:)`. */
    @Query("SELECT * FROM wallets WHERE walletId = :walletId")
    fun observeByWalletId(walletId: ByteArray): Flow<WalletEntity?>

    @Query("SELECT * FROM wallets WHERE walletId = :walletId")
    suspend fun getByWalletId(walletId: ByteArray): WalletEntity?

    /**
     * Mirror of `PersistentWallet.predicate(walletGroupId:)` — every
     * sibling-network row for one seed.
     */
    @Query("SELECT * FROM wallets WHERE walletGroupId = :walletGroupId")
    fun observeByGroupId(walletGroupId: ByteArray): Flow<List<WalletEntity>>

    /** Per-network wallet scan (`$0.networkRaw == networkRaw`). */
    @Query("SELECT * FROM wallets WHERE networkRaw = :networkRaw")
    fun observeByNetwork(networkRaw: Int): Flow<List<WalletEntity>>

    @Query("SELECT * FROM wallets WHERE networkRaw = :networkRaw")
    suspend fun getByNetwork(networkRaw: Int): List<WalletEntity>

    @Upsert
    suspend fun upsert(wallet: WalletEntity)

    /**
     * Stamp the display name onto an existing row. The Rust persister
     * doesn't know user-facing labels, so the name is a Kotlin-side
     * persist step after `createWallet` returns the scoped walletId —
     * mirror of Swift `CreateWalletView.createWallet(using:)`, which
     * passes `name: walletLabel` into the manager and writes the label
     * onto the persisted wallet row / keychain metadata (and of
     * `WalletInfoView.saveWalletName`, which writes `wallet.name`).
     *
     * @param nowMillis epoch millis for the `lastUpdated` stamp.
     * @return number of rows updated (0 when the row doesn't exist yet).
     */
    @Query(
        "UPDATE wallets SET name = :name, lastUpdated = :nowMillis " +
            "WHERE walletId = :walletId"
    )
    suspend fun updateName(walletId: ByteArray, name: String?, nowMillis: Long): Int

    @Delete
    suspend fun delete(wallet: WalletEntity)

    @Query("DELETE FROM wallets WHERE walletId = :walletId")
    suspend fun deleteByWalletId(walletId: ByteArray)

    @Query("DELETE FROM wallets")
    suspend fun deleteAll()

    /** StorageExplorer row count. */
    @Query("SELECT COUNT(*) FROM wallets")
    fun count(): Flow<Long>

    /** StorageExplorer network-scoped row count. */
    @Query("SELECT COUNT(*) FROM wallets WHERE networkRaw = :networkRaw")
    fun countByNetwork(networkRaw: Int): Flow<Long>
}
