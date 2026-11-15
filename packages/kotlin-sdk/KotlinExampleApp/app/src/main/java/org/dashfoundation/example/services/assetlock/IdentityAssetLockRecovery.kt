package org.dashfoundation.example.services.assetlock

import org.dashfoundation.dashsdk.wallet.TrackedAssetLock

/** Pure presentation filtering; Rust remains the recovery state machine. */
object IdentityAssetLockRecovery {
    fun registrations(rows: List<TrackedAssetLock>): List<TrackedAssetLock> =
        rows.filter { it.fundingType == TrackedAssetLock.FundingType.IDENTITY_REGISTRATION }

    /**
     * Top-up locks a resume pays: bound ones for the selected identity, and
     * unbound ones Rust has not flagged unassigned. A resume
     * (`IdentityCredits.resumeTopUpToRecordedRecipient`) pays the identity
     * the lock is for — an unbound lock's recorded recipient, which may be
     * someone else's identity — so the screen's identity is never passed.
     */
    fun topUps(
        rows: List<TrackedAssetLock>,
        selectedIdentityIndex: Int,
    ): List<TrackedAssetLock> =
        rows.filter {
            (it.fundingType == TrackedAssetLock.FundingType.IDENTITY_TOP_UP &&
                it.registrationIndex == selectedIdentityIndex) ||
                (it.fundingType == TrackedAssetLock.FundingType.IDENTITY_TOP_UP_NOT_BOUND &&
                    !it.identityRecipientUnassigned)
        }

    /**
     * Unbound top-up locks recording no identity (Rust's
     * `identity_recipient_unassigned`): no resume pays them, so they are
     * listed apart, for the user to assign an identity first.
     */
    fun unassignedTopUps(rows: List<TrackedAssetLock>): List<TrackedAssetLock> =
        rows.filter { it.identityRecipientUnassigned }

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
            ":${lock.outpointVout} · ${lock.status.name.lowercase().replace('_', ' ')}" +
            (lock.identityRecipient?.let { " → " + it.joinToString("") { b -> "%02x".format(b) }.take(8) } ?: "")
}
