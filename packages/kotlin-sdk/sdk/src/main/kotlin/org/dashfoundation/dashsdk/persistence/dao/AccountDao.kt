package org.dashfoundation.dashsdk.persistence.dao

import androidx.room.Dao
import androidx.room.Delete
import androidx.room.Insert
import androidx.room.Query
import androidx.room.Update
import kotlinx.coroutines.flow.Flow
import org.dashfoundation.dashsdk.persistence.entities.AccountEntity

/**
 * Queries over [AccountEntity], mirroring the account lookups in
 * `PlatformWalletPersistenceHandler.swift` (fetch by `(walletId,
 * accountType, accountIndex)` then verify richer fields in code) and the
 * example app's per-wallet account lists.
 *
 * NOTE: no `@Upsert` here on purpose — the surrogate PK plus the unique
 * identity-tuple index means a REPLACE-style conflict resolution would
 * delete-and-reinsert the row, cascading away its `core_addresses` /
 * `platform_addresses` children. Callers must fetch-then-[update] like the
 * Swift persister does.
 */
@Dao
interface AccountDao {

    /** Per-wallet accounts (`acc.wallet.walletId == walletId`). */
    @Query("SELECT * FROM accounts WHERE walletId = :walletId")
    fun observeByWallet(walletId: ByteArray): Flow<List<AccountEntity>>

    /** Wallet + type scan (ReceiveAddressView). */
    @Query("SELECT * FROM accounts WHERE walletId = :walletId AND accountType = :accountType")
    fun observeByWalletAndType(walletId: ByteArray, accountType: Int): Flow<List<AccountEntity>>

    /**
     * Persister match key — candidates for `(walletId, accountType,
     * accountIndex)`; the caller verifies standardTag / registrationIndex /
     * keyClass / identity ids in code, same as Swift.
     */
    @Query(
        "SELECT * FROM accounts WHERE walletId = :walletId " +
            "AND accountType = :accountType AND accountIndex = :accountIndex"
    )
    suspend fun getByKey(walletId: ByteArray, accountType: Int, accountIndex: Int): List<AccountEntity>

    @Query("SELECT * FROM accounts WHERE id = :id")
    suspend fun getById(id: Long): AccountEntity?

    /** Watch-only restore path lookup by the unique xpub. */
    @Query("SELECT * FROM accounts WHERE accountExtendedPubKeyBytes = :xpub")
    suspend fun getByExtendedPubKey(xpub: ByteArray): AccountEntity?

    @Insert
    suspend fun insert(account: AccountEntity): Long

    @Update
    suspend fun update(account: AccountEntity)

    @Delete
    suspend fun delete(account: AccountEntity)

    /** Wallet teardown mirror of `deleteWalletData`'s account pass. */
    @Query("DELETE FROM accounts WHERE walletId = :walletId")
    suspend fun deleteByWallet(walletId: ByteArray)

    @Query("DELETE FROM accounts")
    suspend fun deleteAll()

    /** StorageExplorer row count. */
    @Query("SELECT COUNT(*) FROM accounts")
    fun count(): Flow<Long>

    @Query("SELECT COUNT(*) FROM accounts WHERE walletId = :walletId")
    fun countByWallet(walletId: ByteArray): Flow<Long>
}
