package org.dashfoundation.example.services.assetlock

import org.dashfoundation.dashsdk.wallet.TrackedAssetLock

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

    /** Display-only; submission passes the snapshot's original bytes/vout. */
    fun label(lock: TrackedAssetLock): String =
        lock.outpointTxid.reversedArray().joinToString("") { "%02x".format(it) }.take(8) +
            ":${lock.outpointVout} · ${lock.status.name.lowercase().replace('_', ' ')}"
}
