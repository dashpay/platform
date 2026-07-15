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

    /**
     * One-shot list of an identity's keys (base58 id). Used by the
     * wallet-deletion keystore sweep to enumerate the `publicKeyData`
     * hexes whose Keystore private halves must be purged BEFORE the
     * `public_keys` rows are cascade-deleted.
     */
    @Query("SELECT * FROM public_keys WHERE identityId = :identityId ORDER BY keyId")
    suspend fun getByIdentityId(identityId: String): List<PublicKeyEntity>

    /** Persister upsert / removal key. */
    @Query("SELECT * FROM public_keys WHERE identityId = :identityId AND keyId = :keyId")
    suspend fun getByIdentityAndKeyId(identityId: String, keyId: Int): PublicKeyEntity?

    /** KeychainSigner: resolve a row from raw public key bytes. */
    @Query("SELECT * FROM public_keys WHERE publicKeyData = :publicKeyData")
    suspend fun getByPublicKeyData(publicKeyData: ByteArray): List<PublicKeyEntity>

    /**
     * How many `public_keys` rows carry [publicKeyData] but belong to an
     * identity OUTSIDE [ownedIdentityIds] (base58, the removed wallet's
     * identities). Keystore private keys are aliased globally by pubkey
     * hex, so the wallet-deletion sweep must retain any alias still
     * referenced by another wallet's identity — deleting it would break
     * signing for the surviving wallet. Identities not attributable to
     * the removed wallet (including standalone loaded identities) count
     * as external references, conservatively keeping the secret.
     */
    @Query(
        "SELECT COUNT(*) FROM public_keys WHERE publicKeyData = :publicKeyData " +
            "AND identityId NOT IN (:ownedIdentityIds)",
    )
    suspend fun countReferencesOutsideIdentities(
        publicKeyData: ByteArray,
        ownedIdentityIds: List<String>,
    ): Int

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
