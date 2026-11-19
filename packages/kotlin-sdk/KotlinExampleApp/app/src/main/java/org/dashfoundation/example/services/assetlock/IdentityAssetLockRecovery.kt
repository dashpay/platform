package org.dashfoundation.example.services.assetlock

import org.dashfoundation.dashsdk.errors.DashSdkError
import org.dashfoundation.dashsdk.wallet.TrackedAssetLock
import org.dashfoundation.example.util.Base58

/**
 * A registration lock whose resume found its slot holding another identity
 * of this wallet: Rust recorded [identityIdBase58] at [identityIndex] and
 * kept [lock] tracked, and the lock can top that identity up.
 */
class RegistrationLockTopUp(
    val lock: TrackedAssetLock,
    val identityIndex: Int,
    val identityId: ByteArray,
    val identityIdBase58: String,
)

/** Pure presentation filtering; Rust remains the recovery state machine. */
object IdentityAssetLockRecovery {
    fun registrations(rows: List<TrackedAssetLock>): List<TrackedAssetLock> =
        rows.filter { it.fundingType == TrackedAssetLock.FundingType.IDENTITY_REGISTRATION }

    fun topUps(
        rows: List<TrackedAssetLock>,
        selectedIdentityIndex: Int,
    ): List<TrackedAssetLock> =
        rows.filter {
            (it.fundingType == TrackedAssetLock.FundingType.IDENTITY_TOP_UP &&
                it.registrationIndex == selectedIdentityIndex) ||
                it.fundingType == TrackedAssetLock.FundingType.IDENTITY_TOP_UP_NOT_BOUND
        }

    /** Resume-only dispatch: callers cannot accidentally select a fresh-funding operation. */
    suspend fun <T> submitRegistrationResume(
        lock: TrackedAssetLock,
        resume: suspend (TrackedAssetLock) -> T,
    ): T = resume(lock)

    /** Resume-only dispatch: callers cannot accidentally select a fresh-funding operation. */
    suspend fun <T> submitTopUpResume(
        lock: TrackedAssetLock,
        resume: suspend (TrackedAssetLock) -> T,
    ): T = resume(lock)

    /**
     * What a failed registration resume leaves to do. A resume answered
     * [DashSdkError.PlatformWallet.IdentitySlotTakenOnChain] is not retried
     * at another slot (the lock's index is fixed); it offers a top-up of the
     * identity Rust found there. Every other failure offers nothing.
     */
    fun registrationResumeTopUp(lock: TrackedAssetLock, error: Throwable): RegistrationLockTopUp? {
        val taken = error as? DashSdkError.PlatformWallet.IdentitySlotTakenOnChain ?: return null
        val identityId = Base58.decodeIdentifier(taken.identityId) ?: return null
        return RegistrationLockTopUp(lock, taken.identityIndex, identityId, taken.identityId)
    }

    /** Display-only; submission passes the snapshot's original bytes/vout. */
    fun label(lock: TrackedAssetLock): String =
        lock.outpointTxid.reversedArray().joinToString("") { "%02x".format(it) }.take(8) +
            ":${lock.outpointVout} · ${lock.status.name.lowercase().replace('_', ' ')}"
}
