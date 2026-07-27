package org.dashfoundation.dashsdk.persistence.entities

import androidx.room.Entity
import androidx.room.Index
import androidx.room.PrimaryKey

/**
 * One sent DashPay invitation (DIP-13) — a funded one-time asset-lock
 * voucher — keyed by its 36-byte funding outpoint. Durable mirror of the
 * `InvitationChangeSet` rows forwarded from Rust through
 * `NativePersistenceBridge.onPersistInvitationUpsert`.
 *
 * Durability matters: the Rust `create_invitation` durability gate only lets
 * the voucher be minted because this row is genuinely persisted. A row that
 * were lost across a restart could let the same one-time voucher key be
 * re-exported, so this must never be a no-op store.
 *
 * The PK is the raw 36-byte outpoint (`txid[32] || vout_le[4]`) — the same
 * encoding the FFI keys every invitation upsert/removal by — so there is one
 * outpoint shape across the whole bridge, no display-hex conversion.
 */
@Entity(
    tableName = "invitations",
    indices = [Index(value = ["walletId"])],
)
data class InvitationEntity(
    /** 36-byte funding outpoint: `txid[32] || vout_le[4]`. */
    @PrimaryKey val outPoint: ByteArray,
    /** 32-byte owning wallet id. */
    val walletId: ByteArray,
    /** DIP-13 funding index the voucher key derives from (unsigned, in an Int). */
    val fundingIndex: Int,
    /** Voucher amount in duffs. */
    val amountDuffs: Long,
    /** Advisory expiry, unix seconds. */
    val expiryUnix: Long,
    /** Creation time, unix seconds. */
    val createdAtSecs: Long,
    /** 1 if the link carries inviter/contact-bootstrap info, else 0. */
    val hasInviter: Int,
    /** `InvitationStatus` discriminant: 0 Created, 1 Claimed, 2 Reclaimed. */
    val statusRaw: Int,
) {
    override fun equals(other: Any?): Boolean {
        if (this === other) return true
        if (other !is InvitationEntity) return false
        return outPoint.contentEquals(other.outPoint) &&
            walletId.contentEquals(other.walletId) &&
            fundingIndex == other.fundingIndex &&
            amountDuffs == other.amountDuffs &&
            expiryUnix == other.expiryUnix &&
            createdAtSecs == other.createdAtSecs &&
            hasInviter == other.hasInviter &&
            statusRaw == other.statusRaw
    }

    override fun hashCode(): Int {
        var result = outPoint.contentHashCode()
        result = 31 * result + walletId.contentHashCode()
        result = 31 * result + fundingIndex
        result = 31 * result + amountDuffs.hashCode()
        result = 31 * result + expiryUnix.hashCode()
        result = 31 * result + createdAtSecs.hashCode()
        result = 31 * result + hasInviter
        result = 31 * result + statusRaw
        return result
    }
}
