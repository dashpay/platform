package org.dashfoundation.dashsdk.wallet

import org.dashfoundation.dashsdk.ffi.TrackedAssetLockNativeData
import org.dashfoundation.dashsdk.ffi.TrackedAssetLocksNativeResult

/**
 * Rust-authoritative tracked asset-lock snapshot eligible for generic
 * identity recovery. Invitation (3), address/shielded (4/5), consumed (4),
 * and malformed rows are deliberately absent.
 */
data class TrackedAssetLock(
    val outpointTxid: ByteArray,
    val outpointVout: Int,
    val fundingType: FundingType,
    val status: Status,
    val registrationIndex: Int,
    val instantLockPresent: Boolean,
    val chainLockHeight: Int,
) {
    enum class FundingType(val raw: Int) {
        IDENTITY_REGISTRATION(0),
        IDENTITY_TOP_UP(1),
        IDENTITY_TOP_UP_NOT_BOUND(2),
    }

    enum class Status(val raw: Int) {
        BUILT(0),
        BROADCAST(1),
        INSTANT_SEND_LOCKED(2),
        CHAIN_LOCKED(3),
    }

    init {
        require(outpointTxid.size == 32) {
            "outpointTxid must be exactly 32 bytes, got ${outpointTxid.size}"
        }
        require(outpointVout >= 0) { "outpointVout must be non-negative" }
        require(registrationIndex >= 0) { "registrationIndex must be non-negative" }
        require(chainLockHeight >= 0) { "chainLockHeight must be non-negative" }
    }

    override fun equals(other: Any?): Boolean =
        other is TrackedAssetLock &&
            outpointTxid.contentEquals(other.outpointTxid) &&
            outpointVout == other.outpointVout &&
            fundingType == other.fundingType &&
            status == other.status &&
            registrationIndex == other.registrationIndex &&
            instantLockPresent == other.instantLockPresent &&
            chainLockHeight == other.chainLockHeight

    override fun hashCode(): Int = outpointTxid.contentHashCode() * 31 + outpointVout

    internal companion object {
        fun eligibleFromNative(result: TrackedAssetLocksNativeResult): List<TrackedAssetLock> =
            result.entries.mapNotNull(::eligibleFromNative)

        private fun eligibleFromNative(row: TrackedAssetLockNativeData): TrackedAssetLock? {
            if (row.outpointTxid.size != 32 || row.outpointVout < 0 ||
                row.registrationIndex < 0 || row.chainLockHeight < 0
            ) return null
            val fundingType = FundingType.entries.firstOrNull { it.raw == row.fundingType }
                ?: return null
            val statusRaw = row.status.toInt() and 0xFF
            val status = Status.entries.firstOrNull { it.raw == statusRaw } ?: return null
            return TrackedAssetLock(
                outpointTxid = row.outpointTxid.copyOf(),
                outpointVout = row.outpointVout,
                fundingType = fundingType,
                status = status,
                registrationIndex = row.registrationIndex,
                instantLockPresent = row.instantLockPresent,
                chainLockHeight = row.chainLockHeight,
            )
        }
    }
}
