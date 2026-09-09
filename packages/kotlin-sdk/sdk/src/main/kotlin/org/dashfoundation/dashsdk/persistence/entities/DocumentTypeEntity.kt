package org.dashfoundation.dashsdk.persistence.entities

import androidx.room.Entity
import androidx.room.ForeignKey
import androidx.room.Index
import androidx.room.PrimaryKey
import java.util.Date

/**
 * Port of `PersistentDocumentType.swift` — one document type definition.
 *
 * Swift `@Attribute(.unique)` on `id` (`contractId || utf8(name)`) →
 * primary key.
 *
 * The [contractId] scalar always equals the parent contract's id and rows
 * are only created after the contract row exists (DataContractParser), so
 * the FK is declared directly on it with CASCADE (Swift
 * `PersistentDataContract.documentTypes` declares `.cascade`).
 */
@Entity(
    tableName = "document_types",
    indices = [Index(value = ["contractId"])],
    foreignKeys = [
        ForeignKey(
            entity = DataContractEntity::class,
            parentColumns = ["id"],
            childColumns = ["contractId"],
            onDelete = ForeignKey.CASCADE,
        ),
    ],
)
data class DocumentTypeEntity(
    /** `contractId (32B) || utf8(name)` — the Swift synthetic unique id. */
    @PrimaryKey val id: ByteArray,
    /** 32-byte parent contract id (also the FK column). */
    val contractId: ByteArray,
    val name: String,
    /** JSON schema blob — passthrough. */
    val schemaJSON: ByteArray,
    /** JSON properties blob — passthrough. */
    val propertiesJSON: ByteArray,
    val documentsKeepHistory: Boolean = false,
    val documentsMutable: Boolean = true,
    val documentsCanBeDeleted: Boolean = true,
    val documentsTransferable: Boolean = false,
    /** JSON `[String]` blob — passthrough. */
    val requiredFieldsJSON: ByteArray? = null,
    val securityLevel: Int = 0,
    val tradeMode: Int = 0,
    val creationRestrictionMode: Int = 0,
    val requiresIdentityEncryptionBoundedKey: Boolean = false,
    val requiresIdentityDecryptionBoundedKey: Boolean = false,
    val createdAt: Date = Date(),
    val lastAccessedAt: Date = Date(),
) {
    override fun equals(other: Any?): Boolean =
        other is DocumentTypeEntity && id.contentEquals(other.id)

    override fun hashCode(): Int = id.contentHashCode()
}
