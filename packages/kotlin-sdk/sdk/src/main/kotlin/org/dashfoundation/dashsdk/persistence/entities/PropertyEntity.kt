package org.dashfoundation.dashsdk.persistence.entities

import androidx.room.Entity
import androidx.room.ForeignKey
import androidx.room.Index
import androidx.room.PrimaryKey
import java.util.Date

/**
 * Port of `PersistentProperty.swift` — one document-type property
 * definition.
 *
 * Swift `@Attribute(.unique)` on `id`
 * (`contractId || utf8(documentTypeName) || utf8(name)`) → primary key.
 *
 * [documentTypeId] materializes the optional `documentType` relationship
 * with CASCADE (Swift `PersistentDocumentType.propertiesList` declares
 * `.cascade`).
 */
@Entity(
    tableName = "properties",
    indices = [Index(value = ["documentTypeId"])],
    foreignKeys = [
        ForeignKey(
            entity = DocumentTypeEntity::class,
            parentColumns = ["id"],
            childColumns = ["documentTypeId"],
            onDelete = ForeignKey.CASCADE,
        ),
    ],
)
data class PropertyEntity(
    /** Swift synthetic unique id (contractId + docType name + prop name). */
    @PrimaryKey val id: ByteArray,
    /** 32-byte contract id. */
    val contractId: ByteArray,
    val documentTypeName: String,
    val name: String,
    val type: String,
    val format: String? = null,
    val contentMediaType: String? = null,
    val byteArray: Boolean = false,
    val minItems: Int? = null,
    val maxItems: Int? = null,
    val pattern: String? = null,
    val minLength: Int? = null,
    val maxLength: Int? = null,
    val minValue: Int? = null,
    val maxValue: Int? = null,
    val fieldDescription: String? = null,
    /** Column name is an SQL keyword; Room quotes it in generated SQL. */
    val transient: Boolean = false,
    val isRequired: Boolean = false,
    val createdAt: Date = Date(),
    /** FK materialization of the Swift `documentType` relationship. */
    val documentTypeId: ByteArray? = null,
) {
    override fun equals(other: Any?): Boolean =
        other is PropertyEntity && id.contentEquals(other.id)

    override fun hashCode(): Int = id.contentHashCode()
}
