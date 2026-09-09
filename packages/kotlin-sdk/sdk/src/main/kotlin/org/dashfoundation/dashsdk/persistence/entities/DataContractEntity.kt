package org.dashfoundation.dashsdk.persistence.entities

import androidx.room.Entity
import androidx.room.ForeignKey
import androidx.room.Index
import androidx.room.PrimaryKey
import java.util.Date

/**
 * Port of `PersistentDataContract.swift` — one cached data contract.
 *
 * Swift `@Attribute(.unique)` on `id` → primary key (32-byte contract id).
 * Swift `#Index([\.networkRaw])` → index below.
 *
 * [ownerIdentityId] materializes the optional `ownerIdentity` relationship
 * (back-filled by `ContractIdentityLinker` when the owner is local) with
 * SET_NULL (Swift declares `.nullify` — deleting the identity leaves
 * contract rows alive). Distinct from the [ownerId] scalar, which is the
 * on-chain owner and set even when that identity isn't stored locally.
 *
 * Children reference this row with CASCADE FKs on their side: `keywords`,
 * `tokens`, `document_types`, `documents` — matching the Swift
 * `.cascade` relationships.
 */
@Entity(
    tableName = "data_contracts",
    indices = [
        Index(value = ["networkRaw"]),
        Index(value = ["ownerIdentityId"]),
    ],
    foreignKeys = [
        ForeignKey(
            entity = IdentityEntity::class,
            parentColumns = ["identityId"],
            childColumns = ["ownerIdentityId"],
            onDelete = ForeignKey.SET_NULL,
        ),
    ],
)
data class DataContractEntity(
    /** 32-byte contract id. */
    @PrimaryKey val id: ByteArray,
    val name: String,
    /** JSON-serialized contract (Swift parses it with JSONSerialization). */
    val serializedContract: ByteArray,
    val createdAt: Date = Date(),
    val lastAccessedAt: Date = Date(),
    /** Binary (CBOR) serialization when available — opaque passthrough. */
    val binarySerialization: ByteArray? = null,
    val version: Int? = 1,
    /** On-chain owner id (32 bytes); may reference a non-local identity. */
    val ownerId: ByteArray? = null,
    val contractDescription: String? = null,
    /** JSON `[String: Any]` blob — passthrough, same shape Swift writes. */
    val schemaData: ByteArray = ByteArray(0),
    /** JSON `[String]` blob of document type names — passthrough. */
    val documentTypesData: ByteArray = ByteArray(0),
    /** JSON `[String: Any]` blob of group definitions — passthrough. */
    val groupsData: ByteArray? = null,
    /** `Network.rawValue`; Swift `UInt32` → [Int]. */
    val networkRaw: Int,
    val lastUpdated: Date = Date(),
    val lastSyncedAt: Date? = null,
    val canBeDeleted: Boolean = false,
    val readonly: Boolean = false,
    val keepsHistory: Boolean = false,
    val schemaDefs: Int? = null,
    val documentsKeepHistoryContractDefault: Boolean = false,
    val documentsMutableContractDefault: Boolean = true,
    val documentsCanBeDeletedContractDefault: Boolean = true,
    val hasTokens: Boolean = false,
    /** JSON `[String: Any]` blob of token configurations — passthrough. */
    val tokensData: ByteArray? = null,
    /** FK materialization of the optional `ownerIdentity` relationship. */
    val ownerIdentityId: ByteArray? = null,
) {
    override fun equals(other: Any?): Boolean =
        other is DataContractEntity && id.contentEquals(other.id)

    override fun hashCode(): Int = id.contentHashCode()
}
