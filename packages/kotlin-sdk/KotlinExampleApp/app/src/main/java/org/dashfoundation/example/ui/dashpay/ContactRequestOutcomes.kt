package org.dashfoundation.example.ui.dashpay

import org.dashfoundation.dashsdk.errors.DashSdkError
import org.dashfoundation.dashsdk.tokens.Dashpay

/**
 * How a contact request send or accept that threw ended, for the DashPay
 * screens. Follows the send screen (and iOS `SendViewModel.swift`): a shielded
 * spend whose result could not be confirmed may already be on chain (Rust
 * keeps its notes reserved), so the screen keeps it pending and never offers a
 * fresh submission; the next sync settles it. A private request Rust reports
 * as pending (one to the same contact in flight, or paid and not yet read
 * back), which a public send or accept can also meet, sent nothing new: the
 * screen shows it as pending and offers a private sync, which settles it,
 * after which the user may try again. Unrestored private payments (code 72)
 * hold the wallet, not one contact: the screen shows "restoring private
 * payments" until the wallet's readiness reads restored; no private sync
 * lifts it. Every other error is an ordinary failure the user may retry.
 * ← Swift `ContactRequestSubmissionOutcome` (`ContactRequestOutcomes.swift`).
 */
sealed interface ContactRequestSubmissionOutcome {
    /** The private reply may have gone through: do not submit again. */
    data object MayHaveGoneThrough : ContactRequestSubmissionOutcome

    /** A private request to the contact is pending in the wallet; nothing was sent. */
    data object PrivateRequestPending : ContactRequestSubmissionOutcome

    /** The wallet's private payments are not restored yet; nothing was sent. */
    data object PrivatePaymentsNotRestored : ContactRequestSubmissionOutcome

    /** It failed; the message is the error's. */
    data class Failed(val message: String) : ContactRequestSubmissionOutcome

    companion object {
        const val MAY_HAVE_GONE_THROUGH_MESSAGE = "This may have gone through. It shows up " +
            "after the next sync confirms it; do not submit it again."

        const val PRIVATE_REQUEST_PENDING_MESSAGE = "A private request to this contact is " +
            "pending. Sync private requests to settle it, then try again if it did not " +
            "go through."

        const val RESTORING_PRIVATE_PAYMENTS_MESSAGE = "Restoring this wallet's private " +
            "payments. Contact requests wait until they are restored."

        /**
         * Code 73 is a configuration error of the app's storage, which
         * declares what a private payment needs; shown as such.
         */
        const val RECOVERY_UNSUPPORTED_MESSAGE = "This storage backend can't protect " +
            "private payments, so private contact requests are off."

        fun of(error: Throwable): ContactRequestSubmissionOutcome = when (error) {
            is DashSdkError.PlatformWallet.ShieldedSpendUnconfirmed -> MayHaveGoneThrough
            is DashSdkError.PlatformWallet.PrivateContactRequestPending -> PrivateRequestPending
            is DashSdkError.PlatformWallet.PrivateContactPaymentsNotRestored -> PrivatePaymentsNotRestored
            is DashSdkError.PlatformWallet.PrivateContactRecoveryUnsupported ->
                Failed(RECOVERY_UNSUPPORTED_MESSAGE)
            else -> Failed(error.message ?: "unknown error")
        }

        /**
         * The message a request row shows: the restoring notice while the
         * wallet's private payments are not restored, the pending notice
         * while the wallet has a private request to the contact pending, the
         * unavailable notice while the wallet could not be read, and
         * whatever error an earlier attempt left otherwise.
         */
        fun rowMessage(
            pending: Boolean,
            restoring: Boolean = false,
            unavailable: Boolean = false,
            error: String?,
        ): String? = when {
            restoring -> RESTORING_PRIVATE_PAYMENTS_MESSAGE
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
 * private sync offered, until the wallet's readiness says the store was
 * read to its end.
 *
 * Whether the wallet's private payments are restored is the wallet's
 * readiness (`Dashpay.privateContactReadiness`), not anything the screens
 * conclude: a code 72 refusal, or a readiness read that says not restored,
 * shows "restoring private payments" for every contact, and only a
 * readiness read that says restored lifts it. A private sync changes
 * neither on its own.
 * ← Swift `PrivateRepliesPendingSnapshot` (`ContactRequestOutcomes.swift`).
 */
data class PrivateRepliesPendingSnapshot(
    /** The pending contacts the last successful read found. */
    val pending: Set<String> = emptySet(),
    /** The contacts the last successful read covered. */
    val covered: Set<String> = emptySet(),
    /** False after a failed read, until a read succeeds. */
    val available: Boolean = true,
    /** Contacts a send or accept was refused for as pending (code 71), until the store is read to its end. */
    val refused: Set<String> = emptySet(),
    /** The wallet's readiness for the identity, as last read; null until a read succeeds. */
    val readiness: Dashpay.PrivateContactReadiness? = null,
    /** Set by a code 72 refusal; cleared only by a readiness read that says restored. */
    val restoringRefused: Boolean = false,
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

    /** This snapshot after a send or accept refused because the private payments are not restored (code 72). */
    fun withRestoringRefusal(): PrivateRepliesPendingSnapshot = copy(restoringRefused = true)

    /**
     * This snapshot after a read of the wallet's readiness, or null when it
     * failed, which keeps what the last read said. A readiness that says
     * restored lifts a code 72 refusal; one that says the private store was
     * read to its end leaves a contact's pending state to the pending read.
     */
    fun withReadiness(read: Dashpay.PrivateContactReadiness?): PrivateRepliesPendingSnapshot =
        if (read == null) {
            this
        } else {
            copy(
                readiness = read,
                restoringRefused = restoringRefused && !read.privatePaymentsRestored,
                refused = if (read.privateStoreRead) emptySet() else refused,
            )
        }

    /**
     * Whether the wallet's private payments are being restored: a send or
     * accept was refused with code 72, or the last readiness read says they
     * are not restored. Every contact waits while they are.
     */
    val isRestoringPrivatePayments: Boolean
        get() = restoringRefused || readiness?.privatePaymentsRestored == false

    /**
     * Whether [contactId] is pending: the last successful read found a
     * private request to it, or a send or accept to it was refused as
     * pending and the store has not been read to its end since.
     */
    fun isPending(contactId: String): Boolean = contactId in pending || contactId in refused

    /**
     * Whether a send or reply to [contactId] may be offered: a successful
     * read covered it and found nothing pending.
     */
    fun allowsSubmission(contactId: String): Boolean =
        available && contactId in covered && !isPending(contactId) && !isRestoringPrivatePayments

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
