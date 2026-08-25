package org.dashfoundation.dashsdk.persistence.entities

import androidx.room.Entity
import androidx.room.ForeignKey
import androidx.room.Index
import java.util.Date

/**
 * Port of `PersistentDPNSName.swift` — one confirmed DPNS label owned by
 * an identity (pure label cache).
 *
 * Swift `#Unique([\.networkRaw, \.normalizedParentDomainName,
 * \.normalizedLabel])` → composite primary key (mirrors the DPNS
 * `parentNameAndLabel` unique index scoped by network).
 *
 * [identityId] materializes the NON-optional `identity` relationship with
 * CASCADE (Swift `PersistentIdentity.dpnsNames` declares `.cascade`).
 */
@Entity(
    tableName = "dpns_names",
    primaryKeys = ["networkRaw", "normalizedParentDomainName", "normalizedLabel"],
    indices = [Index(value = ["identityId"]), Index(value = ["documentId"])],
    foreignKeys = [
        ForeignKey(
            entity = IdentityEntity::class,
            parentColumns = ["identityId"],
            childColumns = ["identityId"],
            onDelete = ForeignKey.CASCADE,
        ),
    ],
)
data class DpnsNameEntity(
    /** `Network.rawValue`; Swift `UInt32` → [Int]. */
    val networkRaw: Int,
    /** Display label as registered, e.g. "Alice". */
    val label: String,
    /** Homograph-safe lowercase form (o/O→0, i/I/l/L→1, else lowercased). */
    val normalizedLabel: String,
    /** Display parent domain — "dash" today. */
    val parentDomainName: String = "dash",
    val normalizedParentDomainName: String = "dash",
    /** Unix millis first observed; Swift `UInt64` → [Long]. 0 = unknown. */
    val acquiredAt: Long = 0,
    /** Owning identity (32 bytes) — non-optional in Swift. */
    val identityId: ByteArray,
    /** Stable DPNS domain-document id; null for legacy label-only rows. */
    val documentId: ByteArray? = null,
    /** Ownership relative to [identityId]. False retains sale/transfer history. */
    val isOwned: Boolean = true,
    /** Listed price in Platform credits, or null when not for sale. */
    val priceCredits: Long? = null,
    /** 0 owned, 1 sold, 2 transferred. */
    val saleStatusRaw: Int = 0,
    /** Buyer/recipient for departed names. */
    val counterpartyIdentityId: ByteArray? = null,
    val documentCreatedAtMs: Long = 0,
    val documentUpdatedAtMs: Long = 0,
    val documentTransferredAtMs: Long = 0,
    /** Wall-clock time of the marketplace reconciliation that wrote this row. */
    val marketplaceUpdatedAt: Long = 0,
    val createdAt: Date = Date(),
    val lastUpdated: Date = Date(),
)
