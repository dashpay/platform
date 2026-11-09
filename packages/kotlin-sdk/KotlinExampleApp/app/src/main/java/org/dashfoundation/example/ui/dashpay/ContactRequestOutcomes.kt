package org.dashfoundation.example.ui.dashpay

import org.dashfoundation.dashsdk.errors.DashSdkError

/**
 * How a contact request send or accept that threw ended, for the DashPay
 * screens. Follows the send screen (and iOS `SendViewModel.swift`): a shielded
 * spend whose result could not be confirmed may already be on chain (Rust
 * keeps its notes reserved), so the screen keeps it pending and never offers a
 * fresh submission; the next sync settles it. A private request Rust reports
 * as pending (one to the same contact in flight, or paid and not yet read
 * back), which a public send or accept can also meet, sent nothing new: the
 * screen shows it as pending and offers a private sync, which settles it,
 * after which the user may try again. Every other error is an ordinary
 * failure the user may retry.
 * ← Swift `ContactRequestSubmissionOutcome` (`ContactRequestOutcomes.swift`).
 */
sealed interface ContactRequestSubmissionOutcome {
    /** The private reply may have gone through: do not submit again. */
    data object MayHaveGoneThrough : ContactRequestSubmissionOutcome

    /** A private request to the contact is pending in the wallet; nothing was sent. */
    data object PrivateRequestPending : ContactRequestSubmissionOutcome

    /** It failed; the message is the error's. */
    data class Failed(val message: String) : ContactRequestSubmissionOutcome

    companion object {
        const val MAY_HAVE_GONE_THROUGH_MESSAGE = "This may have gone through. It shows up " +
            "after the next sync confirms it; do not submit it again."

        const val PRIVATE_REQUEST_PENDING_MESSAGE = "A private request to this contact is " +
            "pending. Sync private requests to settle it, then try again if it did not " +
            "go through."

        fun of(error: Throwable): ContactRequestSubmissionOutcome = when (error) {
            is DashSdkError.PlatformWallet.ShieldedSpendUnconfirmed -> MayHaveGoneThrough
            is DashSdkError.PlatformWallet.PrivateContactRequestPending -> PrivateRequestPending
            else -> Failed(error.message ?: "unknown error")
        }

        /**
         * The message a request row shows: the pending notice while the
         * wallet has a private request to the contact pending, the
         * unavailable notice while the wallet could not be read, whatever
         * error an earlier attempt left, and that error otherwise.
         */
        fun rowMessage(pending: Boolean, unavailable: Boolean = false, error: String?): String? = when {
            pending -> PRIVATE_REQUEST_PENDING_MESSAGE
            unavailable -> PrivateRepliesPendingSnapshot.UNAVAILABLE_MESSAGE
            else -> error
        }
    }
}

/**
 * Which contacts (lower hex ids) have a private request pending in the
 * wallet, as the screens last read it (`privateRepliesPending`). While one
 * is, Rust refuses another send or reply to that contact, so the screens turn
 * off what would submit one (Accept, Ignore, Send). A failed read is not
 * "nothing pending": the last successful snapshot stays, and those controls
 * stay off for every contact until a read covering it succeeds.
 *
 * A send or accept Rust refused as pending (code 71) is kept apart from the
 * read: code 71 also means the private store could not be read to its end
 * before the first send or accept since load, with nothing reserved, so the
 * read may find nothing pending. That contact stays pending, with the
 * private sync offered, until a private sync completes.
 * ← Swift `PrivateRepliesPendingSnapshot` (`ContactRequestOutcomes.swift`).
 */
data class PrivateRepliesPendingSnapshot(
    /** The pending contacts the last successful read found. */
    val pending: Set<String> = emptySet(),
    /** The contacts the last successful read covered. */
    val covered: Set<String> = emptySet(),
    /** False after a failed read, until a read succeeds. */
    val available: Boolean = true,
    /** Contacts a send or accept was refused for as pending since the last completed private sync. */
    val refused: Set<String> = emptySet(),
) {
    /** Whether the last read failed. */
    val isUnavailable: Boolean get() = !available

    /**
     * This snapshot after a read of [contactIds]: [read] holds the pending
     * ones, or is null when the read failed.
     */
    fun recorded(read: Set<String>?, contactIds: Collection<String>): PrivateRepliesPendingSnapshot =
        if (read == null) {
            copy(available = false)
        } else {
            copy(pending = read, covered = contactIds.toSet(), available = true)
        }

    /** This snapshot after a send or accept to [contactId] was refused as pending (code 71). */
    fun withRefusal(contactId: String): PrivateRepliesPendingSnapshot = copy(refused = refused + contactId)

    /** This snapshot after a completed private sync, which settles every refusal. */
    fun afterPrivateSync(): PrivateRepliesPendingSnapshot = copy(refused = emptySet())

    /**
     * Whether [contactId] is pending: the last successful read found a
     * private request to it, or a send or accept to it was refused as
     * pending since the last completed private sync.
     */
    fun isPending(contactId: String): Boolean = contactId in pending || contactId in refused

    /**
     * Whether a send or reply to [contactId] may be offered: a successful
     * read covered it and found nothing pending.
     */
    fun allowsSubmission(contactId: String): Boolean =
        available && contactId in covered && !isPending(contactId)

    companion object {
        const val UNAVAILABLE_MESSAGE = "Could not read whether a private request to this " +
            "contact is pending. Sync private requests to check again."
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
