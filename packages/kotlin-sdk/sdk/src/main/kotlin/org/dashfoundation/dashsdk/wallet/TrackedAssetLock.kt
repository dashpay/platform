package org.dashfoundation.dashsdk.wallet

import org.dashfoundation.dashsdk.ffi.TrackedAssetLockNativeData
import org.dashfoundation.dashsdk.ffi.TrackedAssetLocksNativeResult

/**
 * Rust-authoritative tracked asset-lock snapshot eligible for generic
 * identity recovery. Invitation (3), address/shielded (4/5), consumed (4),
 * and malformed rows are deliberately absent.
 *
 * [Status.RECOVERED_FROM_CHAIN] rows ARE eligible: they are asset locks
 * the restore scan rebuilt from chain-locked history (Core finality
 * proven, Platform-side consumption unknown), and the registration /
 * top-up recovery screens are exactly the user-driven surface allowed
 * to try consuming one — Platform rejects an already-spent outpoint
 * with a typed error. Do NOT feed this status into any automatic
 * stuck-lock retry sweep; blind retries of historical locks are the
 * failure mode the dedicated status exists to prevent.
 *
 * Recipient fields, from `TrackedAssetLockEntryFFI`
 * (`packages/rs-platform-wallet-ffi/src/core_wallet_types.rs`):
 *
 * - [identityRecipient]: the 32-byte identity an unbound identity top-up
 *   ([FundingType.IDENTITY_TOP_UP_NOT_BOUND]) was built or assigned to pay,
 *   null when the row records none. A resume
 *   ([org.dashfoundation.dashsdk.credits.IdentityCredits.resumeTopUpToRecordedRecipient])
 *   pays that identity, in this wallet or not, and refuses any other.
 * - [identityRecipientUnassigned]: an unbound top-up recording no identity
 *   that the wallet knows is not consumed (status [Status.BUILT] through
 *   [Status.CHAIN_LOCKED]). No resume pays it; list it for the user, who
 *   assigns a recipient
 *   ([org.dashfoundation.dashsdk.credits.IdentityCredits.assignTopUpRecipient]),
 *   rather than resuming it into one of the wallet's identities. False for a
 *   recipient-less [Status.RECOVERED_FROM_CHAIN] lock, which every past top-up
 *   rebuilt after a restore from the seed carries: Platform may have used it
 *   long ago and no query tells the wallet, so do not present it as
 *   recoverable; assigning one is refused with
 *   [AssetLockAlreadyConsumed][org.dashfoundation.dashsdk.errors.DashSdkError.PlatformWallet.AssetLockAlreadyConsumed].
 *   Rust decides it; this model only carries it.
 *
 * The shielded recipient a type-5 lock records is not carried: this model
 * admits identity locks only, and the entry it is copied from has no such
 * field.
 */
data class TrackedAssetLock(
    val outpointTxid: ByteArray,
    val outpointVout: Int,
    val fundingType: FundingType,
    val status: Status,
    val registrationIndex: Int,
    val instantLockPresent: Boolean,
    val chainLockHeight: Int,
    val identityRecipient: ByteArray? = null,
    val identityRecipientUnassigned: Boolean = false,
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
        // 4 (CONSUMED) stays deliberately unmapped: consumed rows are
        // terminal tombstones and must never surface as recoverable.

        /** Rebuilt by the restore scan from a chain-locked record —
         * finality proven (chain proof attached Rust-side), consumption
         * unknown. Selectable for user-driven recovery only. */
        RECOVERED_FROM_CHAIN(5),
    }

    init {
        require(outpointTxid.size == 32) {
            "outpointTxid must be exactly 32 bytes, got ${outpointTxid.size}"
        }
        require(outpointVout >= 0) { "outpointVout must be non-negative" }
        require(registrationIndex >= 0) { "registrationIndex must be non-negative" }
        require(chainLockHeight >= 0) { "chainLockHeight must be non-negative" }
        require(identityRecipient == null || identityRecipient.size == 32) {
            "identityRecipient must be exactly 32 bytes, got ${identityRecipient?.size}"
        }
    }

    override fun equals(other: Any?): Boolean =
        other is TrackedAssetLock &&
            outpointTxid.contentEquals(other.outpointTxid) &&
            outpointVout == other.outpointVout &&
            fundingType == other.fundingType &&
            status == other.status &&
            registrationIndex == other.registrationIndex &&
            instantLockPresent == other.instantLockPresent &&
            chainLockHeight == other.chainLockHeight &&
            identityRecipient.contentEquals(other.identityRecipient) &&
            identityRecipientUnassigned == other.identityRecipientUnassigned

    override fun hashCode(): Int = outpointTxid.contentHashCode() * 31 + outpointVout

    internal companion object {
        fun eligibleFromNative(result: TrackedAssetLocksNativeResult): List<TrackedAssetLock> =
            result.entries.mapNotNull(::eligibleFromNative)

        private fun eligibleFromNative(row: TrackedAssetLockNativeData): TrackedAssetLock? {
            if (row.outpointTxid.size != 32 || row.outpointVout < 0 ||
                row.registrationIndex < 0 || row.chainLockHeight < 0 ||
                (row.identityRecipient != null && row.identityRecipient.size != 32)
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
                identityRecipient = row.identityRecipient?.copyOf(),
                identityRecipientUnassigned = row.identityRecipientUnassigned,
            )
        }
    }
}
