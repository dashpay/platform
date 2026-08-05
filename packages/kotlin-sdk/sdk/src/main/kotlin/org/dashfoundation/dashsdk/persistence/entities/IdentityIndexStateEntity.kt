package org.dashfoundation.dashsdk.persistence.entities

import androidx.room.Entity
import androidx.room.PrimaryKey

/**
 * Durable per-wallet high-water mark for issued DIP-9 identity indices.
 *
 * The mark advances before key preview/provisioning and never moves backward.
 * It is intentionally independent of identity persistence: native registration
 * treats local bookkeeping as best-effort, so a missing [IdentityEntity] row
 * must not make an already-derived key path reusable after process death.
 */
@Entity(tableName = "identity_index_state")
data class IdentityIndexStateEntity(
    @PrimaryKey val walletId: ByteArray,
    val lastIssuedIndex: Int,
) {
    init {
        require(walletId.size == 32) { "walletId must be 32 bytes" }
        require(lastIssuedIndex >= 0) { "lastIssuedIndex must be non-negative" }
    }

    override fun equals(other: Any?): Boolean =
        other is IdentityIndexStateEntity &&
            walletId.contentEquals(other.walletId) &&
            lastIssuedIndex == other.lastIssuedIndex

    override fun hashCode(): Int = 31 * walletId.contentHashCode() + lastIssuedIndex
}
