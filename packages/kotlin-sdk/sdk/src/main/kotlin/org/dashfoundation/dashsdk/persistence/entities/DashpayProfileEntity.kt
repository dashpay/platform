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
    /**
     * Published `corePaymentAddress` in storage form: a 0x00 (P2PKH) or
     * 0x01 (P2SH) type byte, then the 20-byte HASH160. Null when the
     * profile publishes none. Swift keeps the three addresses in the
     * `PersistentDashpayPaymentAddresses` sidecar to leave its released
     * SwiftData schema alone; Room migrates in place, so they ride this row
     * and commit with the rest of the profile.
     */
    val corePaymentAddress: ByteArray? = null,
    /** Published `platformPaymentAddress`, same 21-byte form; null when none. */
    val platformPaymentAddress: ByteArray? = null,
    /** Published `shieldedAddress`, the raw 43-byte Orchard address; null when none. */
    val shieldedAddress: ByteArray? = null,
    val createdAt: Date = Date(),
    val lastUpdated: Date = Date(),
)
