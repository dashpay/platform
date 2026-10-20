package org.dashfoundation.dashsdk.persistence.entities

import androidx.room.Entity
import androidx.room.ForeignKey
import androidx.room.Index
import androidx.room.PrimaryKey

/**
 * Port of `PersistentKeyword.swift` — one search keyword on a contract.
 *
 * Swift `@Attribute(.unique)` on `id` (`"<contractId>_<keyword>"`) →
 * primary key.
 *
 * [dataContractId] materializes the optional `dataContract` relationship
 * with CASCADE (Swift `PersistentDataContract.keywordRelations` declares
 * `.cascade`). The [contractId] scalar stays a base58 String — that's what
 * Swift stores (note the asymmetry with the contract's binary `id`).
 */
@Entity(
    tableName = "keywords",
    indices = [
        Index(value = ["contractId"]),
        Index(value = ["dataContractId"]),
    ],
    foreignKeys = [
        ForeignKey(
            entity = DataContractEntity::class,
            parentColumns = ["id"],
            childColumns = ["dataContractId"],
            onDelete = ForeignKey.CASCADE,
        ),
    ],
)
data class KeywordEntity(
    /** `"<contractId base58>_<keyword>"`. */
    @PrimaryKey val id: String,
    val keyword: String,
    /** Contract id as base58 String (verbatim Swift storage shape). */
    val contractId: String,
    /** FK materialization of the Swift `dataContract` relationship (32 bytes). */
    val dataContractId: ByteArray? = null,
)
