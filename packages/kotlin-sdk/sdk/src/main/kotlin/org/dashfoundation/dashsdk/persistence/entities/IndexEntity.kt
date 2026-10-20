package org.dashfoundation.dashsdk.persistence.entities

import androidx.room.Entity
import androidx.room.ForeignKey
import androidx.room.Index
import androidx.room.PrimaryKey
import java.util.Date

/**
 * Port of `PersistentIndex.swift` — one document-type index definition.
 *
 * Swift `@Attribute(.unique)` on `id`
 * (`contractId || utf8(documentTypeName) || utf8(name)`) → primary key.
 *
 * [documentTypeId] materializes the optional `documentType` relationship
 * with CASCADE (Swift `PersistentDocumentType.indices` declares
 * `.cascade`).
 */
@Entity(
    tableName = "indices",
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
data class IndexEntity(
    /** Swift synthetic unique id (contractId + docType name + index name). */
    @PrimaryKey val id: ByteArray,
    /** 32-byte contract id. */
    val contractId: ByteArray,
    val documentTypeName: String,
    val name: String,
    /** Column name is an SQL keyword; Room quotes it in generated SQL. */
    val unique: Boolean = false,
    val nullSearchable: Boolean = false,
    val contested: Boolean = false,
    /** JSON array of indexed properties with sort order — passthrough. */
    val propertiesJSON: ByteArray,
    /** JSON contested-details blob — passthrough. */
    val contestedDetailsJSON: ByteArray? = null,
    val createdAt: Date = Date(),
    /** FK materialization of the Swift `documentType` relationship. */
    val documentTypeId: ByteArray? = null,
) {
    override fun equals(other: Any?): Boolean =
        other is IndexEntity && id.contentEquals(other.id)

    override fun hashCode(): Int = id.contentHashCode()
}
