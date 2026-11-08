package org.dashfoundation.example.ui.dashpay

import org.dashfoundation.dashsdk.errors.DashSdkError

/**
 * How a contact request accept that threw ended, for the DashPay screens.
 * Follows the send screen (and iOS `SendViewModel.swift`): a shielded spend
 * whose result could not be confirmed may already be on chain (Rust keeps its
 * notes reserved), so the screen keeps it pending and never offers a fresh
 * submission; the next sync settles it. Every other error is an ordinary
 * failure the user may retry.
 * ← Swift `ContactRequestSubmissionOutcome` (`ContactRequestOutcomes.swift`).
 */
sealed interface ContactRequestSubmissionOutcome {
    /** The private reply may have gone through: do not submit again. */
    data object MayHaveGoneThrough : ContactRequestSubmissionOutcome

    /** It failed; the message is the error's. */
    data class Failed(val message: String) : ContactRequestSubmissionOutcome

    companion object {
        const val MAY_HAVE_GONE_THROUGH_MESSAGE = "This may have gone through. It shows up " +
            "after the next sync confirms it; do not submit it again."

        fun of(error: Throwable): ContactRequestSubmissionOutcome =
            if (error is DashSdkError.PlatformWallet.ShieldedSpendUnconfirmed) {
                MayHaveGoneThrough
            } else {
                Failed(error.message ?: "unknown error")
            }
    }
}

/**
 * What the request list's private-store refresh tells the user. A network
 * that does not serve the store yet (before protocol version 15) is expected
 * and stays quiet; an operational failure, or requests read but not saved,
 * is shown so the user knows to refresh again.
 * ← Swift `PrivateContactRefreshNotice` (`ContactRequestOutcomes.swift`).
 */
object PrivateContactRefreshNotice {
    /** The notice a pass that read the store leaves, or null when every identity's requests reached disk. */
    fun notice(unpersistedIdentityCount: Long): String? {
        if (unpersistedIdentityCount <= 0) return null
        val who = if (unpersistedIdentityCount == 1L) "identity were" else "identities were"
        return "Private requests for $unpersistedIdentityCount $who read but not saved. " +
            "Pull to refresh again."
    }

    /** The notice a pass that failed leaves, or null when the network does not serve the private store yet. */
    fun notice(error: Throwable): String? =
        if (error is DashSdkError.PlatformWallet.ProtocolVersionNotActive) {
            null
        } else {
            "Private requests could not be read: ${error.message ?: "unknown error"}"
        }
}
