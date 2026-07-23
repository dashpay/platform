package org.dashfoundation.dashsdk.persistence.entities

import androidx.room.Entity
import androidx.room.ForeignKey
import androidx.room.Index
import androidx.room.PrimaryKey
import java.util.Date

/**
 * Port of `PersistentPublicKey.swift` — one identity public key.
 *
 * The Swift model declares no unique attribute; the persister upserts by
 * `(keyId, identityId)` (PlatformWalletPersistenceHandler.swift:1419) —
 * mirrored here as a surrogate rowid PK plus a `(identityId, keyId)` index
 * (non-unique, matching the Swift schema exactly).
 *
 * [identityIdData] materializes the optional `identity` relationship with
 * CASCADE (Swift `PersistentIdentity.publicKeys` declares `.cascade`).
 * The [identityId] scalar stays a base58 String — that's what Swift
 * stores and matches against (verbatim greppability).
 */
@Entity(
    tableName = "public_keys",
    indices = [
        Index(value = ["identityId", "keyId"]),
        Index(value = ["identityIdData"]),
        Index(value = ["publicKeyData"]),
    ],
    foreignKeys = [
        ForeignKey(
            entity = IdentityEntity::class,
            parentColumns = ["identityId"],
            childColumns = ["identityIdData"],
            onDelete = ForeignKey.CASCADE,
        ),
    ],
)
data class PublicKeyEntity(
    /** Surrogate rowid — Swift has no unique attribute on this model. */
    @PrimaryKey(autoGenerate = true) val id: Long = 0,
    /** Swift `Int32`. */
    val keyId: Int,
    /** `KeyPurpose.rawValue` stringified (Swift stores `String(purpose.rawValue)`). */
    val purpose: String,
    /** `SecurityLevel.rawValue` stringified. */
    val securityLevel: String,
    /** `KeyType.rawValue` stringified. */
    val keyType: String,
    val readOnly: Boolean = false,
    val disabledAt: Long? = null,
    val publicKeyData: ByteArray,
    /**
     * JSON-encoded `[base64(contractId)]` blob — legacy shape kept verbatim
     * (Swift PersistentPublicKey.swift:27). Passthrough, never re-encoded.
     */
    val contractBoundsData: ByteArray? = null,
    /** Document-type qualifier for `.singleContractDocumentType` bounds. */
    val contractBoundsDocumentTypeName: String? = null,
    val privateKeyKeychainIdentifier: String? = null,
    /** Owning identity id as base58 String (Swift stores String here). */
    val identityId: String,
    val createdAt: Date = Date(),
    val lastAccessed: Date? = null,
    /** FK materialization of the Swift `identity` relationship (32 bytes). */
    val identityIdData: ByteArray? = null,
)
