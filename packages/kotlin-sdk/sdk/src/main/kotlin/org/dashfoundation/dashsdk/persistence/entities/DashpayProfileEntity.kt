package org.dashfoundation.dashsdk.persistence.entities

import androidx.room.Entity
import androidx.room.ForeignKey
import androidx.room.Index
import java.util.Date

/**
 * Port of `PersistentDashpayProfile.swift` — DashPay `profile` document
 * cache, at most one row per (network, identity).
 *
 * Swift `#Unique([\.networkRaw, \.identity])` → composite primary key on
 * `(networkRaw, identityId)` (the relationship's key is the identity id).
 *
 * [identityId] materializes the NON-optional `identity` relationship with
 * CASCADE (Swift `PersistentIdentity.dashpayProfile` declares `.cascade`).
 */
@Entity(
    tableName = "dashpay_profiles",
    primaryKeys = ["networkRaw", "identityId"],
    indices = [Index(value = ["identityId"])],
    foreignKeys = [
        ForeignKey(
            entity = IdentityEntity::class,
            parentColumns = ["identityId"],
            childColumns = ["identityId"],
            onDelete = ForeignKey.CASCADE,
        ),
    ],
)
data class DashpayProfileEntity(
    /** `Network.rawValue`; Swift `UInt32` → [Int]. */
    val networkRaw: Int,
    /** Owning identity (32 bytes) — non-optional in Swift. */
    val identityId: ByteArray,
    /** `displayName` document field, ≤25 chars per contract schema. */
    val displayName: String? = null,
    /** `publicMessage` document field, ≤140 chars per contract schema. */
    val publicMessage: String? = null,
    /** Reserved forwards-compat slot (not in the v3 contract). */
    val bio: String? = null,
    val avatarUrl: String? = null,
    /** 32-byte hash of the avatar binary. */
    val avatarHash: ByteArray? = null,
    /** 8-byte perceptual hash. */
    val avatarFingerprint: ByteArray? = null,
    val createdAt: Date = Date(),
    val lastUpdated: Date = Date(),
)
