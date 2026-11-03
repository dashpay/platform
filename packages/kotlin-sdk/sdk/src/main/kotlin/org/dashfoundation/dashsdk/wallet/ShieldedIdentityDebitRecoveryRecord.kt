package org.dashfoundation.dashsdk.wallet

import org.dashfoundation.dashsdk.ffi.ShieldedIdentityDebitRecoveryData

/** Recovery state, separately from the activity's execution outcome. */
enum class ShieldedIdentityDebitRecoveryStatus {
    RETRYING,
    PARKED,
    /** Automatic retry was explicitly abandoned; execution remains possible. */
    UNKNOWN,
}

/**
 * Durable identity-funded shield record, scoped by the listing's wallet id plus
 * [accountIndex] and [activityId]. Identity, nonce and amount are null when the
 * signed record cannot be read. Unsigned fields preserve the full native range.
 */
data class ShieldedIdentityDebitRecoveryRecord(
    val accountIndex: UInt,
    val activityId: ByteArray,
    val identityId: ByteArray?,
    val nonce: ULong?,
    val amount: ULong?,
    val status: ShieldedIdentityDebitRecoveryStatus,
) {
    internal companion object {
        fun fromNative(record: ShieldedIdentityDebitRecoveryData): ShieldedIdentityDebitRecoveryRecord {
            val status = when (record.status) {
                0 -> ShieldedIdentityDebitRecoveryStatus.RETRYING
                1 -> ShieldedIdentityDebitRecoveryStatus.PARKED
                2 -> ShieldedIdentityDebitRecoveryStatus.UNKNOWN
                else -> error("Unknown identity debit recovery status: ${record.status}")
            }
            return ShieldedIdentityDebitRecoveryRecord(
                record.accountIndex.toUInt(),
                record.activityId.copyOf(),
                record.identityId?.copyOf(),
                if (record.hasNonce) record.nonce.toULong() else null,
                if (record.hasAmount) record.amount.toULong() else null,
                status,
            )
        }
    }
}
