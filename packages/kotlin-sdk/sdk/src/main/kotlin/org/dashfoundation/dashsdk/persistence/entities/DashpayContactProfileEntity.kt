package org.dashfoundation.dashsdk.persistence.entities

import androidx.room.Entity
import androidx.room.ForeignKey
import androidx.room.Index
import java.util.Date

/**
 * Port of `PersistentDashpayContactProfile.swift` — one cached DashPay
 * **contact** profile, a mirror of one entry in the Rust-side
 * `contact_profiles` map on a `ManagedIdentity`, one row per
 * `(network, owner, contact)` triple.
 *
 * Distinct from [DashpayProfileEntity], which is the owner's *own*
 * profile: this row is a *contact's* public profile, cached so the
 * requests / contacts UI can show a display name + avatar without
 * re-fetching on every launch. It holds **only the public profile
 * fields** parsed from the on-chain `profile` document; it must never
 * receive anything derived from the encrypted `contactInfo` path.
 *
 * Populated by the platform-wallet persister callback whenever an
 * `IdentityEntry.contact_profiles` entry rides on the FFI changeset. A
 * present profile (`is_present == true`) upserts this row; a
 * confirmed-absent entry (`is_present == false`) DELETEs it, so a
 * contact who removed their on-chain profile can't leave a stale
 * name/avatar behind. Read back at load to rebuild the Rust
 * `contact_profiles` map so the cache survives relaunch.
 *
 * [ownerIdentityId] materializes the NON-optional `owner` relationship
 * with CASCADE (Swift `PersistentIdentity.contactProfiles` declares
 * `.cascade`).
 */
@Entity(
    tableName = "dashpay_contact_profiles",
    primaryKeys = ["networkRaw", "ownerIdentityId", "contactIdentityId"],
    indices = [Index(value = ["ownerIdentityId"])],
    foreignKeys = [
        ForeignKey(
            entity = IdentityEntity::class,
            parentColumns = ["identityId"],
            childColumns = ["ownerIdentityId"],
            onDelete = ForeignKey.CASCADE,
        ),
    ],
)
data class DashpayContactProfileEntity(
    /** `Network.rawValue`; Swift `UInt32` → [Int]. */
    val networkRaw: Int,
    /** Owning (wallet-managed) identity's 32-byte id. */
    val ownerIdentityId: ByteArray,
    /** The contact's 32-byte identity id — the `contact_profiles` map key. */
    val contactIdentityId: ByteArray,
    /** `displayName` field on the contact's `profile` document. */
    val displayName: String? = null,
    /** `publicMessage` field on the contact's `profile` document. */
    val publicMessage: String? = null,
    /** Reserved forwards-compat slot (not in the v3 contract). */
    val bio: String? = null,
    /**
     * `avatarUrl` field — URL the consumer fetches + caches locally; the
     * binary asset itself is never persisted. Untrusted public data: the
     * Rust side caches and restores it only when it is a bounded
     * `https://` URL.
     */
    val avatarUrl: String? = null,
    /** 32-byte hash of the avatar binary. */
    val avatarHash: ByteArray? = null,
    /** 8-byte perceptual hash. */
    val avatarFingerprint: ByteArray? = null,
    /**
     * Wall-clock ms of the last fetch attempt on the Rust side
     * (`ContactProfileEntry.checked_at_ms`) — drives the self-heal
     * backoff. Round-tripped verbatim so the restored cache keeps the
     * same re-query schedule it had before relaunch. Swift `UInt64` →
     * [Long].
     */
    val checkedAtMs: Long,
    val createdAt: Date = Date(),
    val lastUpdated: Date = Date(),
)
