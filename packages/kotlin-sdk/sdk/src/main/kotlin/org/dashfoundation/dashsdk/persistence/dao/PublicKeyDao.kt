package org.dashfoundation.dashsdk.persistence.dao

import androidx.room.Dao
import androidx.room.Delete
import androidx.room.Insert
import androidx.room.Query
import androidx.room.Update
import kotlinx.coroutines.flow.Flow
import org.dashfoundation.dashsdk.persistence.entities.PublicKeyEntity

/**
 * Queries over [PublicKeyEntity], mirroring the Swift call sites:
 * `KeychainSigner`'s lookup by raw key bytes
 * (`row.publicKeyData == publicKey`) and the persister's upsert / delete
 * by `(keyId, identityId-base58)`.
 */
@Dao
interface PublicKeyDao {

    /** All keys of one identity (base58 id — the Swift storage shape). */
    @Query("SELECT * FROM public_keys WHERE identityId = :identityId ORDER BY keyId")
    fun observeByIdentityId(identityId: String): Flow<List<PublicKeyEntity>>

    /** Persister upsert / removal key. */
    @Query("SELECT * FROM public_keys WHERE identityId = :identityId AND keyId = :keyId")
    suspend fun getByIdentityAndKeyId(identityId: String, keyId: Int): PublicKeyEntity?

    /** KeychainSigner: resolve a row from raw public key bytes. */
    @Query("SELECT * FROM public_keys WHERE publicKeyData = :publicKeyData")
    suspend fun getByPublicKeyData(publicKeyData: ByteArray): List<PublicKeyEntity>

    @Insert
    suspend fun insert(publicKey: PublicKeyEntity): Long

    @Update
    suspend fun update(publicKey: PublicKeyEntity)

    @Delete
    suspend fun delete(publicKey: PublicKeyEntity)

    /** Mirror of the persister's removed-key pass. */
    @Query("DELETE FROM public_keys WHERE identityId = :identityId AND keyId = :keyId")
    suspend fun deleteByIdentityAndKeyId(identityId: String, keyId: Int)

    @Query("DELETE FROM public_keys")
    suspend fun deleteAll()

    /** StorageExplorer row count. */
    @Query("SELECT COUNT(*) FROM public_keys")
    fun count(): Flow<Long>
}
