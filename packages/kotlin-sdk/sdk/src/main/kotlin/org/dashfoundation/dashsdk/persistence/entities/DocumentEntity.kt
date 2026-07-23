package org.dashfoundation.dashsdk.persistence.entities

import androidx.room.Entity
import androidx.room.ForeignKey
import androidx.room.Index
import androidx.room.PrimaryKey
import java.util.Date

/**
 * Port of `PersistentDocument.swift` — one platform document.
 *
 * Swift `@Attribute(.unique)` on `documentId` (base58 String) → primary
 * key. Swift `#Index([\.networkRaw])` → index below.
 *
 * Relationship materialization:
 * - `documentType_relation` → [documentTypeRelationId], CASCADE (Swift
 *   `PersistentDocumentType.documents` declares `.cascade`).
 * - `dataContract` → [dataContractId], CASCADE (Swift
 *   `PersistentDataContract.documents` declares `.cascade`).
 * - `ownerIdentity` → [ownerIdentityId], CASCADE (Swift
 *   `PersistentIdentity.documents` declares `.cascade`). Nullable — only
 *   linked when the owner identity is local; the [ownerId]/[ownerIdData]
 *   scalars are always set regardless.
 */
@Entity(
    tableName = "documents",
    indices = [
        Index(value = ["networkRaw"]),
        Index(value = ["contractId"]),
        Index(value = ["ownerId"]),
        Index(value = ["documentTypeRelationId"]),
        Index(value = ["dataContractId"]),
        Index(value = ["ownerIdentityId"]),
    ],
    foreignKeys = [
        ForeignKey(
            entity = DocumentTypeEntity::class,
            parentColumns = ["id"],
            childColumns = ["documentTypeRelationId"],
            onDelete = ForeignKey.CASCADE,
        ),
        ForeignKey(
            entity = DataContractEntity::class,
            parentColumns = ["id"],
            childColumns = ["dataContractId"],
            onDelete = ForeignKey.CASCADE,
        ),
        ForeignKey(
            entity = IdentityEntity::class,
            parentColumns = ["identityId"],
            childColumns = ["ownerIdentityId"],
            onDelete = ForeignKey.CASCADE,
        ),
    ],
)
data class DocumentEntity(
    /** Document id as base58 String (verbatim Swift storage shape). */
    @PrimaryKey val documentId: String,
    val documentType: String,
    /** Swift `Int32`. */
    val revision: Int,
    /** JSON document properties blob — passthrough. */
    val data: ByteArray,
    /** Contract id as base58 String (query column in Swift). */
    val contractId: String,
    /** Owner id as base58 String (query column in Swift). */
    val ownerId: String,
    /** Binary twin of [contractId] (32 bytes). */
    val contractIdData: ByteArray,
    /** Binary twin of [ownerId] (32 bytes). */
    val ownerIdData: ByteArray,
    val createdAt: Date = Date(),
    val updatedAt: Date = Date(),
    val transferredAt: Date? = null,
    val createdAtBlockHeight: Long? = null,
    val updatedAtBlockHeight: Long? = null,
    val transferredAtBlockHeight: Long? = null,
    val createdAtCoreBlockHeight: Long? = null,
    val updatedAtCoreBlockHeight: Long? = null,
    val transferredAtCoreBlockHeight: Long? = null,
    /** `Network.rawValue`; Swift `UInt32` → [Int]. */
    val networkRaw: Int,
    val isDeleted: Boolean = false,
    val localCreatedAt: Date = Date(),
    val localUpdatedAt: Date = Date(),
    /** FK materialization of the Swift `documentType_relation` relationship. */
    val documentTypeRelationId: ByteArray? = null,
    /** FK materialization of the Swift `dataContract` relationship. */
    val dataContractId: ByteArray? = null,
    /** FK materialization of the Swift `ownerIdentity` relationship. */
    val ownerIdentityId: ByteArray? = null,
)
